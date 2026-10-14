//! FFI for the profile domain: the own profile, public profiles, owned names, deploying edits
//! and the account deletion request. `sign_out` is in `session`.

use std::sync::{Arc, OnceLock};

use super::{core, CoreError};
use crate::profile::{
    Endpoints, Profile, ProfileError, ProfileService, ProfileUpdate, ReqwestTransport,
    SessionIdentity,
};

impl From<ProfileError> for CoreError {
    fn from(error: ProfileError) -> Self {
        Self::InvalidCall {
            detail: error.to_string(),
        }
    }
}

fn service() -> Arc<ProfileService> {
    static SERVICE: OnceLock<Arc<ProfileService>> = OnceLock::new();
    SERVICE
        .get_or_init(|| {
            Arc::new(ProfileService::new(
                Arc::new(ReqwestTransport::new()),
                Arc::new(SessionIdentity),
                Endpoints::live(),
            ))
        })
        .clone()
}

/// Runs the call on the core runtime, so it has a tokio context whatever executor polls it.
async fn on_core<T, F>(future: F) -> Result<T, CoreError>
where
    T: Send + 'static,
    F: std::future::Future<Output = Result<T, ProfileError>> + Send + 'static,
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

/// The own profile (`address` empty or `None`, or the signed-in address) or a public one.
#[uniffi::export]
pub async fn profile(address: Option<String>) -> Result<Profile, CoreError> {
    let service = service();
    on_core(async move { service.profile(address.as_deref()).await }).await
}

/// The claimed names the account owns.
#[uniffi::export]
pub async fn owned_names() -> Result<Vec<String>, CoreError> {
    let service = service();
    on_core(async move { service.owned_names().await }).await
}

/// Applies the edits to the own profile and publishes it; returns the profile as published.
#[uniffi::export]
pub async fn deploy_profile(update: ProfileUpdate) -> Result<Profile, CoreError> {
    let service = service();
    on_core(async move { service.deploy(&update).await }).await
}

/// Asks the server to delete the account; the request is processed on its side.
#[uniffi::export]
pub async fn delete_account() -> Result<(), CoreError> {
    let service = service();
    on_core(async move { service.request_account_deletion().await }).await
}
