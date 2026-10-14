//! FFI for the places domain: the signed reads and writes of places, event reminders and credits.
//! Each call is typed; the identity key stays in the core.

use std::future::Future;

use super::{core, CoreError};
use crate::places::{self, CreditsBalance, Place, PlaceUserState, SignedClient, SignedError, Vote};

impl From<SignedError> for CoreError {
    fn from(error: SignedError) -> Self {
        Self::InvalidCall {
            detail: error.to_string(),
        }
    }
}

/// Runs `call` on the core runtime with the client of the signed-in account, so the HTTP stack has
/// a tokio context whatever executor the foreign side polls from.
pub(super) async fn on_core<T, F, Fut>(call: F) -> Result<T, CoreError>
where
    T: Send + 'static,
    F: FnOnce(SignedClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, SignedError>> + Send + 'static,
{
    let core = core()?;
    core.handle()
        .spawn(async move {
            let client = SignedClient::live(core)?;
            call(client).await
        })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?
        .map_err(CoreError::from)
}

/// One page of the user's favorite places; `search` narrows it by text (and ignores `offset`).
#[uniffi::export]
pub async fn favorite_places(
    offset: u32,
    limit: u32,
    search: Option<String>,
) -> Result<Vec<Place>, CoreError> {
    on_core(move |client| async move {
        places::favorite_places(&client, offset, limit, search.as_deref()).await
    })
    .await
}

/// The user's favorite flag and vote on a place; `None` when the service does not know it.
#[uniffi::export]
pub async fn place_user_state(place_id: String) -> Result<Option<PlaceUserState>, CoreError> {
    on_core(move |client| async move { places::place_user_state(&client, &place_id).await }).await
}

#[uniffi::export]
pub async fn set_favorite(place_id: String, value: bool, is_world: bool) -> Result<(), CoreError> {
    on_core(move |client| async move {
        places::set_favorite(&client, &place_id, value, is_world).await
    })
    .await
}

#[uniffi::export]
pub async fn vote(place_id: String, choice: Vote, is_world: bool) -> Result<(), CoreError> {
    on_core(move |client| async move { places::vote(&client, &place_id, choice, is_world).await })
        .await
}

/// Sets or clears the user's reminder on an event.
#[uniffi::export]
pub async fn set_event_reminder(event_id: String, on: bool) -> Result<(), CoreError> {
    on_core(move |client| async move { places::set_event_reminder(&client, &event_id, on).await })
        .await
}

/// Spendable credits of the signed-in wallet.
#[uniffi::export]
pub async fn credits_balance() -> Result<CreditsBalance, CoreError> {
    on_core(|client| async move { places::credits_balance(&client).await }).await
}
