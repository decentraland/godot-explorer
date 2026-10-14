//! The ways in: guest, email code, browser hand-off and an external wallet. Each moves the app
//! state, installs the wallet in the session, saves the account and tells the observers.

use std::time::{Instant, SystemTime};

use crate::{
    app::{AppEvents, SignInMethod, SignInStep, State},
    session::{
        auth_identity::{create_ephemeral_from_external_signature, generate_ephemeral_for_signing},
        decentraland_auth_server::mobile_auth_url,
        identity::SessionEvent,
        thirdweb_guest::GuestLoginError,
    },
    OpenUrlRequest,
};

use super::{GuestLoginReport, Hub};

/// A failed sign-in step. `code` is the stable value sent to the shell as `auth_error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInError {
    pub code: String,
    pub detail: String,
}

impl SignInError {
    pub(super) fn new(code: impl Into<String>, detail: impl std::fmt::Display) -> Self {
        Self {
            code: code.into(),
            detail: detail.to_string(),
        }
    }
}

impl std::fmt::Display for SignInError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SignInError {}

/// What `wallet_begin` keeps until the wallet signs: the ephemeral key never leaves the core.
pub(super) struct PendingWallet {
    message: String,
    key: Vec<u8>,
    expiration: SystemTime,
}

impl Hub {
    fn invalid_state(&self, call: &str) -> SignInError {
        let state = self.with_app(|app| app.state().clone());
        SignInError::new("invalid_state", format!("{call} in {state:?}"))
    }

    fn begin(&self, method: SignInMethod) -> Result<(), SignInError> {
        self.with_app(|app| app.begin_sign_in(method))
            .map_err(|e| SignInError::new("invalid_state", e))
    }

    /// Back to `SignedOut`, and the shell hears why.
    fn fail(&self, code: &str, detail: impl std::fmt::Display) -> SignInError {
        let error = SignInError::new(code, detail);
        tracing::warn!("sign-in failed: {error}");
        let _ = self.with_app(|app| app.sign_in_failed());
        self.events.auth_error(code);
        error
    }

    fn succeed(&self, event: SessionEvent) {
        self.notify(&event);
        self.save_account();
        if let Err(e) = self.with_app(|app| app.sign_in_succeeded()) {
            tracing::warn!("sign-in succeeded outside a sign-in: {e}");
        }
    }

