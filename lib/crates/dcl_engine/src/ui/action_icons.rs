//! What an `ia_*` action looks like on screen: the one table the joypad buttons and the
//! interaction pill both read, so a hint can never show a different glyph than the button.
//!
//! Not covered, by design: the joypad's per-frame jump state (jump, double jump, glide) and a
//! creator's `PBTouchScreenControls` override. Both are runtime state, not a property of the action.

use godot::classes::{RefCounted, Texture2D};
use godot::prelude::*;

pub const IC_JUMP_NORMAL: &str = "uid://d4neuk8df8m4y";
pub const IC_JUMP_PRESSED: &str = "uid://dykud4ptnkdei";

pub struct ActionLook {
    pub normal: &'static str,
    pub pressed: &'static str,
    /// The letter a keyboard-style action shows instead of a glyph; empty for icon actions.
    pub keycap: &'static str,
    /// A translation key (not a `TranslationKey`, which is built at runtime).
    pub label_key: &'static str,
}

// The label keys are passed as arguments only, which `extract_strings.py` cannot see.
// i18n-keys: TOOLTIP_ACTION_TAP, TOOLTIP_ACTION_JUMP, TOOLTIP_ACTION_PRIMARY
// i18n-keys: TOOLTIP_ACTION_SECONDARY, TOOLTIP_ACTION_1, TOOLTIP_ACTION_2
// i18n-keys: TOOLTIP_ACTION_3, TOOLTIP_ACTION_4
pub fn look(action: &str) -> Option<ActionLook> {
    let (normal, pressed, keycap, label_key) = match action {
        "ia_pointer" => (
            "uid://c55dfgqwdxs8f",
            "uid://ct0wqa804vtni",
            "",
            "TOOLTIP_ACTION_TAP",
        ),
        "ia_jump" => (IC_JUMP_NORMAL, IC_JUMP_PRESSED, "", "TOOLTIP_ACTION_JUMP"),
        "ia_primary" => (
            "uid://ck3e0eaelc3rq",
            "uid://01qlcj0sqqnw",
            "E",
            "TOOLTIP_ACTION_PRIMARY",
        ),
        "ia_secondary" => (
            "uid://72h2xkpj1hgk",
            "uid://c5u8stl6jg8cl",
            "F",
            "TOOLTIP_ACTION_SECONDARY",
        ),
        "ia_action_3" => (
            "uid://e0ug4dbj1y10",
            "uid://ddrk8qdneg8lw",
            "1",
            "TOOLTIP_ACTION_1",
        ),
        "ia_action_4" => (
            "uid://cqxtpai3pix5u",
            "uid://bbqcb676u1mrv",
            "2",
            "TOOLTIP_ACTION_2",
        ),
        "ia_action_5" => (
            "uid://sw0euo71n3gv",
            "uid://dm5mjc6eto6v1",
            "3",
            "TOOLTIP_ACTION_3",
        ),
        "ia_action_6" => (
            "uid://dvpirmcnk4c2a",
            "uid://dx8f2nledowsj",
            "4",
            "TOOLTIP_ACTION_4",
        ),
        _ => return None,
    };
    Some(ActionLook {
        normal,
        pressed,
        keycap,
        label_key,
    })
}

pub fn texture(uid: &str) -> Option<Gd<Texture2D>> {
    godot::tools::try_load::<Texture2D>(uid).ok()
}

/// The normal and pressed glyphs a joypad button is skinned with. Jump is excluded: the joypad
/// resolves it per frame from the player's jump state.
pub fn button_icons(action: &str) -> Option<(Gd<Texture2D>, Gd<Texture2D>)> {
    if action == "ia_jump" {
        return None;
    }
    let look = look(action)?;
    Some((texture(look.normal)?, texture(look.pressed)?))
}

/// The same table for GDScript callers, until the tooltip that reads it is Rust too.
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct ActionIcons {
    base: Base<RefCounted>,
}

#[godot_api]
impl ActionIcons {
    #[func]
    pub fn has_action(action: GString) -> bool {
        look(&action.to_string()).is_some()
    }

    /// The glyph the interaction pill shows, or null when the action uses a letter keycap.
    #[func]
    pub fn tooltip_icon(action: GString) -> Option<Gd<Texture2D>> {
        let look = look(&action.to_string())?;
        if look.keycap.is_empty() {
            texture(look.normal)
        } else {
            None
        }
    }

    #[func]
    pub fn tooltip_keycap(action: GString) -> GString {
        look(&action.to_string())
            .map(|l| l.keycap.into())
            .unwrap_or_default()
    }

    /// The pill's fallback label as a raw translation key, empty for an unknown action.
    #[func]
    pub fn tooltip_label_key(action: GString) -> GString {
        look(&action.to_string())
            .map(|l| l.label_key.into())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_priority_action_has_a_look() {
        for action in [
            "ia_jump",
            "ia_pointer",
            "ia_primary",
            "ia_secondary",
            "ia_action_3",
            "ia_action_4",
            "ia_action_5",
            "ia_action_6",
        ] {
            assert!(look(action).is_some(), "{action}");
        }
        assert!(look("ia_unknown").is_none());
    }

    #[test]
    fn keyboard_actions_carry_a_keycap() {
        assert_eq!(look("ia_primary").unwrap().keycap, "E");
        assert_eq!(look("ia_pointer").unwrap().keycap, "");
    }
}
