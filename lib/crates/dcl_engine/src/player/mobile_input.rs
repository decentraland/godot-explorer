//! The touch catcher behind the HUD: single-finger look, a second finger inside the joystick's
//! area looking too, the two-finger pinch that swaps first/third person, and gestures adopted
//! from scene UI.
//!
//! Pinch belongs to the CAMERA area only: the joystick's active rect is movement (walk plus its
//! own second-finger look), so a touch there is never a pinch candidate. That alone kills the
//! walk-left + look-right false positive. The pinch and the joystick run independently (you can
//! walk and zoom at once). With no competing gesture in the camera area a pinch is simply the
//! finger spread changing past a threshold (the math is in `rig`).

use std::collections::HashMap;

use godot::classes::{
    control::MouseFilter, Control, IControl, Input, InputEvent, InputEventScreenDrag,
    InputEventScreenTouch,
};
use godot::prelude::*;

use super::{body::Player, rig};
use crate::ui::joystick::VirtualJoystick;
use crate::{fetcher::globals, godot_classes::dcl_global::DclGlobal};

pub const HORIZONTAL_SENS: f32 = 0.5;
pub const VERTICAL_SENS: f32 = 0.5;
/// Pixels a free finger must travel before single-finger look engages.
const LOOK_DEADZONE: f32 = 8.0;

const NOTIFICATION_APPLICATION_FOCUS_OUT: i32 = 1005;

/// A gesture adopted from scene UI (a finger pressed a UI element then swiped off it), kept apart
/// from the gui_input state so it never collides with normal touches handled by this catcher.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AdoptedMode {
    Camera,
    Joystick,
}

#[derive(GodotClass)]
#[class(init, base=Control)]
pub struct MobileCameraInput {
    base: Base<Control>,
    player: Option<Gd<Player>>,
    joystick: Option<Gd<VirtualJoystick>>,

    /// Single-finger camera look, driven from gui_input (free touches only), so a finger over the
    /// joystick, a button or scene UI never looks.
    look_index: i32,
    /// True once the look finger travelled past LOOK_DEADZONE (the camera is rotating). Gates
    /// joystick-zone pinch candidacy: an engaged look plus a joystick touch is walk+look.
    look_engaged: bool,
    /// Roblox-style: a SECOND finger inside the joystick's active area drives the camera (the
    /// thumbstick keeps only its first finger). The joystick ignores extra touches and its STOP
    /// filter eats them from gui_input, so this look is driven from the global input.
    js_look_index: i32,

    /// Every active touch (index -> position), tracked in input.
    touches: HashMap<i32, Vector2>,
    /// Touches that reached gui_input, i.e. consumed by no UI, mapped to their PRESS position.
    /// Only these are pinch candidates, so a pinch over interactive UI never steals its drags.
    free_touches: HashMap<i32, Vector2>,
    pinch_active: bool,
    pinch_a: i32,
    pinch_b: i32,
    /// Candidate start positions, seeded when the count reaches two.
    pinch_start_a: Vector2,
    pinch_start_b: Vector2,
    pinch_prev_distance: f32,

    /// Adopted scene-UI swipe gestures, keyed per finger so concurrent breakouts don't clobber
    /// each other's state.
    adopted: HashMap<i32, AdoptedMode>,
}

#[godot_api]
impl IControl for MobileCameraInput {
    fn ready(&mut self) {
        if !DclGlobal::singleton().bind().is_mobile {
            self.base_mut().hide();
            self.base_mut().set_process_input(false);
            return;
        }
        self.look_index = -1;
        self.js_look_index = -1;
        self.pinch_a = -1;
        self.pinch_b = -1;
        self.base_mut().set_mouse_filter(MouseFilter::STOP);
        let mut this = self.to_gd();
        this.call_deferred("resolve_player", &[]);
    }

    /// Backgrounding the app or an OS gesture-cancel can drop the touch-release events, leaving
    /// stale entries that would seed a bogus finger-spread on the next pinch: all touch state is
    /// cleared on focus loss so the next gesture starts clean.
    fn on_notification(&mut self, what: godot::classes::notify::ControlNotification) {
        if matches!(
            what,
            godot::classes::notify::ControlNotification::Unknown(
                NOTIFICATION_APPLICATION_FOCUS_OUT
            )
        ) {
            self.touches.clear();
            self.free_touches.clear();
            self.look_index = -1;
            self.look_engaged = false;
            self.js_look_index = -1;
            if self.pinch_active {
                self.end_pinch();
            }
        }
    }

