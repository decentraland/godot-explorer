//! The touch joystick: a thumbstick that rests at the safe area's bottom-left and, in dynamic
//! mode, appears where the thumb lands. It drives the `ia_*` movement actions.

use godot::classes::control::MouseFilter;
use godot::classes::{
    ColorRect, Control, IControl, Input, InputEvent, InputEventScreenDrag, InputEventScreenTouch,
    ShaderMaterial, Timer,
};
use godot::prelude::*;

use super::host;
use super::safe_area::SafeAreaMargins;
use crate::fetcher::globals;

/// A gesture handed off from scene UI (the finger started on a UI element and swiped into the
/// joystick zone) takes this index so it never collides with a real gui touch.
const EXTERNAL_TOUCH_INDEX: i32 = -2;
const NO_TOUCH: i32 = -1;
/// The fixed-mode grab radius around the resting base.
const BASE_GRAB_RADIUS: f32 = 25.0;
const SHOW_DELAY: f64 = 0.25;
/// Past this stick length the avatar sprints, after a short hold.
const SPRINT_THRESHOLD: f32 = 0.95;

const STATE_IDLE: i32 = 0;
const STATE_SHOWN: i32 = 1;
const STATE_RELEASING: i32 = 2;

/// The stick output for a finger `vector` away from the base: zero inside the deadzone, then
/// scaled so the clamp radius reads as 1.
pub fn stick_output(vector: Vector2, deadzone: f32, clampzone: f32) -> (bool, Vector2) {
    if vector.length_squared() > deadzone * deadzone {
        let scaled = (vector - vector.normalized() * deadzone) / (clampzone - deadzone);
        (true, scaled)
    } else {
        (false, Vector2::ZERO)
    }
}

/// The resting base position: the default offset from the safe area's left and bottom edges.
pub fn base_position(default: Vector2, safe_left: f32, safe_bottom: f32, height: f32) -> Vector2 {
    Vector2::new(default.x + safe_left, height - default.y - safe_bottom)
}

#[derive(GodotClass)]
#[class(base=Control)]
pub struct VirtualJoystick {
    base: Base<Control>,
    /// If the input is inside this range, the output is zero.
    #[export]
    deadzone_size: f32,
    /// The max distance the tip can reach.
    #[export]
    clampzone_size: f32,
    /// Resting position, from the safe area's left and bottom edges.
    #[export]
    joystick_default_position: Vector2,
    /// 0 fixed, 1 dynamic: the base appears where the thumb lands.
    #[export(enum = (Fixed = 0, Dynamic = 1))]
    joystick_mode: i32,
    /// 0 always visible, 1 only on touch screens.
    #[export(enum = (Always = 0, TouchscreenOnly = 1))]
    visibility_mode: i32,
    #[export]
    action_left: GString,
    #[export]
    action_right: GString,
    #[export]
    action_up: GString,
    #[export]
    action_down: GString,
    #[export]
    action_walk: GString,
    #[export]
    action_sprint: GString,
    /// The shared HUD inset profile, so the resting spot follows the safe area.
    #[export]
    margin_profile: Option<Gd<SafeAreaMargins>>,
    #[var]
    pub is_pressed: bool,
    #[var]
    pub touch_index: i32,
    output: Vector2,
    joystick_position: Vector2,
    joystick_visible: bool,
    active_area: OnReady<Gd<Control>>,
    dynamic: OnReady<Gd<ColorRect>>,
    sprint_timer: OnReady<Gd<Timer>>,
}

#[godot_api]
impl IControl for VirtualJoystick {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            deadzone_size: 0.0,
            clampzone_size: 75.0,
            joystick_default_position: Vector2::ZERO,
            joystick_mode: 0,
            visibility_mode: 0,
            action_left: "ia_left".into(),
            action_right: "ia_right".into(),
            action_up: "ia_forward".into(),
            action_down: "ia_backward".into(),
            action_walk: "ia_walk".into(),
            action_sprint: "ia_sprint".into(),
            margin_profile: None,
            is_pressed: false,
            touch_index: NO_TOUCH,
            output: Vector2::ZERO,
            joystick_position: Vector2::ZERO,
            joystick_visible: false,
            active_area: OnReady::from_node("ActiveArea"),
            dynamic: OnReady::from_node("Dynamic"),
            sprint_timer: OnReady::from_node("SprintTimer"),
        }
    }

    fn ready(&mut self) {
        let me = self.to_gd();
        self.sprint_timer
            .signals()
            .timeout()
            .connect_other(&me, |this| this.start_sprint());
        let hide = self.visibility_mode == 1
            && !godot::classes::DisplayServer::singleton().is_touchscreen_available();
        if hide {
            self.base_mut().hide();
        }
        self.active_area.set_mouse_filter(MouseFilter::STOP);
        self.active_area
            .signals()
            .gui_input()
            .connect_other(&me, |this, event| this.on_gui_input(event));
        self.base()
            .signals()
            .resized()
            .connect_other(&me, |this| this.reset());
        host::connect("on_menu_open", &me, "on_menu_open");
        host::connect("on_menu_close", &me, "on_menu_close");
        host::connect("loading_started", &me, "on_loading_scene");
        self.on_loading_scene();
    }
}

