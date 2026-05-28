//! Device anchor identifier — a stable string keyed to the physical install
//! that survives uninstall/reinstall on the same device.
//!
//! Platform sources:
//! - Android: `Settings.Secure.ANDROID_ID` (provided from GDScript via the native plugin)
//! - iOS: UUID stored in Keychain (provided from GDScript via the native plugin)
//! - Desktop: UUID stored in `user://device_anchor.txt` (this file is responsible)
//!
//! `compute_session_id` hashes whatever anchor the caller provides into the
//! opaque thirdweb `sessionId` so the raw device identifier never leaves the device.

use ethers_core::utils::{hex, keccak256};
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use godot::prelude::GString;
use uuid::Uuid;

const DESKTOP_ANCHOR_PATH: &str = "user://device_anchor.txt";
const SESSION_ID_PREFIX: &str = "dcl-godot";

/// Reads the desktop UUID anchor from `user://device_anchor.txt`, creating it
/// on first use. Used when the platform-native source (SSAID / Keychain) is
/// unavailable (desktop) or returned empty.
pub fn get_or_create_desktop_anchor() -> String {
    let path = GString::from(DESKTOP_ANCHOR_PATH);

    if FileAccess::file_exists(&path) {
        if let Some(mut file) = FileAccess::open(&path, ModeFlags::READ) {
            let content = file.get_as_text().to_string();
            file.close();
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        tracing::warn!(
            "device_anchor: existing file at {} is empty or unreadable, regenerating",
            DESKTOP_ANCHOR_PATH
        );
    }

    let uuid = Uuid::new_v4().to_string();
    match FileAccess::open(&path, ModeFlags::WRITE) {
        Some(mut file) => {
            file.store_string(&GString::from(&uuid));
            file.close();
            tracing::info!(
                "device_anchor: wrote new desktop anchor to {}",
                DESKTOP_ANCHOR_PATH
            );
        }
        None => {
            tracing::error!(
                "device_anchor: failed to open {} for writing — anchor will not persist",
                DESKTOP_ANCHOR_PATH
            );
        }
    }
    uuid
}

/// Resolves the device anchor: returns the caller-provided value if non-empty,
/// otherwise falls back to the persisted desktop UUID. Centralises the
/// "platform native value, then desktop file, then freshly minted" decision.
pub fn resolve_anchor(native_anchor: &str) -> String {
    let trimmed = native_anchor.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    get_or_create_desktop_anchor()
}

/// Hashes the anchor into the opaque thirdweb `sessionId`. Same anchor →
/// same session id → same wallet address (this is the load-bearing property).
pub fn compute_session_id(anchor: &str) -> String {
    let digest = keccak256(anchor.as_bytes());
    format!("{}-{}", SESSION_ID_PREFIX, hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_is_deterministic() {
        let a = compute_session_id("abc");
        let b = compute_session_id("abc");
        assert_eq!(a, b);
    }

    #[test]
    fn session_id_differs_per_anchor() {
        assert_ne!(compute_session_id("abc"), compute_session_id("def"));
    }

    #[test]
    fn session_id_has_expected_shape() {
        let id = compute_session_id("anchor-1234");
        assert!(id.starts_with("dcl-godot-"));
        // 32 bytes of keccak256 → 64 hex chars
        assert_eq!(id.len(), "dcl-godot-".len() + 64);
    }
}
