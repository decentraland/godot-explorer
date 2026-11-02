//! Platform attestation for the core. The host owns the OS calls (Play Integrity, App Attest);
//! the core owns the session cycle, the cache and the retries.

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use dcl_core::attestation::{Attestation, AttestationHost, State};
use dcl_core::Platform;

use crate::{core, HostError};

#[derive(Debug, uniffi::Error)]
pub enum AttestationFailure {
    Failed { reason: String },
}

impl std::fmt::Display for AttestationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { reason } => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for AttestationFailure {}

/// Implemented in Kotlin/Swift: the OS calls attestation needs. Called from a core thread.
#[uniffi::export(with_foreign)]
#[async_trait]
pub trait AttestationService: Send + Sync {
    /// False on the iOS simulator and Android without Play Services.
    fn is_supported(&self) -> bool;
    /// Android: a Play Integrity token whose nonce is the base64url (no padding) request hash.
    async fn play_integrity_token(
        &self,
        request_hash_b64url: String,
    ) -> Result<String, AttestationFailure>;
    /// iOS: `DCAppAttestService.generateKey`, the key id.
    async fn ios_generate_key(&self) -> Result<String, AttestationFailure>;
    /// iOS: `attestKey`, the attestation object as base64url.
    async fn ios_attest_key(
        &self,
        key_id: String,
        client_data_hash: Vec<u8>,
    ) -> Result<String, AttestationFailure>;
}

struct ServiceAdapter {
    service: Arc<dyn AttestationService>,
    platform: Platform,
}

#[async_trait]
impl AttestationHost for ServiceAdapter {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn is_supported(&self) -> bool {
        self.service.is_supported()
    }

    async fn play_integrity_token(&self, request_hash_b64url: String) -> Result<String, String> {
        self.service
            .play_integrity_token(request_hash_b64url)
            .await
            .map_err(|e| e.to_string())
    }

    async fn ios_generate_key(&self) -> Result<String, String> {
        self.service
            .ios_generate_key()
            .await
            .map_err(|e| e.to_string())
    }

    async fn ios_attest_key(
        &self,
        key_id: String,
        client_data_hash: Vec<u8>,
    ) -> Result<String, String> {
        self.service
            .ios_attest_key(key_id, client_data_hash)
            .await
            .map_err(|e| e.to_string())
    }
}

static ATTESTATION: OnceLock<Attestation> = OnceLock::new();

/// Where attestation stands, for diagnostics.
#[derive(Debug, uniffi::Record)]
pub struct AttestationStatus {
    pub supported: bool,
    /// `idle`, `waiting_eula`, `attesting` or `backoff`.
    pub state: String,
    pub has_valid_session: bool,
    pub last_failure: Option<String>,
}

/// Hands the core the OS half of attestation and starts it: the session is created once the
/// player accepted the terms, cached on disk and renewed by the core. Calling it again is a no-op.
#[uniffi::export]
pub fn install_attestation(service: Arc<dyn AttestationService>) -> Result<(), HostError> {
    let core = core()?;
    if ATTESTATION.get().is_some() {
        return Ok(());
    }
    let host = Arc::new(ServiceAdapter {
        service,
        platform: core.config().platform,
    });
    let attestation = Attestation::new(
        host,
        Arc::new(dcl_core::ffi::eula_accepted),
        !cfg!(debug_assertions),
    )
    .map_err(|_| HostError::NotInstalled)?;
    let _runtime = core.handle().enter();
    attestation.start();
    let _ = ATTESTATION.set(attestation);
    Ok(())
}

#[uniffi::export]
pub fn attestation_status() -> Option<AttestationStatus> {
    ATTESTATION.get().map(status_of)
}

fn status_of(attestation: &Attestation) -> AttestationStatus {
    AttestationStatus {
        supported: attestation.is_supported(),
        state: match attestation.state() {
            State::Idle => "idle",
            State::WaitingEula => "waiting_eula",
            State::Attesting => "attesting",
            State::Backoff => "backoff",
        }
        .to_string(),
        has_valid_session: attestation.has_valid_session(),
        last_failure: attestation
            .last_failure()
            .map(|f| format!("{}: {}", f.step, f.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        supported: bool,
        token: Result<String, String>,
    }

    #[async_trait]
    impl AttestationService for Fake {
        fn is_supported(&self) -> bool {
            self.supported
        }
        async fn play_integrity_token(&self, hash: String) -> Result<String, AttestationFailure> {
            self.token
                .clone()
                .map(|t| format!("{t}:{hash}"))
                .map_err(|reason| AttestationFailure::Failed { reason })
        }
        async fn ios_generate_key(&self) -> Result<String, AttestationFailure> {
            Ok("key".into())
        }
        async fn ios_attest_key(
            &self,
            key_id: String,
            _: Vec<u8>,
        ) -> Result<String, AttestationFailure> {
            Ok(key_id)
        }
    }

    fn adapter(fake: Fake) -> ServiceAdapter {
        ServiceAdapter {
            service: Arc::new(fake),
            platform: Platform::Android,
        }
    }

    #[tokio::test]
    async fn the_adapter_passes_the_host_answers_through_as_text() {
        let ok = adapter(Fake {
            supported: true,
            token: Ok("jwt".into()),
        });
        assert!(ok.is_supported());
        assert_eq!(ok.platform(), Platform::Android);
        assert_eq!(
            ok.play_integrity_token("h".into()).await,
            Ok("jwt:h".to_string())
        );
        assert_eq!(ok.ios_generate_key().await, Ok("key".to_string()));

        let failing = adapter(Fake {
            supported: false,
            token: Err("no play services".into()),
        });
        assert!(!failing.is_supported());
        assert_eq!(
            failing.play_integrity_token("h".into()).await,
            Err("no play services".to_string())
        );
    }
}