    /// Global touch tracking and pinch recognition. Runs before gui_input, so it can take over a
    /// finger the joystick/UI would otherwise own. Only pinch-owned touches are consumed here;
    /// everything else falls through untouched.
    fn input(&mut self, event: Gd<InputEvent>) {
        let touch = event.clone().try_cast::<InputEventScreenTouch>().ok();

        // A safety net: an adopted scene-UI finger is drag-captured by its control, which
        // normally forwards the release; end the gesture here too in case that control was
        // freed or hidden mid-gesture (otherwise the joystick could stay engaged with no finger).
        if let Some(touch) = &touch {
            if !touch.is_pressed() && self.adopted.contains_key(&touch.get_index()) {
                self.end_adopted(touch.get_index());
            }
        }

        // Releases are processed FIRST, unconditionally: a touch-up arriving during cinematic or
        // pointer mode (or before the player resolves) must never be dropped, or a phantom
        // finger lingers and a lone real finger confirms a spurious pinch.
        if let Some(touch) = &touch {
            if !touch.is_pressed() {
                let index = touch.get_index();
                let was_pinch_finger =
                    self.pinch_active && (index == self.pinch_a || index == self.pinch_b);
                self.touches.remove(&index);
                self.free_touches.remove(&index);
                if index == self.look_index {
                    self.look_index = -1;
                    self.look_engaged = false;
                }
                if index == self.js_look_index {
                    self.js_look_index = -1;
                }
                self.on_touch_count_changed();
                // Swallow a pinch finger's release so gui_input doesn't read it as a look end.
                if was_pinch_finger {
                    self.handled();
                }
                return;
            }
        }

        if self.player.is_none() {
            return;
        }
        // No zoom in cinematic / pointer mode: gui_input drives the pointer there.
        if globals::scene_runner()
            .bind()
            .get_raycast_use_cursor_position()
        {
            return;
        }

        if let Some(touch) = touch {
            // pressed
            self.touches.insert(touch.get_index(), touch.get_position());
            // Deferred, not immediate: for a press landing in the joystick's active area, the
            // joystick's OWN gui_input (which updates is_pressed/touch_index for this same
            // press) runs after input returns, later in this frame. Checking the candidates
            // synchronously would race it (lifting and re-pressing the joystick thumb could be
            // misread as a free pinch candidate).
            self.base_mut().call_deferred("on_touch_count_changed", &[]);
        } else if let Ok(drag) = event.try_cast::<InputEventScreenDrag>() {
            let index = drag.get_index();
            self.touches.insert(index, drag.get_position());
            if self.pinch_active {
                if index == self.pinch_a || index == self.pinch_b {
                    self.update_pinch();
                    self.handled();
                }
                return;
            }
            // Re-seed on drag too: the free-touch set is filled by gui_input, which runs AFTER
            // input, so a just-pressed free finger may not have been eligible on its press event.
            if self.pinch_a == -1 || self.pinch_b == -1 {
                self.on_touch_count_changed();
            }
            if self.pinch_a != -1 && self.pinch_b != -1 {
                self.try_recognize_pinch();
                if self.pinch_active {
                    self.handled();
                    return;
                }
            }
            self.drive_joystick_area_look(&drag);
        }
    }

    fn gui_input(&mut self, event: Gd<InputEvent>) {
        // A tap reaching this catcher means no UI consumed it: reclaim ui_root focus so movement
        // re-enables if some control stole it without handing it back.
        if let Ok(touch) = event.clone().try_cast::<InputEventScreenTouch>() {
            let focused = DclGlobal::explorer_has_focus();
            if touch.is_pressed() && !focused {
                DclGlobal::explorer_grab_focus();
            }
        }
        if globals::scene_runner()
            .bind()
            .get_raycast_use_cursor_position()
        {
            self.handle_cinematic(&event);
            return;
        }
        if self.player.is_none() {
            return;
        }
        // A committed pinch consumes its fingers in input, so they never arrive here;
        // single-finger look tracks whichever free finger it first sees. Reaching gui_input at
        // all marks the touch FREE (no UI consumed it), hence pinch-eligible.
        if let Ok(touch) = event.clone().try_cast::<InputEventScreenTouch>() {
            let index = touch.get_index();
            if touch.is_pressed() {
                self.free_touches.insert(index, touch.get_position());
                if self.look_index == -1 {
                    self.look_index = index;
                }
                // A second free finger seeds the pinch pair immediately (kills single-finger
                // look at touchdown instead of waiting for the first drag).
                self.on_touch_count_changed();
            } else {
                self.free_touches.remove(&index);
                if index == self.look_index {
                    self.look_index = -1;
                    self.look_engaged = false;
                }
            }
            self.base_mut().accept_event();
        } else if let Ok(drag) = event.try_cast::<InputEventScreenDrag>() {
            if !self.pinch_active && drag.get_index() == self.look_index {
                // Look deadzone: the finger must travel LOOK_DEADZONE px from its press point
                // before the camera moves. An intended pinch lands the second finger first
                // (which kills the look), so the camera no longer rotates through the pinch's
                // opening gap.
                let press = self
                    .free_touches
                    .get(&drag.get_index())
                    .copied()
                    .unwrap_or_else(|| drag.get_position());
                if drag.get_position().distance_to(press) > LOOK_DEADZONE {
                    self.look_engaged = true;
                    if let Some(player) = &mut self.player {
                        player.bind_mut().apply_look_delta(drag.get_relative());
                    }
                }
            }
            self.base_mut().accept_event();
        }
    }
}

