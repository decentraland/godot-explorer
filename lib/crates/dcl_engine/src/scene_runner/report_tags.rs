//! What a Sentry report says about the scene the player is in, and the event a crashed scene sends.

use dcl_core::Core;

const NO_SCENE: &str = "none";
const TAG_VALUE_MAX: usize = 200;

/// A tag value on one line and short; `none` when there is nothing to say.
fn tag_value(value: &str) -> String {
    let one_line = value.replace(['\n', '\r'], " ");
    let trimmed = one_line.trim();
    if trimmed.is_empty() {
        NO_SCENE.to_string()
    } else {
        trimmed.chars().take(TAG_VALUE_MAX).collect()
    }
}

/// The scene the player stands in, or none.
pub(super) fn tag_current_scene(scene: Option<(&str, &str, (i32, i32))>) {
    let Some(core) = Core::try_get() else {
        return;
    };
    let (urn, title, parcel) = match scene {
        Some((urn, title, (x, y))) => (tag_value(urn), tag_value(title), format!("{x},{y}")),
        None => (
            NO_SCENE.to_string(),
            NO_SCENE.to_string(),
            NO_SCENE.to_string(),
        ),
    };
    let hub = core.hub();
    hub.set_report_tag("scene_urn", &urn);
    hub.set_report_tag("scene_title", &title);
    hub.set_report_tag("scene_parcel", &parcel);
}

pub(super) fn report_crash(entity_id: &str, title: &str, base_parcel: (i32, i32), reason: &str) {
    if let Some(core) = Core::try_get() {
        core.hub()
            .report_scene_crash(entity_id, title, base_parcel, reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_value_is_one_short_line_or_none() {
        assert_eq!(tag_value("  My\nScene\r "), "My Scene");
        assert_eq!(tag_value(" \n "), "none");
        assert_eq!(tag_value(&"x".repeat(500)).len(), TAG_VALUE_MAX);
    }
}
