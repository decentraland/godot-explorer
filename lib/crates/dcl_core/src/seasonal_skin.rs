//! The seasonal Discover reskin: the `seasonal-skin-key` flag names a folder of assets
//! (an overlay image, an ambient track and the Discover category tile); empty means off.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{flags, urls};

const FLAG: &str = "seasonal-skin-key";
/// The tile label: plain text, or a JSON object of locale → text (`{"en": "Halloween", "es": …}`).
const NAME_FLAG: &str = "seasonal-skin-name";
const OVERLAY_FILE: &str = "discover-overlay.png";
const AMBIENT_FILE: &str = "discover-ambient.mp3";
const TILE_FILE: &str = "discover-tile.png";
/// A corrupt asset fails the same way every time: the cap keeps a transient failure retryable
/// without refetching a permanently bad file on every Discover open.
pub const MAX_FETCH_ATTEMPTS: u32 = 3;

static ATTEMPTS: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SeasonalSkin {
    pub key: String,
    pub overlay_url: String,
    pub ambient_url: String,
    pub tile_url: String,
    /// Tile label by locale (`en`, `es`, `pt_BR`); a plain-text flag is stored under `en`.
    pub names: HashMap<String, String>,
}

/// The key doubles as a path segment: a `/` or `..` would escape the seasons folder.
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn names_from(text: &str) -> HashMap<String, String> {
    let text = text.trim();
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(text) {
        return map
            .into_iter()
            .filter_map(|(locale, name)| Some((locale, name.as_str()?.trim().to_string())))
            .filter(|(_, name)| !name.is_empty())
            .collect();
    }
    if text.is_empty() {
        HashMap::new()
    } else {
        HashMap::from([("en".to_string(), text.to_string())])
    }
}

fn skin_for(key: &str, name: &str) -> Option<SeasonalSkin> {
    let key = key.trim();
    if !valid_key(key) {
        if !key.is_empty() {
            tracing::warn!("invalid seasonal skin key, the skin stays off: {key}");
        }
        return None;
    }
    let base = format!("{}{key}/", urls::seasonal_assets());
    Some(SeasonalSkin {
        key: key.to_string(),
        overlay_url: format!("{base}{OVERLAY_FILE}"),
        ambient_url: format!("{base}{AMBIENT_FILE}"),
        tile_url: format!("{base}{TILE_FILE}"),
        names: names_from(name),
    })
}

/// The active season; `None` while the flags are not loaded, when the key is empty or invalid.
pub fn active() -> Option<SeasonalSkin> {
    if !flags::is_loaded() {
        return None;
    }
    skin_for(&flags::get_text(FLAG, ""), &flags::get_text(NAME_FLAG, ""))
}

/// Spends one asset fetch attempt; `false` once the cap is reached.
pub fn take_fetch_attempt() -> bool {
    take(&ATTEMPTS)
}

fn take(counter: &AtomicU32) -> bool {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < MAX_FETCH_ATTEMPTS).then_some(n + 1)
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_lowercase_alphanumeric_and_dashes() {
        for ok in ["halloween", "christmas-2026", "a1"] {
            assert!(valid_key(ok), "{ok}");
        }
        for bad in ["", "Halloween", "a/b", "..", "a b", "x.y", "a_b", "../x"] {
            assert!(!valid_key(bad), "{bad}");
        }
    }

    #[test]
    fn urls_follow_the_hosting_convention() {
        let skin = skin_for(" halloween ", "").unwrap();
        assert_eq!(skin.key, "halloween");
        assert_eq!(
            skin.overlay_url,
            "https://mobile-ota.dclregenesislabs.xyz/seasons/halloween/discover-overlay.png"
        );
        assert_eq!(
            skin.ambient_url,
            "https://mobile-ota.dclregenesislabs.xyz/seasons/halloween/discover-ambient.mp3"
        );
        assert_eq!(
            skin.tile_url,
            "https://mobile-ota.dclregenesislabs.xyz/seasons/halloween/discover-tile.png"
        );
    }

    #[test]
    fn names_are_plain_text_or_per_locale() {
        assert!(names_from("").is_empty());
        assert_eq!(names_from(" Halloween ")["en"], "Halloween");
        let names = names_from(r#"{"en": "Christmas", "es": "Navidad", "pt_BR": " ", "x": 1}"#);
        assert_eq!(names.len(), 2);
        assert_eq!(names["es"], "Navidad");
    }

    #[test]
    fn empty_or_invalid_key_is_off() {
        assert_eq!(skin_for("", "Halloween"), None);
        assert_eq!(skin_for("   ", ""), None);
        assert_eq!(skin_for("../etc", ""), None);
    }

    #[test]
    fn fetch_attempts_are_capped() {
        let counter = AtomicU32::new(0);
        let taken = (0..MAX_FETCH_ATTEMPTS + 2)
            .filter(|_| take(&counter))
            .count();
        assert_eq!(taken as u32, MAX_FETCH_ATTEMPTS);
    }
}