#[godot_api]
impl MobileCameraInput {
    #[func]
    fn resolve_player(&mut self) {
        let Some(explorer) = globals::explorer() else {
            return;
        };
        self.player = explorer.get("player").try_to::<Gd<Player>>().ok();
        self.joystick = explorer
            .get("virtual_joystick")
            .try_to::<Gd<VirtualJoystick>>()
            .ok();
    }

    /// (Re)establishes the candidate pair when the touch count changes; ends the pinch when it
    /// drops below two.
    #[func]
    fn on_touch_count_changed(&mut self) {
        // Only FREE touches are pinch candidates (not the joystick, scene UI, HUD, chat).
        let candidates = self.free_pinch_candidates();
        if candidates.len() < 2 {
            if self.pinch_active {
                self.end_pinch();
            } else {
                self.pinch_a = -1;
                self.pinch_b = -1;
            }
            return;
        }
        if !self.pinch_active {
            self.seed_pinch_candidate(&candidates);
        } else if !self.touches.contains_key(&self.pinch_a)
            || !self.touches.contains_key(&self.pinch_b)
        {
            // A committed pinch finger lifted but two camera-area ones remain: re-seat onto the
            // survivors so the gesture keeps going instead of stranding.
            self.pinch_a = candidates[0];
            self.pinch_b = candidates[1];
            self.pinch_prev_distance =
                self.touches[&self.pinch_a].distance_to(self.touches[&self.pinch_b]);
        }
    }

    /// Adopts a gesture that broke out of a scene-UI element. `index` is the touch index;
    /// `start_position` the original press point (decides camera vs joystick and seeds the
    /// joystick base); `current_position` and `relative` describe the drag at the breakout.
    #[func]
    pub fn adopt_touch(
        &mut self,
        index: i32,
        start_position: Vector2,
        current_position: Vector2,
        relative: Vector2,
    ) {
        if globals::scene_runner()
            .bind()
            .get_raycast_use_cursor_position()
            || self.player.is_none()
        {
            return;
        }
        if let Some(joystick) = &mut self.joystick {
            let area = joystick.bind().get_active_area_global_rect();
            if area.contains_point(start_position) {
                self.adopted.insert(index, AdoptedMode::Joystick);
                joystick.bind_mut().external_begin(start_position);
                joystick.bind_mut().external_update(current_position);
                return;
            }
        }
        self.adopted.insert(index, AdoptedMode::Camera);
        if let Some(player) = &mut self.player {
            player.bind_mut().apply_look_delta(relative);
        }
    }

    #[func]
    pub fn update_adopted_touch(&mut self, index: i32, position: Vector2, relative: Vector2) {
        match self.adopted.get(&index) {
            Some(AdoptedMode::Camera) => {
                if let Some(player) = &mut self.player {
                    player.bind_mut().apply_look_delta(relative);
                }
            }
            Some(AdoptedMode::Joystick) => {
                if let Some(joystick) = &mut self.joystick {
                    joystick.bind_mut().external_update(position);
                }
            }
            None => {}
        }
    }

    #[func]
    pub fn release_adopted_touch(&mut self, index: i32) {
        self.end_adopted(index);
    }
}

impl MobileCameraInput {
    fn handled(&self) {
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }

    /// Free touch indices (those seen by gui_input, so owned by no UI) PLUS fingers inside the
    /// joystick's active rect while the joystick hasn't engaged movement: a horizontal pinch often
    /// anchors a thumb there. Excluded while the joystick is walking or the look is engaged, so
    /// walk + look is untouched.
    fn free_pinch_candidates(&self) -> Vec<i32> {
        let mut out: Vec<i32> = self
            .free_touches
            .keys()
            .filter(|index| self.touches.contains_key(index))
            .copied()
            .collect();
        out.sort_unstable();
        if let Some(joystick) = &self.joystick {
            if !joystick.bind().is_pressed && !self.look_engaged {
                let area = joystick.bind().get_active_area_global_rect();
                let mut inside: Vec<i32> = self
                    .touches
                    .iter()
                    .filter(|(index, position)| {
                        !self.free_touches.contains_key(index) && area.contains_point(**position)
                    })
                    .map(|(index, _)| *index)
                    .collect();
                inside.sort_unstable();
                out.extend(inside);
            }
        }
        out
    }

