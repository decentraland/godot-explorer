//! The emote wheel: ten slots fanned around a button. A tap on a slot plays its emote and closes
//! the wheel; an empty slot opens the backpack's emotes tab instead.

use godot::classes::{
    Button, Control, IControl, InputEvent, InputEventScreenTouch, Label, TranslationServer,
};
use godot::prelude::*;

use super::emote_item::EmoteItemUi;
use super::host;
use super::hud_button::HudButton;
use super::ui_sounds;
use crate::avatars::emotes;
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;

const SLOT_COUNT: usize = 10;
const SLOT_ANGLE_DEGREES: f32 = 36.0;
const NO_TOUCH: i32 = -1;

/// The urn at slot `index`, or none for an unequipped slot (the avatar always reports ten).
pub fn slot_urn(urns: &[String], index: usize) -> Option<&str> {
    urns.get(index)
        .map(String::as_str)
        .filter(|u| !u.is_empty())
}

#[derive(GodotClass)]
#[class(base=Control, init)]
pub struct EmoteWheel {
    base: Base<Control>,
    #[var(get = get_avatar_node, set = set_avatar_node)]
    avatar_node: Option<Gd<DclAvatar>>,
    items: Vec<Gd<EmoteItemUi>>,
    last_selected_urn: String,
    /// The finger that owns the open/close press, so a second finger works while the joystick is
    /// held. -1 is none.
    #[init(val = NO_TOUCH)]
    emote_touch_index: i32,
    #[init(node = "%EmoteWheelContainer")]
    container: OnReady<Gd<Control>>,
    #[init(node = "%Label_EmoteName")]
    label_name: OnReady<Gd<Label>>,
    #[init(node = "%Control_Wheel")]
    control_wheel: OnReady<Gd<Control>>,
    #[init(node = "Button_Emotes")]
    button_emotes: OnReady<Gd<HudButton>>,
    #[init(node = "%EmoteWheelItem1")]
    template: OnReady<Gd<EmoteItemUi>>,
    #[init(node = "Control_Wheel/TextureRect_Background/VBoxContainer/Button_Edit")]
    button_edit: OnReady<Gd<Button>>,
}

#[godot_api]
impl IControl for EmoteWheel {
    fn ready(&mut self) {
        self.reset_name();
        self.button_emotes
            .set_meta("attenuated_sound", &true.to_variant());
        // The wheel toggles from raw touch so a second finger opens it while the joystick is held
        // (Godot only synthesizes a mouse event from the primary touch). `button_mask = 0` routes
        // all activation here; desktop clicks arrive as emulated touches.
        self.button_emotes
            .set_button_mask(godot::global::MouseButtonMask::default());
        let me = self.to_gd();
        self.button_emotes
            .signals()
            .gui_input()
            .connect_other(&me, |this: &mut Self, event| {
                this.on_button_gui_input(event)
            });
        self.button_emotes
            .signals()
            .visibility_changed()
            .connect_other(&me, |this: &mut Self| this.on_button_visibility_changed());
        self.button_emotes
            .signals()
            .toggled()
            .connect_other(&me, |this: &mut Self, on| {
                if on {
                    this.open();
                } else {
                    this.close();
                }
            });
        self.control_wheel
            .signals()
            .gui_input()
            .connect_other(&me, |this: &mut Self, event| {
                if event.try_cast::<InputEventScreenTouch>().is_ok() {
                    this.close();
                }
            });
        self.button_edit
            .signals()
            .pressed()
            .connect_other(&me, |this: &mut Self| {
                open_backpack_on_emotes();
                this.close();
            });
        self.control_wheel.hide();

        // Clone the template nine more times and fan them 36 degrees apart.
        let mut slots = vec![self.template.clone()];
        for i in 0..(SLOT_COUNT - 1) {
            let mut copy = self.template.duplicate_node();
            // The copy carries the template's scene-unique name, which would make `%` ambiguous.
            copy.set_unique_name_in_owner(false);
            copy.set_rotation_degrees(SLOT_ANGLE_DEGREES * (i + 1) as f32);
            self.container.add_child(&copy);
            slots.push(copy);
        }
        for slot in &mut slots {
            slot.signals()
                .play_emote()
                .connect_other(&me, |this: &mut Self, urn| {
                    this.on_play_emote(urn.to_string())
                });
            let item = slot.clone();
            slot.signals().select_emote().connect_other(
                &me,
                move |this: &mut Self, selected, urn| {
                    this.on_select_emote(selected, urn.to_string(), &item)
                },
            );
            // Fire from raw touch so a second finger works too.
            slot.set_button_mask(godot::global::MouseButtonMask::default());
            let item = slot.clone();
            slot.signals()
                .gui_input()
                .connect_other(&me, move |this: &mut Self, event| {
                    this.on_item_gui_input(event, &item)
                });
        }
        self.items = slots;
        if let Some(avatar) = self.avatar_node.clone() {
            self.watch_avatar(&avatar);
        }
        // The default emotes are the initial state.
        let defaults: Vec<String> = emotes::default_emote_ids()
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.update_wheel(&defaults);
    }

