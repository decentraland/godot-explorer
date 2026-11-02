//! Verifier binding a mobile sign-in to this install. The `/auth/mobile` page is opened with an
//! `authRequestId` derived from a secret verifier; the auth server only hands the identity to a
//! `GET /identities/:id` that presents the verifier, so the `signin=` deep link alone is not
//! enough to take over the session.
//!
//! Verifiers are persisted because the OS can kill the app while the user is in the browser,
//! and the parked identity id is redeemed on the next launch. A few attempts are kept, keyed by
//! their `authRequestId`, so a page from an attempt started before a kill still matches.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const VERIFIER_HEADER: &str = "x-identity-verifier";

const FILE_NAME: &str = "mobile_signin_verifier.json";
const MAX_PENDING: usize = 3;
// Housekeeping only: never consulted when picking the verifier for a matching id, since the
// server enforces the identity lifetime itself.
const HOUSEKEEPING_MAX_AGE_SECS: u64 = 24 * 3600;

/// One writer at a time: every update is a read-modify-write of the whole file.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PendingVerifier {
    pub auth_request_id: String,
    pub verifier: String,
    created_at_unix: u64,
}

#[derive(Debug, PartialEq)]
pub enum Selection {
    /// The verifier to send with the identity fetch.
    Use(PendingVerifier),
    /// The deep link has no `authRequestId` and none is pending: fetch without the header.
    NoVerifier,
    /// The deep link names an `authRequestId` this install never started.
    Foreign,
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

/// A non-success status of the identity fetch that settles the attempt: the identity is gone or
/// the verifier is wrong. Anything else (transport, 5xx) keeps the verifier for another try.
pub fn is_terminal_status(status: Option<u16>) -> bool {
    matches!(status, Some(401 | 404 | 410))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn push(entries: &mut Vec<PendingVerifier>, entry: PendingVerifier, now: u64) {
    entries.retain(|e| now.saturating_sub(e.created_at_unix) <= HOUSEKEEPING_MAX_AGE_SECS);
    entries.push(entry);
    let excess = entries.len().saturating_sub(MAX_PENDING);
    entries.drain(..excess);
}

/// An empty `auth_request_id` comes from an auth site that predates it: use the latest attempt.
fn select(entries: &[PendingVerifier], auth_request_id: &str) -> Selection {
    if auth_request_id.is_empty() {
        return entries
            .last()
            .cloned()
            .map_or(Selection::NoVerifier, Selection::Use);
    }
    entries
        .iter()
        .find(|e| e.auth_request_id.eq_ignore_ascii_case(auth_request_id))
        .cloned()
        .map_or(Selection::Foreign, Selection::Use)
}

/// `mobile_signin_verifier.json` in the data dir.
pub struct VerifierStore {
    path: PathBuf,
}

impl VerifierStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(FILE_NAME),
        }
    }

    fn load_all(&self) -> Vec<PendingVerifier> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save_all(&self, entries: &[PendingVerifier]) -> std::io::Result<()> {
        let json = serde_json::to_vec(entries)?;
        let tmp = self.path.with_extension("json.tmp");
        let mut out = std::fs::File::create(&tmp)?;
        out.write_all(&json)?;
        out.sync_all()?;
        std::fs::rename(&tmp, &self.path)
    }

    /// Persists a new attempt's verifier and returns its `authRequestId`, or `None` when it could
    /// not be saved: the browser must then be opened without one, as before this binding existed.
    pub fn add_pending(&self, verifier: String) -> Option<String> {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let auth_request_id = derive_auth_request_id(&verifier);
        let mut entries = self.load_all();
        push(
            &mut entries,
            PendingVerifier {
                auth_request_id: auth_request_id.clone(),
                verifier,
                created_at_unix: now_unix(),
            },
            now_unix(),
        );
        self.save_all(&entries).ok().map(|()| auth_request_id)
    }

    pub fn select_pending(&self, auth_request_id: &str) -> Selection {
        select(&self.load_all(), auth_request_id)
    }

    pub fn pending_ids(&self) -> Vec<String> {
        self.load_all()
            .into_iter()
            .map(|e| e.auth_request_id)
            .collect()
    }

    pub fn remove_pending(&self, auth_request_id: &str) {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries = self.load_all();
        let before = entries.len();
        entries.retain(|e| !e.auth_request_id.eq_ignore_ascii_case(auth_request_id));
        if entries.len() != before && self.save_all(&entries).is_err() {
            tracing::warn!("signin_verifier: failed to remove a used verifier");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shared with auth-server, the auth site, unity-explorer and creator-hub.
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const AUTH_REQUEST_ID: &str = "13d31e96-1a1a-48ec-af16-b10c4c982e08";
    const NOW: u64 = 1_800_000_000;

    fn entry(verifier: &str, created_at_unix: u64) -> PendingVerifier {
        PendingVerifier {
            auth_request_id: derive_auth_request_id(verifier),
            verifier: verifier.to_string(),
            created_at_unix,
        }
    }

    fn entries(count: usize) -> Vec<PendingVerifier> {
        let mut entries = vec![];
        for i in 0..count {
            push(&mut entries, entry(&format!("verifier-{i}"), NOW), NOW);
        }
        entries
    }

    fn store(name: &str) -> (VerifierStore, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("dcl_core_verifier_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (VerifierStore::new(&dir), dir)
    }

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

    #[test]
    fn a_matching_id_picks_its_own_verifier() {
        let entries = entries(3);
        assert_eq!(
            select(&entries, &entries[0].auth_request_id.to_uppercase()),
            Selection::Use(entries[0].clone())
        );
    }

    #[test]
    fn an_unknown_id_is_foreign() {
        assert_eq!(
            select(&entries(2), "11111111-1111-4111-8111-111111111111"),
            Selection::Foreign
        );
    }

    #[test]
    fn no_id_uses_the_most_recent_verifier() {
        let entries = entries(2);
        assert_eq!(select(&entries, ""), Selection::Use(entries[1].clone()));
    }

    #[test]
    fn no_id_and_nothing_pending_fetches_without_a_verifier() {
        assert_eq!(select(&[], ""), Selection::NoVerifier);
    }

    #[test]
    fn keeps_only_the_latest_attempts() {
        let entries = entries(MAX_PENDING + 2);
        assert_eq!(entries.len(), MAX_PENDING);
        assert_eq!(entries[0].verifier, "verifier-2");
        assert_eq!(entries[MAX_PENDING - 1].verifier, "verifier-4");
    }

    #[test]
    fn housekeeping_drops_day_old_entries_when_a_new_one_is_added() {
        let mut entries = vec![entry("old", NOW - HOUSEKEEPING_MAX_AGE_SECS - 1)];
        push(&mut entries, entry("new", NOW), NOW);
        assert_eq!(entries, vec![entry("new", NOW)]);
    }

    #[test]
    fn an_old_matching_entry_is_still_selected() {
        let entries = vec![entry(VERIFIER, NOW - 2 * 3600)];
        assert_eq!(
            select(&entries, AUTH_REQUEST_ID),
            Selection::Use(entries[0].clone())
        );
    }

    #[test]
    fn a_stored_verifier_survives_a_restart_and_is_removed_once_used() {
        let (store, dir) = store("round_trip");
        assert_eq!(
            store.add_pending(VERIFIER.to_string()).as_deref(),
            Some(AUTH_REQUEST_ID)
        );

        let reopened = VerifierStore::new(&dir);
        assert!(matches!(
            reopened.select_pending(AUTH_REQUEST_ID),
            Selection::Use(p) if p.verifier == VERIFIER
        ));
        reopened.remove_pending(&AUTH_REQUEST_ID.to_uppercase());
        assert_eq!(reopened.select_pending(AUTH_REQUEST_ID), Selection::Foreign);
        assert_eq!(reopened.select_pending(""), Selection::NoVerifier);
    }

    #[test]
    fn a_damaged_file_reads_as_nothing_pending() {
        let (store, dir) = store("damaged");
        std::fs::write(dir.join(FILE_NAME), "{not json").unwrap();
        assert!(store.pending_ids().is_empty());
        assert!(store.add_pending(VERIFIER.to_string()).is_some());
    }

    #[test]
    fn only_401_404_and_410_settle_an_attempt() {
        for status in [401, 404, 410] {
            assert!(is_terminal_status(Some(status)));
        }
        for status in [Some(500), Some(503), None] {
            assert!(!is_terminal_status(status));
        }
    }
}
