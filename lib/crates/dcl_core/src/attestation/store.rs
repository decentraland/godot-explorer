//! The on-disk session: `{token, expires_at, platform}`, the same file the GDScript service wrote.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const SESSION_FILE: &str = "attest_session.json";

/// Treat tokens this close to expiry as expired (clock skew and round trip).
pub const EXPIRY_MARGIN_SECS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub token: String,
    /// Unix seconds.
    pub expires_at: i64,
}

impl Session {
    pub fn is_valid_at(&self, now: i64) -> bool {
        !self.token.is_empty() && now < self.expires_at - EXPIRY_MARGIN_SECS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLoad {
    Hit,
    MissNoFile,
    MissExpired,
    MissCorrupted,
}

#[derive(Serialize, Deserialize)]
struct Persisted {
    token: String,
    expires_at: i64,
    #[serde(default)]
    platform: String,
}

/// An expired but well-formed session is returned too, with `MissExpired`.
pub fn load(path: &Path, now: i64) -> (Option<Session>, CacheLoad) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (None, CacheLoad::MissNoFile),
        Err(e) => {
            tracing::warn!("[Attestation] cannot read {}: {e}", path.display());
            return (None, CacheLoad::MissCorrupted);
        }
    };
    let Ok(persisted) = serde_json::from_str::<Persisted>(&text) else {
        tracing::warn!(
            "[Attestation] {} is not a valid session file",
            path.display()
        );
        return (None, CacheLoad::MissCorrupted);
    };
    if persisted.token.is_empty() || persisted.expires_at <= 0 {
        return (None, CacheLoad::MissCorrupted);
    }
    let session = Session {
        token: persisted.token,
        expires_at: persisted.expires_at,
    };
    let result = if session.is_valid_at(now) {
        CacheLoad::Hit
    } else {
        CacheLoad::MissExpired
    };
    (Some(session), result)
}

pub fn save(path: &Path, session: &Session, platform: &str) {
    let persisted = Persisted {
        token: session.token.clone(),
        expires_at: session.expires_at,
        platform: platform.to_owned(),
    };
    let written = serde_json::to_string(&persisted)
        .map_err(|e| e.to_string())
        .and_then(|json| std::fs::write(path, json).map_err(|e| e.to_string()));
    if let Err(e) = written {
        tracing::warn!(
            "[Attestation] could not persist session to {}: {e}",
            path.display()
        );
    }
}

pub fn clear(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("[Attestation] could not remove {}: {e}", path.display());
        }
    }
}