    fn on_notification(&mut self, what: godot::classes::notify::ControlNotification) {
        // Only the default title re-translates; a selected emote's name is data, not copy.
        if what == godot::classes::notify::ControlNotification::TRANSLATION_CHANGED
            && self.base().is_node_ready()
            && self.last_selected_urn.is_empty()
        {
            self.reset_name();
        }
    }
}

#[godot_api]
impl EmoteWheel {
    #[signal]
    fn emote_wheel_opened();

    #[signal]
    fn emote_wheel_closed();

    #[func]
    fn get_avatar_node(&self) -> Option<Gd<DclAvatar>> {
        self.avatar_node.clone()
    }

    #[func]
    fn set_avatar_node(&mut self, avatar: Option<Gd<DclAvatar>>) {
        if avatar == self.avatar_node {
            return;
        }
        if let Some(old) = self.avatar_node.take().filter(|a| a.is_instance_valid()) {
            let callable = self.base().callable("on_avatar_loaded");
            let mut old = old.upcast::<Object>();
            if old.is_connected("avatar_loaded", &callable) {
                old.disconnect("avatar_loaded", &callable);
            }
        }
        if let Some(avatar) = &avatar {
            self.watch_avatar(avatar);
        }
        self.avatar_node = avatar;
    }

    #[func]
    fn on_avatar_loaded(&mut self) {
        let Some(avatar) = self.avatar_node.clone() else {
            return;
        };
        let urns: Vec<String> = avatar
            .bind()
            .avatar_data
            .bind()
            .get_emotes()
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.update_wheel(&urns);
    }

    #[func]
    pub fn close(&mut self) {
        if !self.control_wheel.is_visible() {
            return;
        }
        self.control_wheel.hide();
        self.base_mut().emit_signal("emote_wheel_closed", &[]);
        host::explorer_grab_focus();
        // Clearing the pressed state with its signal lets the HUD button repaint its default look;
        // the re-entrant close() returns early since the wheel is hidden.
        if self.button_emotes.is_pressed() {
            self.button_emotes.set_pressed(false);
        }
    }

    #[func]
    pub fn open(&mut self) {
        if self.control_wheel.is_visible() {
            return;
        }
        self.reset_name();
        self.last_selected_urn.clear();
        self.control_wheel.show();
        self.base_mut().emit_signal("emote_wheel_opened", &[]);
        self.base_mut().grab_focus();
        DclGlobal::singleton().call("release_mouse", &[]);
    }
}

impl EmoteWheel {
    fn watch_avatar(&self, avatar: &Gd<DclAvatar>) {
        let callable = self.base().callable("on_avatar_loaded");
        let mut avatar = avatar.clone().upcast::<Object>();
        if !avatar.is_connected("avatar_loaded", &callable) {
            avatar.connect("avatar_loaded", &callable);
        }
    }

    /// The label is `auto_translate_mode = 2` because a creator-authored emote name is written into
    /// it and must never be looked up: the default title is this component's job.
    fn reset_name(&mut self) {
        let title = TranslationServer::singleton().translate("EMOTES_EMOTES");
        self.label_name.set_text(&title.to_string());
    }