#[godot_api]
impl VirtualJoystick {
    #[signal]
    fn stick_position(position: Vector2);

    #[signal]
    fn is_holded(held: bool);

    #[func]
    fn on_menu_open(&mut self) {
        self.active_area.set_mouse_filter(MouseFilter::IGNORE);
    }

    #[func]
    fn on_menu_close(&mut self) {
        self.active_area.set_mouse_filter(MouseFilter::STOP);
    }

    #[func]
    fn on_loading_scene(&mut self) {
        self.set_state(STATE_IDLE);
    }

    /// Hides the graphic and the touch area, for scenes that hide the native joystick.
    #[func]
    pub fn set_visuals_hidden(&mut self, hidden: bool) {
        self.dynamic.set_visible(!hidden);
        self.active_area.set_visible(!hidden);
    }

    /// Starts a gesture from a screen point: the base goes where the scene UI press began so
    /// movement engages at once.
    #[func]
    pub fn external_begin(&mut self, start_position: Vector2) {
        if self.touch_index != NO_TOUCH {
            return;
        }
        self.touch_index = EXTERNAL_TOUCH_INDEX;
        self.move_base(start_position);
        self.set_state(STATE_SHOWN);
        self.joystick_visible = true;
        self.update_joystick(start_position);
    }

    #[func]
    pub fn external_update(&mut self, position: Vector2) {
        if self.touch_index == EXTERNAL_TOUCH_INDEX {
            self.update_joystick(position);
        }
    }

    #[func]
    pub fn external_end(&mut self) {
        if self.touch_index != EXTERNAL_TOUCH_INDEX {
            return;
        }
        self.release_visual();
        self.reset();
        self.emit_stick(Vector2::ZERO);
    }

    /// Drops any active gesture (walk and the claimed touch): a pinch over the joystick zone
    /// must not keep driving the avatar.
    #[func]
    pub fn cancel(&mut self) {
        self.reset();
    }

    #[func]
    pub fn get_active_area_global_rect(&self) -> Rect2 {
        self.active_area.get_global_rect()
    }

    #[func]
    fn on_show_joystick_timer(&mut self) {
        if self.touch_index != NO_TOUCH {
            self.set_state(STATE_SHOWN);
            self.joystick_visible = true;
        }
    }
}

impl VirtualJoystick {
    fn set_state(&mut self, state: i32) {
        if let Some(mut material) = self
            .dynamic
            .get_material()
            .and_then(|m| m.try_cast::<ShaderMaterial>().ok())
        {
            material.set_shader_parameter("state", &state.to_variant());
        }
    }

    fn set_shader_vec(&mut self, name: &str, value: Vector2) {
        if let Some(mut material) = self
            .dynamic
            .get_material()
            .and_then(|m| m.try_cast::<ShaderMaterial>().ok())
        {
            material.set_shader_parameter(name, &value.to_variant());
        }
    }

    fn release_visual(&mut self) {
        if self.joystick_visible {
            self.set_state(STATE_RELEASING);
            self.joystick_visible = false;
        }
    }

    fn emit_stick(&mut self, value: Vector2) {
        self.base_mut()
            .emit_signal("stick_position", &[value.to_variant()]);
    }

    fn start_sprint(&mut self) {
        Input::singleton().action_press(&self.action_sprint.to_string());
    }

    /// `gui_input` positions are local to the active area, the shader draws in the joystick's own
    /// space: shift by the area's offset so the drawn stick tracks the finger.
    fn to_render_space(&self, local: Vector2) -> Vector2 {
        local + self.active_area.get_position()
    }

    /// Cinematic camera mode lets the event bubble so scene click handlers still get the pointer.
    fn consume_unless_cinematic(&mut self) {
        if !globals::scene_runner()
            .bind()
            .get_raycast_use_cursor_position()
        {
            self.active_area.accept_event();
        }
    }

    fn on_gui_input(&mut self, event: Gd<InputEvent>) {
        if !host::is_mobile() {
            return;
        }
        if let Ok(touch) = event.clone().try_cast::<InputEventScreenTouch>() {
            self.on_touch(&touch);
        } else if let Ok(drag) = event.try_cast::<InputEventScreenDrag>() {
            if drag.get_index() == self.touch_index {
                let position = self.to_render_space(drag.get_position());
                self.update_joystick(position);
                self.consume_unless_cinematic();
            }
        }
    }

    fn on_touch(&mut self, touch: &Gd<InputEventScreenTouch>) {
        if touch.is_pressed() {
            // A panel we came from can leave GUI focus on itself, and the joystick consumes its
            // own touches so nothing else would take it back: without this the stick animates
            // but the avatar does not move (#2361).
            if !host::explorer_has_focus() {
                host::explorer_grab_focus();
            }
            if self.touch_index != NO_TOUCH {
                return;
            }
            let position = self.to_render_space(touch.get_position());
            if self.joystick_mode == 0 && !self.is_inside_base(position) {
                return;
            }
            if self.joystick_mode == 1 {
                self.move_base(position);
                if let Some(mut tree) = self.base().get_tree_or_null() {
                    let me = self.to_gd();
                    tree.create_timer(SHOW_DELAY)
                        .signals()
                        .timeout()
                        .connect_other(&me, |this: &mut Self| this.on_show_joystick_timer());
                }
            }
            self.touch_index = touch.get_index();
            self.update_joystick(position);
            self.consume_unless_cinematic();
        } else if touch.get_index() == self.touch_index {
            self.release_visual();
            self.reset();
            self.emit_stick(Vector2::ZERO);
            self.consume_unless_cinematic();
        }
    }

