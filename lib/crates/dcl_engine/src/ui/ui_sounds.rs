//! The UI sound effects: one player per sound on the `UI` bus, and the helpers that wire a control's
//! hover, press and toggle signals to them.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::{
    AudioStream, AudioStreamPlayer, BaseButton, Button, CheckBox, INode, LineEdit, OptionButton,
    Slider, TextEdit, TextureButton,
};
use godot::prelude::*;

use super::emote_item::EmoteItemUi;

/// Each is `res://assets/sfx/ui/{name}.wav`.
const SOUNDS: [&str; 24] = [
    "toggle_enable",
    "toggle_disable",
    "generic_button_press",
    "generic_button_release",
    "generic_button_hover",
    "backpack_item_equip",
    "backpack_item_highlight",
    "widget_chat_open",
    "widget_chat_close",
    "widget_chat_message_private_send",
    "notification_chatmessage_public_appear",
    "notification_chatmessage_private_appear",
    "voice_chat_mic_on",
    "voice_chat_mic_off",
    "mainmenu_widget_open",
    "mainmenu_widget_close",
    "mainmenu_tab_switch",
    "inputfield_entertext",
    "mainmenu_tile_highlight",
    "widget_emotes_close",
    "widget_emotes_highlight",
    "widget_emotes_open",
    "ui_fade_in",
    "ui_fade_out",
];

/// Attenuated sounds (the HUD in a 3D context, an emote wheel) play this much quieter.
const ATTENUATION_DB: f32 = 20.0;
const NO_SOUNDS_META: &str = "disable_ui_sounds";
const ATTENUATED_META: &str = "attenuated_sound";

thread_local! {
    static PLAYERS: RefCell<HashMap<&'static str, Gd<AudioStreamPlayer>>> =
        RefCell::new(HashMap::new());
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct UiSounds {
    base: Base<Node>,
}

#[godot_api]
impl INode for UiSounds {
    fn ready(&mut self) {
        for name in SOUNDS {
            let mut player = AudioStreamPlayer::new_alloc();
            let path = format!("res://assets/sfx/ui/{name}.wav");
            if let Ok(stream) = godot::tools::try_load::<AudioStream>(&path) {
                player.set_stream(&stream);
            }
            player.set_bus("UI");
            self.base_mut().add_child(&player);
            PLAYERS.with(|p| p.borrow_mut().insert(name, player));
        }
    }

    fn exit_tree(&mut self) {
        PLAYERS.with(|p| p.borrow_mut().clear());
    }
}

#[godot_api]
impl UiSounds {
    #[func(rename = play_sound)]
    fn gd_play_sound(
        sound_name: GString,
        #[opt(default = false)] attenuated: bool,
        #[opt(default = 0.0)] volume_db: f32,
    ) {
        play_sound(&sound_name.to_string(), attenuated, volume_db);
    }

    #[func(rename = install_audio_recursive)]
    fn gd_install_audio_recursive(node: Gd<Node>) {
        install_audio_recursive(&node);
    }
}

pub fn play_sound(name: &str, attenuated: bool, volume_db: f32) {
    let player = PLAYERS.with(|p| p.borrow().get(name).cloned());
    let Some(mut player) = player.filter(|p| p.is_instance_valid()) else {
        godot_error!("UiSounds: no sound named {name}");
        return;
    };
    player.set_volume_db(if attenuated {
        volume_db - ATTENUATION_DB
    } else {
        volume_db
    });
    player.play();
}

/// Wires the sounds of a control by its kind; a control that got them is marked so a second call
/// does not double them.
fn install_audio(node: &Gd<Node>) {
    if node.has_meta(NO_SOUNDS_META) {
        return;
    }
    let attenuated = node.has_meta(ATTENUATED_META);
    let hover = move || play_sound("generic_button_hover", attenuated, 0.0);
    let press = move || play_sound("generic_button_press", attenuated, 0.0);
    let release = move || play_sound("generic_button_release", attenuated, 0.0);
    let toggled = move |on: bool| {
        play_sound(
            if on {
                "toggle_enable"
            } else {
                "toggle_disable"
            },
            attenuated,
            0.0,
        )
    };
    let typed = move || play_sound("inputfield_entertext", attenuated, 0.0);

    let added = if let Ok(item) = node.clone().try_cast::<EmoteItemUi>() {
        item.signals().mouse_entered().connect(hover);
        item.signals().button_down().connect(press);
        item.signals().button_up().connect(release);
        item.signals().play_emote().connect(move |_urn| {
            play_sound("backpack_item_equip", attenuated, 0.0);
        });
        true
    } else if let Ok(check) = node.clone().try_cast::<CheckBox>() {
        check.signals().mouse_entered().connect(hover);
        check.signals().toggled().connect(toggled);
        true
    } else if let Ok(option) = node.clone().try_cast::<OptionButton>() {
        option.signals().mouse_entered().connect(hover);
        option.signals().toggled().connect(toggled);
        true
    } else if let Ok(button) = node.clone().try_cast::<Button>() {
        wire_button(
            &mut button.clone().upcast::<BaseButton>(),
            hover,
            press,
            release,
            toggled,
        );
        true
    } else if let Ok(button) = node.clone().try_cast::<TextureButton>() {
        wire_button(
            &mut button.upcast::<BaseButton>(),
            hover,
            press,
            release,
            toggled,
        );
        true
    } else if let Ok(slider) = node.clone().try_cast::<Slider>() {
        slider.signals().value_changed().connect(move |_v| typed());
        true
    } else if let Ok(line) = node.clone().try_cast::<LineEdit>() {
        line.signals().text_changed().connect(move |_t| typed());
        true
    } else if let Ok(text) = node.clone().try_cast::<TextEdit>() {
        text.signals().text_changed().connect(typed);
        true
    } else {
        false
    };
    if added {
        node.clone().set_meta(NO_SOUNDS_META, &true.to_variant());
    }
}

fn wire_button(
    button: &mut Gd<BaseButton>,
    hover: impl Fn() + 'static,
    press: impl Fn() + 'static,
    release: impl Fn() + 'static,
    toggled: impl Fn(bool) + 'static,
) {
    button.signals().mouse_entered().connect(hover);
    button.signals().button_down().connect(press);
    button.signals().button_up().connect(release);
    if button.is_toggle_mode() {
        button.signals().toggled().connect(toggled);
    }
}

pub fn install_audio_recursive(node: &Gd<Node>) {
    install_audio(node);
    for child in node.get_children().iter_shared() {
        install_audio_recursive(&child);
    }
}
