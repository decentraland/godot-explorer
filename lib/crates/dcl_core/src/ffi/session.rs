//! Signing in and out, as the shell calls it. The flows live in `crate::hub`; these run them on
//! the core's runtime (they use the network) and map their errors.

use std::future::Future;

use super::{core, CoreError};

/// Runs `future` on the core runtime and waits for it, so the flows have a tokio context whatever
/// executor the foreign side polls from.
async fn on_core<T, F>(future: F) -> Result<T, CoreError>
where
    T: Send + 'static,
    F: Future<Output = Result<T, crate::hub::SignInError>> + Send + 'static,
{
    core()?
        .handle()
        .spawn(future)
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?
        .map_err(CoreError::from)
}

fn hub() -> Result<&'static crate::hub::Hub, CoreError> {
    Ok(core()?.hub())
}

/// Silent guest login: the same device always gets the same wallet.
#[uniffi::export]
pub async fn sign_in_guest() -> Result<(), CoreError> {
    let hub = hub()?;
    on_core(async move { hub.sign_in_guest().await }).await
}

/// Sends a one-time code to `email` and waits for it.
#[uniffi::export]
pub async fn sign_in_email_start(email: String) -> Result<(), CoreError> {
    let hub = hub()?;
    on_core(async move { hub.sign_in_email_start(&email).await }).await
}

/// Verifies the code. A wrong code keeps the sign-in on the code step.
#[uniffi::export]
pub async fn sign_in_email_verify(email: String, code: String) -> Result<(), CoreError> {
    let hub = hub()?;
    on_core(async move { hub.sign_in_email_verify(&email, &code).await }).await
}

/// Starts a browser sign-in: the page to show arrives as an `open_url` event, and
/// `complete_browser` finishes it from the deep link.
#[uniffi::export]
pub fn sign_in_browser(provider: Option<String>) -> Result<(), CoreError> {
    hub()?
        .sign_in_browser(provider, None, None)
        .map_err(CoreError::from)
}

/// Finishes a browser sign-in with the id from `decentraland://open?signin=<id>`.
#[uniffi::export]
pub async fn complete_browser(identity_id: String) -> Result<(), CoreError> {
    let hub = hub()?;
    on_core(async move { hub.complete_browser(identity_id).await }).await
}

/// The user closed the browser or the wallet before finishing.
#[uniffi::export]
pub fn cancel_sign_in() -> Result<(), CoreError> {
    hub()?.cancel_sign_in();
    Ok(())
}

/// Starts an external-wallet sign-in: returns the message the wallet must sign. The ephemeral key
/// it delegates to never leaves the core.
#[uniffi::export]
pub fn sign_in_wallet_begin() -> Result<String, CoreError> {
    hub()?.sign_in_wallet_begin().map_err(CoreError::from)
}

/// Finishes the wallet sign-in with the signature of the message `sign_in_wallet_begin` returned.
#[uniffi::export]
pub fn sign_in_wallet_complete(signer_address: String, signature: String) -> Result<(), CoreError> {
    hub()?
        .sign_in_wallet_complete(&signer_address, &signature)
        .map_err(CoreError::from)
}

/// Forgets the account here and on disk.
#[uniffi::export]
pub fn sign_out() -> Result<(), CoreError> {
    hub()?.sign_out().map_err(CoreError::from)
}

/// The signed-in address, `None` when signed out.
#[uniffi::export]
pub fn address() -> Result<Option<String>, CoreError> {
    Ok(hub()?.with_session(|s| s.try_address().map(|a| format!("{a:#x}"))))
}

/// Whether the signed-in account is a guest (the silent device login or a throwaway wallet).
#[uniffi::export]
pub fn is_guest() -> Result<bool, CoreError> {
    Ok(hub()?.with_session(|s| s.is_thirdweb_guest || s.is_disposable_guest()))
}

/// Routes a deep link. A browser sign-in it carries is finished here; the shell gets the
/// navigation, the settings the engine owns and the analytics to record.
#[uniffi::export]
pub fn handle_deep_link(
    url: String,
    start: crate::deep_link_route::StartKind,
) -> Result<crate::deep_link_route::Routed, CoreError> {
    Ok(core()?.handle_deep_link(&url, start))
}