    fn is_inside_base(&self, point: Vector2) -> bool {
        (point - self.joystick_position).length_squared() <= BASE_GRAB_RADIUS * BASE_GRAB_RADIUS
    }

    fn move_base(&mut self, position: Vector2) {
        self.joystick_position = position;
        self.set_shader_vec("joystick_position", position);
    }

    fn update_joystick(&mut self, touch_position: Vector2) {
        let vector = touch_position - self.joystick_position;
        self.set_shader_vec("tip_position", vector);
        let (pressed, output) = stick_output(vector, self.deadzone_size, self.clampzone_size);
        self.is_pressed = pressed;
        self.output = output;
        self.update_input_actions();
    }

    fn update_input_actions(&mut self) {
        let mut input = Input::singleton();
        let o = self.output;
        let axes = [
            (&self.action_left, -o.x),
            (&self.action_right, o.x),
            (&self.action_up, -o.y),
            (&self.action_down, o.y),
        ];
        for (action, strength) in axes {
            let name = action.to_string();
            if strength > 0.0 {
                input.action_press_ex(&name).strength(strength).done();
            } else if input.is_action_pressed(&name) {
                input.action_release(&name);
            }
        }
        let sprint = self.action_sprint.to_string();
        if o.length() < SPRINT_THRESHOLD {
            input.action_release(&sprint);
            self.sprint_timer.stop();
        } else if self.sprint_timer.is_stopped() && !input.is_action_pressed(&sprint) {
            self.sprint_timer.start();
        }
    }

    /// Back to rest: no gesture, no input, the base at its resting spot, shifted by the safe area.
    fn reset(&mut self) {
        self.is_pressed = false;
        self.base_mut()
            .emit_signal("is_holded", &[false.to_variant()]);
        self.output = Vector2::ZERO;
        self.touch_index = NO_TOUCH;

        let (mut safe_left, mut safe_bottom) = (0.0_f32, 0.0_f32);
        if let Some(profile) = &self.margin_profile {
            let p = profile.bind();
            if host::is_portrait() {
                safe_left = p.portrait_left as f32;
                safe_bottom = p.portrait_bottom as f32;
            } else {
                safe_left = p.left as f32;
                safe_bottom = p.bottom as f32;
            }
        }
        if host::has_safe_area() {
            let area = host::safe_area();
            let window = godot::classes::DisplayServer::singleton().window_get_size();
            if let Some(viewport) = self.base().get_viewport() {
                let visible = viewport.get_visible_rect().size;
                safe_left =
                    safe_left.max(area.position.x as f32 * visible.x / window.x.max(1) as f32);
                safe_bottom = safe_bottom.max(
                    (area.end().y - window.y).abs() as f32 * visible.y / window.y.max(1) as f32,
                );
            }
        }
        let size = self.base().get_size();
        let base = base_position(
            self.joystick_default_position,
            safe_left,
            safe_bottom,
            size.y,
        );
        if let Some(mut material) = self
            .dynamic
            .get_material()
            .and_then(|m| m.try_cast::<ShaderMaterial>().ok())
        {
            material.set_shader_parameter("max_chain_length", &(size.length() * 0.25).to_variant());
        }
        self.move_base(base);
        self.set_shader_vec("tip_position", Vector2::ZERO);

        let mut input = Input::singleton();
        for action in [
            &self.action_left,
            &self.action_right,
            &self.action_down,
            &self.action_up,
            &self.action_walk,
            &self.action_sprint,
        ] {
            let name = action.to_string();
            if input.is_action_pressed(&name) || input.is_action_just_pressed(&name) {
                input.action_release(&name);
            }
        }
        self.sprint_timer.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_the_deadzone_is_zero() {
        assert_eq!(
            stick_output(Vector2::new(3.0, 4.0), 10.0, 75.0),
            (false, Vector2::ZERO)
        );
    }

    #[test]
    fn output_reaches_one_at_the_clamp_radius() {
        let (pressed, out) = stick_output(Vector2::new(75.0, 0.0), 0.0, 75.0);
        assert!(pressed);
        assert!((out.x - 1.0).abs() < 1e-6 && out.y == 0.0);
        // With a deadzone the deadzone is subtracted before scaling.
        let (_, out) = stick_output(Vector2::new(0.0, -75.0), 10.0, 75.0);
        assert!((out.y + 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_resting_base_sits_above_the_bottom_inset() {
        let base = base_position(Vector2::new(154.0, 146.0), 108.0, 45.0, 720.0);
        assert_eq!(base, Vector2::new(262.0, 529.0));
    }
}
