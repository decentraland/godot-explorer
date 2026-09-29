//! Locale forwarded to SDK7 scenes (#2707): the client's resolved app locale, never the OS one.
//!
//! Set from `LocaleSettings.apply_locale()` via `DclGlobal.set_scene_locale`, read by the scene
//! threads through `getExplorerInformation` and the `localeChanged` event. The version counter
//! lets each scene detect a change without holding the lock across ticks.

#[cfg(feature = "use_deno")]
use std::sync::atomic::AtomicBool;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    RwLock,
};

const FALLBACK_LOCALE: &str = "en";

// Empty means "never set", reported as FALLBACK_LOCALE.
static SCENE_LOCALE: RwLock<String> = RwLock::new(String::new());
static SCENE_LOCALE_VERSION: AtomicU64 = AtomicU64::new(0);
// ICU's default locale can only be set once the first JsRuntime has loaded the ICU data.
#[cfg(feature = "use_deno")]
static ICU_READY: AtomicBool = AtomicBool::new(false);

/// Godot locale (`pt_BR`) to a BCP-47 tag (`pt-BR`). Empty falls back to `en`.
pub fn to_bcp47(locale: &str) -> String {
    let tag = locale.trim().replace('_', "-");
    if tag.is_empty() {
        FALLBACK_LOCALE.to_owned()
    } else {
        tag
    }
}

/// Store the locale scenes should see. A no-op when unchanged, so `localeChanged` only fires
/// on a real switch.
pub fn set_scene_locale(locale: &str) {
    let new_locale = to_bcp47(locale);
    let mut current = SCENE_LOCALE.write().unwrap_or_else(|e| e.into_inner());
    if *current == new_locale {
        return;
    }
    *current = new_locale;
    SCENE_LOCALE_VERSION.fetch_add(1, Ordering::SeqCst);

    // Keeps `Intl` / `toLocale*` defaults in line with the UI. Isolates that already cached
    // their default keep the old one until the scene reloads.
    #[cfg(feature = "use_deno")]
    if ICU_READY.load(Ordering::SeqCst) {
        v8::icu::set_default_locale(&current);
    }
}

pub fn get_scene_locale() -> String {
    let current = SCENE_LOCALE.read().unwrap_or_else(|e| e.into_inner());
    if current.is_empty() {
        FALLBACK_LOCALE.to_owned()
    } else {
        current.clone()
    }
}

pub fn get_scene_locale_version() -> u64 {
    SCENE_LOCALE_VERSION.load(Ordering::SeqCst)
}

/// Point ICU's process-wide default locale at the scene locale instead of the environment's
/// (which would leak the OS locale through `Intl`). Call right after a `JsRuntime` is created:
/// only the first call applies it, later changes go through [`set_scene_locale`].
#[cfg(feature = "use_deno")]
pub fn init_icu_default_locale() {
    // The write lock serializes this with set_scene_locale.
    let current = SCENE_LOCALE.write().unwrap_or_else(|e| e.into_inner());
    if !ICU_READY.swap(true, Ordering::SeqCst) {
        let locale = if current.is_empty() {
            FALLBACK_LOCALE
        } else {
            current.as_str()
        };
        v8::icu::set_default_locale(locale);
    }
}

#[cfg(test)]
mod tests {
    use super::to_bcp47;

    #[test]
    fn converts_godot_locale_to_bcp47() {
        assert_eq!(to_bcp47("pt_BR"), "pt-BR");
        assert_eq!(to_bcp47("es"), "es");
        assert_eq!(to_bcp47("en"), "en");
    }

    #[test]
    fn empty_locale_falls_back_to_en() {
        assert_eq!(to_bcp47(""), "en");
        assert_eq!(to_bcp47("  "), "en");
    }
}
