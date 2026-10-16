//! The UI language: the saved choice when it is still supported, otherwise the device's, otherwise
//! English.
//!
//! A locale must not be listed in `SUPPORTED` until its catalogue is complete: an incomplete table
//! is the failure mode behind unity-explorer#270 (ES/IT rolled back after shipping half-translated)
//! and #2062 (a Spanish OS locale hung the desktop loading screen). English is the fallback.

use godot::classes::{Os, TranslationServer};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

pub const SUPPORTED: [&str; 3] = ["en", "es", "pt_BR"];
pub const FALLBACK: &str = "en";
/// A QA aid, not a language: it keeps the UI in English and turns on Godot's pseudolocalization,
/// which brackets and pads every translated string (`«____Sign Out____»`). The padding simulates
/// ES/PT running 20-30% longer, so clipping and bad wrapping show before a translation exists. It
/// does not prove a string is translated: the untranslated fallback is pseudolocalized too.
pub const PSEUDO: &str = "xx-pseudo";
/// Whether a fresh install follows the device language (issue #2825).
const FOLLOW_DEVICE: bool = true;

/// The best supported match for a device locale (`es_AR`) and language (`es`): the exact locale,
/// then the language, then a regional variant of it (`pt` is `pt_BR`), then English. `es_AR`,
/// `es_MX` and `es_419` all resolve to `es`.
pub fn detect_device_locale(device: &str, language: &str) -> &'static str {
    if let Some(found) = SUPPORTED.iter().find(|s| **s == device) {
        return found;
    }
    if let Some(found) = SUPPORTED.iter().find(|s| **s == language) {
        return found;
    }
    let prefix = format!("{language}_");
    SUPPORTED
        .iter()
        .find(|s| s.starts_with(&prefix))
        .copied()
        .unwrap_or(FALLBACK)
}

/// The locale to use for the saved choice `configured`.
pub fn resolve_locale(
    configured: &str,
    pseudo_available: bool,
    device: &str,
    language: &str,
) -> &'static str {
    if configured == PSEUDO && pseudo_available {
        return FALLBACK;
    }
    if let Some(found) = SUPPORTED.iter().find(|s| **s == configured) {
        return found;
    }
    if FOLLOW_DEVICE {
        detect_device_locale(device, language)
    } else {
        FALLBACK
    }
}

/// The pseudolocale is offered wherever the build is not a shippable one. Not gated on a debug
/// build: CI exports release builds, which would hide it from the builds QA installs.
fn pseudo_available() -> bool {
    !DclGlobal::is_production()
}

/// Pushes the resolved locale into the translation server and the scenes. Controls re-translate on
/// `NOTIFICATION_TRANSLATION_CHANGED`, so their text updates live.
pub fn apply_locale() {
    let configured = globals::engine()
        .bind()
        .setting("locale".into())
        .try_to::<GString>()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let os = Os::singleton();
    let pseudo_ok = pseudo_available();
    let locale = resolve_locale(
        &configured,
        pseudo_ok,
        &os.get_locale().to_string(),
        &os.get_locale_language().to_string(),
    );
    let mut server = TranslationServer::singleton();
    server.set_locale(locale);
    // Scenes get the same resolved locale as the UI (#2707), never the raw OS locale.
    DclGlobal::set_scene_locale(locale.into());
    server.set_pseudolocalization_enabled(configured == PSEUDO && pseudo_ok);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_locales_fold_to_a_supported_one() {
        assert_eq!(detect_device_locale("pt_BR", "pt"), "pt_BR");
        assert_eq!(detect_device_locale("es_AR", "es"), "es");
        assert_eq!(detect_device_locale("es_419", "es"), "es");
        // Portuguese without a region still finds the Brazilian catalogue.
        assert_eq!(detect_device_locale("pt_PT", "pt"), "pt_BR");
        assert_eq!(detect_device_locale("ja_JP", "ja"), "en");
    }

    #[test]
    fn a_saved_supported_choice_wins() {
        assert_eq!(resolve_locale("es", false, "en_US", "en"), "es");
        assert_eq!(resolve_locale("pt_BR", false, "es_AR", "es"), "pt_BR");
    }

    #[test]
    fn an_unsupported_choice_follows_the_device() {
        assert_eq!(resolve_locale("it", false, "es_AR", "es"), "es");
        assert_eq!(resolve_locale("", false, "pt_BR", "pt"), "pt_BR");
        assert_eq!(resolve_locale("", false, "ja_JP", "ja"), "en");
    }

    #[test]
    fn the_pseudolocale_renders_english_only_where_offered() {
        assert_eq!(resolve_locale(PSEUDO, true, "es_AR", "es"), "en");
        // Not offered: it is just an unknown choice, so the device language applies.
        assert_eq!(resolve_locale(PSEUDO, false, "es_AR", "es"), "es");
    }
}
