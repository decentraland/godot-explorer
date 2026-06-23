//! Guest anchor identifier — the string used to derive the thirdweb guest
//! wallet. `compute_session_id` hashes it into the opaque thirdweb `sessionId`
//! so the raw identifier never leaves the device.
//!
//! `resolve_anchor` decides the source:
//! - a non-empty value passed from GDScript wins. That value is either the
//!   platform-native device anchor (Android SSAID / iOS Keychain UUID, which
//!   persists across reinstall) or the fixed `DEBUG_GUEST_ANCHOR_OVERRIDE`.
//! - otherwise it falls back to a per-install UUID in `user://device_anchor.txt`
//!   (the desktop source, and what mobile uses when GDScript's
//!   `DEBUG_GUEST_ROTATE_ANCHOR_ID` flag makes `get_device_anchor_id` return "").
//!   Deleting `user://` then resets the guest identity to a brand-new wallet.

use ethers_core::utils::{hex, keccak256};
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use godot::prelude::GString;
use uuid::Uuid;

const ANCHOR_PATH: &str = "user://device_anchor.txt";
const SESSION_ID_PREFIX: &str = "dcl-godot";

/// Reads the per-install UUID anchor from `user://device_anchor.txt`, creating
/// it on first use. This is the fallback used whenever `resolve_anchor` gets an
/// empty value (always on desktop; on mobile only when the rotate flag is on).
pub fn get_or_create_user_anchor() -> String {
    let path = GString::from(ANCHOR_PATH);

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
            ANCHOR_PATH
        );
    }

    let uuid = Uuid::new_v4().to_string();
    match FileAccess::open(&path, ModeFlags::WRITE) {
        Some(mut file) => {
            file.store_string(&GString::from(&uuid));
            file.close();
            tracing::info!("device_anchor: wrote new user anchor to {}", ANCHOR_PATH);
        }
        None => {
            tracing::error!(
                "device_anchor: failed to open {} for writing — anchor will not persist",
                ANCHOR_PATH
            );
        }
    }
    uuid
}

/// Resolves the guest anchor: returns the caller-provided value if non-empty
/// (the native device anchor or the debug override), otherwise falls back to
/// the persisted per-install `user://` UUID. Centralises the "explicit value,
/// else user:// file, else freshly minted" decision.
pub fn resolve_anchor(native_anchor: &str) -> String {
    let trimmed = native_anchor.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    get_or_create_user_anchor()
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
