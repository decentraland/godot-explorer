use async_trait::async_trait;
use ethers_core::types::H160;

use crate::session::{
    auth_identity::complete_mobile_auth,
    ephemeral_auth_chain::EphemeralAuthChain,
    review_login,
    thirdweb_flows::{perform_email_login, perform_thirdweb_guest_login, GuestLoginOutcome},
    thirdweb_guest::{self, GuestLoginError},
};

/// The network side of signing in, behind a trait so the flows are testable offline.
#[async_trait]
pub trait SignInBackend: Send + Sync {
    async fn guest(&self, device_anchor: String) -> Result<GuestLoginOutcome, GuestLoginError>;
    async fn email_send(&self, email: &str) -> Result<(), anyhow::Error>;
    async fn email_verify(
        &self,
        email: &str,
        code: &str,
    ) -> Result<(H160, EphemeralAuthChain), anyhow::Error>;
    /// The identity the auth server minted for a browser sign-in, from the id in the deep link.
    async fn browser_identity(
        &self,
        identity_id: String,
    ) -> Result<(EphemeralAuthChain, u64), anyhow::Error>;
}

/// Thirdweb for guest and email, the auth server for the browser hand-off, and mobile-bff for the
/// App Review address.
pub struct LiveBackend;

#[async_trait]
impl SignInBackend for LiveBackend {
    async fn guest(&self, device_anchor: String) -> Result<GuestLoginOutcome, GuestLoginError> {
        perform_thirdweb_guest_login(device_anchor).await
    }

    async fn email_send(&self, email: &str) -> Result<(), anyhow::Error> {
        if review_login::is_review_email(email) {
            review_login::send_code(email).await
        } else {
            thirdweb_guest::email_initiate(email).await
        }
    }

    async fn email_verify(
        &self,
        email: &str,
        code: &str,
    ) -> Result<(H160, EphemeralAuthChain), anyhow::Error> {
        if review_login::is_review_email(email) {
            review_login::verify_code(email, code).await
        } else {
            perform_email_login(email.to_string(), code.to_string()).await
        }
    }

    async fn browser_identity(
        &self,
        identity_id: String,
    ) -> Result<(EphemeralAuthChain, u64), anyhow::Error> {
        complete_mobile_auth(identity_id).await
    }
}
