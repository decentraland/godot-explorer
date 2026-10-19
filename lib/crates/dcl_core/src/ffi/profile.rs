//! FFI for the profile domain: the own profile, public profiles, owned names, deploying edits
//! and the account deletion request. `sign_out` is in `session`.

use std::sync::Arc;

use super::{core, CoreError};
use crate::{
    hub::DeletionPlan,
    profile::{Profile, ProfileError, ProfileService, ProfileUpdate},
};

impl From<ProfileError> for CoreError {
    fn from(error: ProfileError) -> Self {
        Self::InvalidCall {
            detail: error.to_string(),
        }
    }
}

fn service() -> Arc<ProfileService> {
    crate::profile::global()
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

/// What deleting the account did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum AccountDeletion {
    /// The server processes the request; the player stays signed in.
    Requested,
    /// A guest was removed from the device and signed out.
    Deleted,
}

/// Deletes the account: a guest on the device (its thirdweb user is unlinked, the identity files
/// wiped, the player signed out), any other account through the server's deletion request.
#[uniffi::export]
pub async fn delete_account() -> Result<AccountDeletion, CoreError> {
    let core = core()?;
    let service = service();
    core.handle()
        .spawn(async move {
            let hub = core.hub();
            match hub.deletion_plan().await {
                DeletionPlan::Server => service
                    .request_account_deletion()
                    .await
                    .map(|()| AccountDeletion::Requested)
                    .map_err(CoreError::from),
                plan => hub
                    .delete_guest(plan)
                    .await
                    .map(|()| AccountDeletion::Deleted)
                    .map_err(CoreError::from),
            }
        })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?
}
