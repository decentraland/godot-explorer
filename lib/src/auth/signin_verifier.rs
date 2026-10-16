//! Verifier binding a mobile sign-in to this install. The `/auth/mobile` page is opened with an
//! `authRequestId` derived from a secret verifier; the auth server only hands the identity to a
//! `GET /identities/:id` that presents the verifier, so the `signin=` deep link alone is not
//! enough to take over the session.
//!
//! The verifier is persisted because the OS can kill the app while the user is in the browser,
//! and the parked identity id is redeemed on the next launch.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use godot::classes::file_access::ModeFlags;
use godot::classes::{DirAccess, FileAccess};
use godot::prelude::GString;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const VERIFIER_HEADER: &str = "x-identity-verifier";

const PENDING_PATH: &str = "user://mobile_signin_verifier.json";
// The auth server keeps an identity for one hour.
const PENDING_MAX_AGE_SECS: u64 = 3600;

#[derive(Serialize, Deserialize)]
struct PendingVerifier {
    verifier: String,
    created_at_unix: u64,
}

pub fn create_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// First 16 bytes of SHA-256(verifier) with the UUID v4 version and variant bits set. Must match
/// auth-server's derivation byte for byte.
pub fn derive_auth_request_id(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).hyphenated().to_string()
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Replaces any pending verifier with this sign-in's.
pub fn save_pending(verifier: &str) {
    let payload = PendingVerifier {
        verifier: verifier.to_string(),
        created_at_unix: now_unix(),
    };
    let Ok(json) = serde_json::to_string(&payload) else {
        return;
    };
    match FileAccess::open(&GString::from(PENDING_PATH), ModeFlags::WRITE) {
        Some(mut file) => {
            file.store_string(&GString::from(&json));
            file.close();
        }
        None => tracing::warn!("signin_verifier: failed to persist the pending verifier"),
    }
}

/// The pending verifier, or `None` when there is none or it outlived the identity it guards.
pub fn load_pending() -> Option<String> {
    let path = GString::from(PENDING_PATH);
    if !FileAccess::file_exists(&path) {
        return None;
    }
    let mut file = FileAccess::open(&path, ModeFlags::READ)?;
    let content = file.get_as_text().to_string();
    file.close();
    let pending: PendingVerifier = serde_json::from_str(&content).ok()?;
    if now_unix().saturating_sub(pending.created_at_unix) > PENDING_MAX_AGE_SECS {
        return None;
    }
    Some(pending.verifier)
}

pub fn clear_pending() {
    let path = GString::from(PENDING_PATH);
    if FileAccess::file_exists(&path) {
        DirAccess::remove_absolute(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shared with auth-server, the auth site, unity-explorer and creator-hub.
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const AUTH_REQUEST_ID: &str = "13d31e96-1a1a-48ec-af16-b10c4c982e08";

    #[test]
    fn derives_the_shared_test_vector() {
        assert_eq!(derive_auth_request_id(VERIFIER), AUTH_REQUEST_ID);
    }

    #[test]
    fn creates_unpadded_base64url_verifiers_of_32_bytes() {
        let verifier = create_verifier();
        assert_eq!(verifier.len(), 43);
        assert!(verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn derives_a_uuid_v4_from_a_fresh_verifier() {
        let id = Uuid::parse_str(&derive_auth_request_id(&create_verifier())).unwrap();
        assert_eq!(id.get_version_num(), 4);
        assert_eq!(id.get_variant(), uuid::Variant::RFC4122);
    }
}
