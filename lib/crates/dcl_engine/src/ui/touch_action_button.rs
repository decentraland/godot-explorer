//! A round touch button that presses an `ia_*` input action. Driven from raw touch events so
//! every finger works, not just the one Godot synthesizes a mouse event from.

use godot::classes::{Button, IButton, Input, InputEvent, InputEventScreenTouch, TextureRect};
use godot::prelude::*;

use super::orb_skin::OrbSkin;
use crate::godot_classes::dcl_global::DclGlobal;

const NO_TOUCH: i32 = -1;

#[derive(GodotClass)]
#[class(base=Button, init)]
pub struct TouchActionButton {
    base: Base<Button>,
    #[export]
    #[init(val = "ia_primary".into())]
    pub trigger_action: GString,
    touch_index: i32,
    action_active: bool,
    /// The scene-provided icon overlay (PBTouchScreenControls), hidden by default so it never
    /// disturbs the native glyph.
    #[init(node = "CustomIcon")]
    custom_icon: OnReady<Gd<TextureRect>>,
}

#[godot_api]
impl IButton for TouchActionButton {
    fn ready(&mut self) {
        // The pressed state is flipped by hand (`set_pressed_no_signal` needs toggle mode) and
        // the emulated mouse is ignored, so the button stays momentary.
        self.base_mut().set_toggle_mode(true);
        self.base_mut()
            .set_button_mask(godot::global::MouseButtonMask::default());
        let me = self.to_gd();
        self.base()
            .signals()
            .gui_input()
            .connect_other(&me, |this: &mut Self, event| this.on_gui_input(event));
    }
}

#[godot_api]
impl TouchActionButton {
    #[signal]
    pub fn touch_action_changed(pressed: bool);

    /// Latches the orb's hold look (the glider being open, the overflow menu being shown).
    #[func]
    pub fn set_hold(&mut self, on: bool) {
        for child in self.base().get_children().iter_shared() {
            if let Ok(mut orb) = child.try_cast::<OrbSkin>() {
                orb.bind_mut().set_hold(on);
            }
        }
    }

    #[func]
    pub fn set_custom_icon(&mut self, texture: Option<Gd<godot::classes::Texture2D>>) {
        self.custom_icon.set_texture(texture.as_ref());
        self.custom_icon.show();
    }

    #[func]
    pub fn clear_custom_icon(&mut self) {
        self.custom_icon
            .set_texture(Option::<&Gd<godot::classes::Texture2D>>::None);
        self.custom_icon.hide();
    }
}

impl TouchActionButton {
    fn on_gui_input(&mut self, event: Gd<InputEvent>) {
        if self.base().is_disabled() {
            return;
        }
        let Ok(touch) = event.try_cast::<InputEventScreenTouch>() else {
            return;
        };
        let action = self.trigger_action.to_string();
        if touch.is_pressed() {
            if self.touch_index == NO_TOUCH {
                self.touch_index = touch.get_index();
                self.action_active = true;
                self.base_mut().set_pressed_no_signal(true);
                Input::singleton().action_press(&action);
                DclGlobal::notify_joypad_input();
                self.signals().button_down().emit();
                self.signals().touch_action_changed().emit(true);
            }
            self.base_mut().accept_event();
        } else if touch.get_index() == self.touch_index {
            if self.action_active {
                Input::singleton().action_release(&action);
                self.action_active = false;
                self.signals().button_up().emit();
                self.signals().touch_action_changed().emit(false);
            }
            self.base_mut().set_pressed_no_signal(false);
            self.touch_index = NO_TOUCH;
            DclGlobal::notify_joypad_input();
            self.base_mut().accept_event();
        }
    }
}