    fn seed_pinch_candidate(&mut self, candidates: &[i32]) {
        self.pinch_a = candidates[0];
        self.pinch_b = candidates[1];
        self.pinch_start_a = self.touches[&self.pinch_a];
        self.pinch_start_b = self.touches[&self.pinch_b];
        self.pinch_prev_distance = self.pinch_start_a.distance_to(self.pinch_start_b);
        // A second free finger means a likely pinch: stop the single-finger look NOW, not at
        // commit, or the camera keeps rotating through the gesture's opening frames.
        self.look_index = -1;
        self.look_engaged = false;
    }

    fn try_recognize_pinch(&mut self) {
        let (Some(a), Some(b)) = (
            self.touches.get(&self.pinch_a).copied(),
            self.touches.get(&self.pinch_b).copied(),
        ) else {
            return;
        };
        // The finger spread must change past the threshold from where the pair formed.
        let start_distance = self.pinch_start_a.distance_to(self.pinch_start_b);
        if rig::should_commit(start_distance, a.distance_to(b)) {
            self.commit_pinch(a, b);
        }
    }

    /// Drives the camera from a second finger inside the joystick's active area. Latches onto the
    /// first such finger (lazily, so a two-finger simultaneous touchdown still catches it once
    /// the joystick has claimed its primary) and looks until it lifts or a pinch takes over.
    fn drive_joystick_area_look(&mut self, event: &Gd<InputEventScreenDrag>) {
        let (Some(joystick), Some(_)) = (&self.joystick, &self.player) else {
            return;
        };
        let index = event.get_index();
        if self.js_look_index == -1 {
            let touch_index = joystick.bind().touch_index;
            let area = joystick.bind().get_active_area_global_rect();
            if touch_index != -1
                && index != touch_index
                && area.contains_point(event.get_position())
            {
                self.js_look_index = index;
            }
        }
        if index == self.js_look_index {
            if let Some(player) = &mut self.player {
                player.bind_mut().apply_look_delta(event.get_relative());
            }
        }
    }

    fn commit_pinch(&mut self, a: Vector2, b: Vector2) {
        self.pinch_active = true;
        self.pinch_prev_distance = a.distance_to(b);
        // Both fingers are in the camera area (never the joystick's), so the walk keeps running:
        // only the free single-finger look yields to the zoom. A joystick-zone finger (idle
        // joystick) means the "walk" was really the pinch's anchor thumb: cancel the joystick so
        // its base doesn't drive the avatar mid-pinch.
        self.look_index = -1;
        self.look_engaged = false;
        self.js_look_index = -1;
        if !self.free_touches.contains_key(&self.pinch_a)
            || !self.free_touches.contains_key(&self.pinch_b)
        {
            if let Some(joystick) = &mut self.joystick {
                joystick.bind_mut().cancel();
            }
        }
        if let Some(player) = &mut self.player {
            player.bind_mut().begin_pinch_zoom();
        }
    }

    fn update_pinch(&mut self) {
        let (Some(a), Some(b)) = (
            self.touches.get(&self.pinch_a).copied(),
            self.touches.get(&self.pinch_b).copied(),
        ) else {
            return;
        };
        let distance = a.distance_to(b);
        let delta = distance - self.pinch_prev_distance;
        self.pinch_prev_distance = distance;
        if delta.abs() > 1e-5 {
            if let Some(player) = &mut self.player {
                player.bind_mut().apply_pinch_zoom(delta);
            }
        }
    }

    fn end_pinch(&mut self) {
        self.pinch_active = false;
        self.pinch_a = -1;
        self.pinch_b = -1;
        // A surviving finger doesn't resume look until re-pressed (its press was consumed while
        // pinching).
        self.look_index = -1;
        self.look_engaged = false;
        if let Some(player) = &mut self.player {
            player.bind_mut().end_pinch_zoom();
        }
    }

    fn handle_cinematic(&mut self, event: &Gd<InputEvent>) {
        let Ok(touch) = event.clone().try_cast::<InputEventScreenTouch>() else {
            return;
        };
        if let Some(mut explorer) = globals::explorer() {
            explorer.call("set_cursor_position", &[touch.get_position().to_variant()]);
        }
        let mut input = Input::singleton();
        if touch.is_pressed() {
            input.action_press("ia_pointer");
        } else {
            input.action_release("ia_pointer");
        }
    }

    fn end_adopted(&mut self, index: i32) {
        if self.adopted.get(&index) == Some(&AdoptedMode::Joystick) {
            if let Some(joystick) = &mut self.joystick {
                joystick.bind_mut().external_end();
            }
        }
        self.adopted.remove(&index);
    }
}
