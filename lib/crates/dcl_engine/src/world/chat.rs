//! Lines the app says in the chat, as `system`.

use godot::classes::Time;
use godot::prelude::*;

use crate::godot_classes::dcl_global::DclGlobal;

pub const SYSTEM: &str = "system";

pub fn say(text: &str) {
    let now = Time::singleton().get_unix_time_from_system();
    DclGlobal::singleton()
        .signals()
        .on_chat_message()
        .emit(SYSTEM, text, now);
}

/// A translated line; `{name}` placeholders take the `replacements`.
pub fn say_translated(key: &str, replacements: &VarDictionary) {
    let text = DclGlobal::singleton().tr(key);
    let formatted = text
        .to_variant()
        .call("format", &[replacements.to_variant()]);
    say(&formatted.to_string());
}

/// A debug notice, dimmed.
pub fn say_dim(text: &str) {
    say(&format!("[color=#ccc]{text}[/color]"));
}

/// A translated line with no placeholders.
pub fn say_translated_plain(key: &str) {
    say(&DclGlobal::singleton().tr(key).to_string());
}