    fn update_wheel(&mut self, urns: &[String]) {
        for (i, item) in self.items.iter_mut().enumerate() {
            // An unequipped slot renders empty instead of keeping the previous icon (#2458).
            match slot_urn(urns, i) {
                Some(urn) => item.bind_mut().async_load_from_urn(urn.into(), i as i32),
                None => item.bind_mut().set_empty(),
            }
        }
    }

    fn on_play_emote(&mut self, urn: String) {
        self.close();
        // An empty slot has nothing to play: send the user to equip one.
        if urn.is_empty() {
            open_backpack_on_emotes();
            DclGlobal::singleton().call("send_haptic_feedback", &[]);
            return;
        }
        if DclGlobal::singleton().bind().is_emote_disabled() {
            return;
        }
        let Some(mut avatar) = self.avatar_node.clone() else {
            godot_error!("No avatar node in EmoteWheel!");
            return;
        };
        // Base emotes load from remote on demand; -1 is the full body.
        let _ = avatar.bind_mut().gd_play_emote(urn.as_str().into(), -1, -1);
        globals::comms()
            .bind_mut()
            .send_emote(urn.as_str().into(), -1);
    }

    fn on_select_emote(&mut self, selected: bool, urn: String, item: &Gd<EmoteItemUi>) {
        if !selected {
            self.reset_name();
            self.last_selected_urn.clear();
            return;
        }
        // An empty slot hints the equip action instead of a name.
        if urn.is_empty() {
            let hint = TranslationServer::singleton().translate("EMOTES_EQUIP_EMOTE");
            self.label_name.set_text(&hint.to_string());
            self.last_selected_urn.clear();
            return;
        }
        if urn == self.last_selected_urn {
            return;
        }
        self.last_selected_urn = urn;
        let name = item.bind().emote_name.clone();
        self.label_name.set_text(&name);
        ui_sounds::play_sound(
            "backpack_item_highlight",
            item.has_meta("attenuated_sound"),
            0.0,
        );
    }

    fn on_item_gui_input(&mut self, event: Gd<InputEvent>, item: &Gd<EmoteItemUi>) {
        let Ok(touch) = event.try_cast::<InputEventScreenTouch>() else {
            return;
        };
        if touch.is_pressed() {
            // Re-emitting the item's own signal reuses its wiring: play and close here, and the
            // equip sound from UiSounds.
            let urn = item.bind().emote_urn.clone();
            item.clone().emit_signal("play_emote", &[urn.to_variant()]);
            item.clone().accept_event();
        }
    }

    /// Full raw-touch handling, so the orb shows its pressed state during the hold and a second
    /// finger works: this owns one finger and drives button_down/up and the toggle by hand.
    fn on_button_gui_input(&mut self, event: Gd<InputEvent>) {
        let Ok(touch) = event.try_cast::<InputEventScreenTouch>() else {
            return;
        };
        if touch.is_pressed() {
            if self.emote_touch_index == NO_TOUCH {
                self.emote_touch_index = touch.get_index();
                self.button_emotes.emit_signal("button_down", &[]);
                let pressed = self.button_emotes.is_pressed();
                self.button_emotes.set_pressed(!pressed);
            }
            self.button_emotes.accept_event();
        } else if touch.get_index() == self.emote_touch_index {
            self.emote_touch_index = NO_TOUCH;
            self.button_emotes.emit_signal("button_up", &[]);
            self.button_emotes.accept_event();
        }
    }

    /// A button hidden mid-press never gets its release: free the finger and settle the orb.
    fn on_button_visibility_changed(&mut self) {
        if self.emote_touch_index != NO_TOUCH && !self.button_emotes.is_visible_in_tree() {
            self.emote_touch_index = NO_TOUCH;
            self.button_emotes.emit_signal("button_up", &[]);
        }
    }
}

fn open_backpack_on_emotes() {
    DclGlobal::singleton().emit_signal("open_backpack", &[true.to_variant()]);
    DclGlobal::singleton().call("send_haptic_feedback", &[]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_missing_slots_have_no_urn() {
        let urns = vec!["wave".to_string(), String::new()];
        assert_eq!(slot_urn(&urns, 0), Some("wave"));
        assert_eq!(slot_urn(&urns, 1), None);
        assert_eq!(slot_urn(&urns, 5), None);
    }
}