    /// Silent guest login: the same device always gets the same wallet.
    pub async fn sign_in_guest(&self) -> Result<(), SignInError> {
        self.begin(SignInMethod::Guest)?;
        let anchor = self
            .device_anchor
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let started = Instant::now();
        let result = self.backend.guest(anchor).await;
        let duration_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(outcome) => {
                let event = self.with_session(|s| {
                    let event = s.install_remote(outcome.address, 1, outcome.chain);
                    s.is_thirdweb_guest = true;
                    event
                });
                self.notify_guest_login(&GuestLoginReport {
                    success: true,
                    is_new_user: outcome.is_new_user,
                    failure_reason: String::new(),
                    http_status: None,
                    duration_ms,
                });
                self.succeed(event);
                Ok(())
            }
            Err(GuestLoginError {
                reason,
                http_status,
                message,
            }) => {
                self.notify_guest_login(&GuestLoginReport {
                    success: false,
                    is_new_user: false,
                    failure_reason: reason.to_string(),
                    http_status,
                    duration_ms,
                });
                Err(self.fail(&format!("guest_{reason}"), message))
            }
        }
    }

    /// Sends the code and waits for it.
    pub async fn sign_in_email_start(&self, email: &str) -> Result<(), SignInError> {
        self.begin(SignInMethod::Email)?;
        match self.backend.email_send(email).await {
            Ok(()) => self
                .with_app(|app| app.sign_in_step(SignInStep::AwaitingCode))
                .map_err(|e| SignInError::new("invalid_state", e)),
            Err(e) => Err(self.fail("email_send_failed", e)),
        }
    }

    /// A wrong code keeps the user on the code step so they can try again.
    pub async fn sign_in_email_verify(&self, email: &str, code: &str) -> Result<(), SignInError> {
        let awaiting = matches!(
            self.with_app(|app| app.state().clone()),
            State::SigningIn {
                method: SignInMethod::Email,
                step: SignInStep::AwaitingCode
            }
        );
        if !awaiting {
            return Err(self.invalid_state("sign_in_email_verify"));
        }
        match self.backend.email_verify(email, code).await {
            Ok((address, chain)) => {
                let event = self.with_session(|s| s.install_remote(address, 1, chain));
                self.succeed(event);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("email code rejected: {e}");
                self.events.auth_error("invalid_code");
                Err(SignInError::new("invalid_code", e))
            }
        }
    }

    /// Asks the shell to open the auth page; `complete_browser` finishes it from the deep link.
    pub fn sign_in_browser(
        &self,
        provider: Option<String>,
        user_id: Option<String>,
        session_id: Option<String>,
    ) -> Result<(), SignInError> {
        self.begin(SignInMethod::Browser)?;
        self.with_session(|s| {
            s.begin_mobile_auth();
            s.set_pending_mobile_auth();
        });
        self.events.open_url(&OpenUrlRequest {
            url: mobile_auth_url(provider, user_id, session_id),
            description: String::new(),
            use_webview: true,
        });
        Ok(())
    }

    /// Finishes a browser sign-in with the identity id from `decentraland://open?signin=<id>`.
    /// Not gated on a pending attempt: the OS may have killed the process during the browser hop,
    /// and the deep link then arrives on a cold start.
    pub async fn complete_browser(&self, identity_id: String) -> Result<(), SignInError> {
        let state = self.with_app(|app| app.state().clone());
        if matches!(state, State::SignedOut) {
            self.begin(SignInMethod::Browser)?;
        } else if !matches!(
            state,
            State::SigningIn {
                method: SignInMethod::Browser,
                ..
            }
        ) {
            return Err(self.invalid_state("complete_browser"));
        }
        let attempt = self.with_session(|s| {
            s.take_pending_mobile_auth();
            s.mobile_auth_attempt()
        });
        let result = self.backend.browser_identity(identity_id).await;
        if !self.with_session(|s| s.is_current_mobile_attempt(attempt)) {
            return Err(SignInError::new("cancelled", "the sign-in was cancelled"));
        }
        match result {
            Ok((chain, chain_id)) => {
                let address = chain.signer();
                let event = self.with_session(|s| s.install_remote(address, chain_id, chain));
                self.succeed(event);
                Ok(())
            }
            Err(e) => Err(self.fail("browser_completion_failed", e)),
        }
    }

    /// The user closed the browser or the wallet before finishing.
    pub fn cancel_sign_in(&self) {
        self.with_session(|s| s.abort_connect());
        *self
            .wallet_pending
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        let _ = self.with_app(|app| app.sign_in_failed());
    }

    /// Starts an external-wallet sign-in. Returns the message the wallet must sign to delegate to
    /// a fresh ephemeral key, which stays here.
    pub fn sign_in_wallet_begin(&self) -> Result<String, SignInError> {
        self.begin(SignInMethod::Wallet)?;
        let (message, key, expiration) = generate_ephemeral_for_signing();
        *self
            .wallet_pending
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(PendingWallet {
            message: message.clone(),
            key,
            expiration,
        });
        Ok(message)
    }

    /// Finishes the wallet sign-in with the signature of the message `sign_in_wallet_begin` gave.
    pub fn sign_in_wallet_complete(
        &self,
        signer_address: &str,
        signature: &str,
    ) -> Result<(), SignInError> {
        let Some(pending) = self
            .wallet_pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        else {
            return Err(self.invalid_state("sign_in_wallet_complete"));
        };
        match create_ephemeral_from_external_signature(
            signer_address,
            signature,
            &pending.key,
            pending.expiration,
            &pending.message,
        ) {
            Ok(chain) => {
                let address = chain.signer();
                let event = self.with_session(|s| s.install_walletconnect(address, chain));
                self.succeed(event);
                Ok(())
            }
            Err(e) => Err(self.fail("wallet_signature_invalid", e)),
        }
    }

    /// Forgets the account here and on disk; the shell goes back to the sign-in screens.
    pub fn sign_out(&self) -> Result<(), SignInError> {
        if let Some(event) = self.with_session(|s| s.logout()) {
            self.notify(&event);
        }
        self.clear_saved_account();
        self.with_app(|app| app.sign_out())
            .map_err(|e| SignInError::new("invalid_state", e))
    }
}
