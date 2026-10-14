//! Friends, as the shell calls it. The rules live in `crate::social`; these run them on the core's
//! runtime (they use the network) and map their errors.

use std::future::Future;

use super::{core, CoreError};
use crate::social::{self, Friend, FriendRequests, NearbyUser, SocialError};

impl From<SocialError> for CoreError {
    fn from(error: SocialError) -> Self {
        Self::InvalidCall {
            detail: error.to_string(),
        }
    }
}

/// Runs `future` on the core runtime and waits for it, so the calls have a tokio context whatever
/// executor the foreign side polls from.
async fn on_core<T, F>(future: F) -> Result<T, CoreError>
where
    T: Send + 'static,
    F: Future<Output = Result<T, SocialError>> + Send + 'static,
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

/// One page of the friends list. `limit` 0 asks for the service's default page.
#[uniffi::export]
pub async fn friends(limit: u32, offset: u32) -> Result<Vec<Friend>, CoreError> {
    on_core(async move { social::friends().friends(limit, offset).await }).await
}

/// Pending requests, received and sent.
#[uniffi::export]
pub async fn friend_requests() -> Result<FriendRequests, CoreError> {
    on_core(async move { social::friends().friend_requests().await }).await
}

/// Players around, without the blocked ones.
#[uniffi::export]
pub async fn nearby() -> Result<Vec<NearbyUser>, CoreError> {
    let users = core()?
        .handle()
        .spawn(async move { social::friends().nearby().await })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?;
    Ok(users)
}

#[uniffi::export]
pub async fn send_friend_request(address: String, message: String) -> Result<(), CoreError> {
    on_core(async move { social::friends().send_request(&address, &message).await }).await
}

#[uniffi::export]
pub async fn accept_friend_request(address: String) -> Result<(), CoreError> {
    on_core(async move { social::friends().accept(&address).await }).await
}

#[uniffi::export]
pub async fn reject_friend_request(address: String) -> Result<(), CoreError> {
    on_core(async move { social::friends().reject(&address).await }).await
}

#[uniffi::export]
pub async fn cancel_friend_request(address: String) -> Result<(), CoreError> {
    on_core(async move { social::friends().cancel(&address).await }).await
}

#[uniffi::export]
pub async fn block_user(address: String) -> Result<(), CoreError> {
    on_core(async move { social::friends().block(&address).await }).await
}

#[uniffi::export]
pub async fn unblock_user(address: String) -> Result<(), CoreError> {
    on_core(async move { social::friends().unblock(&address).await }).await
}
