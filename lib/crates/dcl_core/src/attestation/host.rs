//! The OS half of attestation, implemented by the host (Kotlin or Swift).

use async_trait::async_trait;

use crate::Platform;

/// Errors are the OS error text; the core wraps them with the failing step.
#[async_trait]
pub trait AttestationHost: Send + Sync {
    fn platform(&self) -> Platform;

    /// False on desktop, the iOS simulator and Android without Play Services.
    fn is_supported(&self) -> bool;

    /// Android: Play Integrity token whose nonce is the base64url (no padding) request hash.
    async fn play_integrity_token(&self, request_hash_b64url: String) -> Result<String, String>;

    /// iOS: `DCAppAttestService.generateKey`, returns the key id.
    async fn ios_generate_key(&self) -> Result<String, String>;

    /// iOS: `attestKey(key_id, clientDataHash)`, returns the attestation object as base64url.
    async fn ios_attest_key(
        &self,
        key_id: String,
        client_data_hash: Vec<u8>,
    ) -> Result<String, String>;
}
