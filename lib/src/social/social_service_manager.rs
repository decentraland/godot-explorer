use std::sync::Arc;

use anyhow::Result;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::urls;

use super::api::{FriendshipRequestIntention, Pagination, RequestsDirection, SocialApiClient};
use super::profiles::ProfileResolver;
pub use super::stream::ConnectionState;
use super::stream::SocialStream;
use super::types::{
    BlockStreamEvent, BlockedUser, BlockingStatus, FriendConnectivityStreamEvent, FriendProfile,
    FriendshipRequest, FriendshipRequestEntry, FriendshipStatusResponse, FriendshipStreamEvent,
    Paginated,
};

const FRIENDSHIP_UPDATES: &str = "friendship_updates";
const FRIEND_CONNECTIVITY_UPDATES: &str = "friend_connectivity_updates";
const BLOCK_UPDATES: &str = "block_updates";

/// Social service client: plain signed HTTP for requests and the `/v1/stream` WebSocket for
/// real-time updates. The WebSocket is opened on the first subscription; requests never need it.
/// The service answers with addresses only, so the lists resolve their profiles here.
pub struct SocialServiceManager {
    api: SocialApiClient,
    stream: SocialStream,
    profiles: ProfileResolver,
}

impl SocialServiceManager {
    pub fn new(wallet: Arc<EphemeralAuthChain>) -> Self {
        Self::with_urls(wallet, urls::social_api(), urls::social_service_stream())
    }

    pub fn with_urls(wallet: Arc<EphemeralAuthChain>, api_url: String, stream_url: String) -> Self {
        Self {
            api: SocialApiClient::new(api_url, wallet.clone()),
            stream: SocialStream::new(stream_url, wallet),
            profiles: ProfileResolver::new(),
        }
    }

    /// State of the real-time stream connection.
    pub fn connection_state(&self) -> ConnectionState {
        self.stream.connection_state()
    }

    // ========================================
    // Friends
    // ========================================

    pub async fn get_friends(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<FriendProfile>> {
        let page = self.api.get_friends(pagination).await?;
        let addresses: Vec<String> = page.results.into_iter().map(|f| f.address).collect();
        Ok(Paginated {
            results: self.profiles.resolve(&addresses).await,
            total: page.total,
        })
    }

    pub async fn get_mutual_friends(
        &self,
        user_address: &str,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<FriendProfile>> {
        let page = self
            .api
            .get_mutual_friends(user_address, pagination)
            .await?;
        let addresses: Vec<String> = page.results.into_iter().map(|f| f.address).collect();
        Ok(Paginated {
            results: self.profiles.resolve(&addresses).await,
            total: page.total,
        })
    }

    pub async fn get_pending_friendship_requests(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<FriendshipRequest>> {
        let page = self
            .api
            .get_friendship_requests(RequestsDirection::Received, pagination)
            .await?;
        Ok(self.with_profiles(page).await)
    }

    pub async fn get_sent_friendship_requests(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<FriendshipRequest>> {
        let page = self
            .api
            .get_friendship_requests(RequestsDirection::Sent, pagination)
            .await?;
        Ok(self.with_profiles(page).await)
    }

    pub async fn send_friend_request(
        &self,
        user_address: &str,
        message: Option<String>,
    ) -> Result<FriendshipRequestEntry> {
        self.api
            .send_friendship_request(user_address, message)
            .await
    }

    pub async fn accept_friend_request(&self, user_address: &str) -> Result<()> {
        self.api
            .update_friendship_request(user_address, FriendshipRequestIntention::Accepted)
            .await
    }

    pub async fn reject_friend_request(&self, user_address: &str) -> Result<()> {
        self.api
            .update_friendship_request(user_address, FriendshipRequestIntention::Rejected)
            .await
    }

    pub async fn cancel_friend_request(&self, user_address: &str) -> Result<()> {
        self.api
            .update_friendship_request(user_address, FriendshipRequestIntention::Cancelled)
            .await
    }

    pub async fn delete_friendship(&self, user_address: &str) -> Result<()> {
        self.api.remove_friend(user_address).await
    }

    pub async fn get_friendship_status(
        &self,
        user_address: &str,
    ) -> Result<FriendshipStatusResponse> {
        self.api.get_friendship_status(user_address).await
    }

    // ========================================
    // Blocks
    // ========================================

    pub async fn block_user(&self, user_address: &str) -> Result<()> {
        self.api.block_user(user_address).await
    }

    pub async fn unblock_user(&self, user_address: &str) -> Result<()> {
        self.api.unblock_user(user_address).await
    }

    pub async fn get_blocked_users(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<Paginated<BlockedUser>> {
        let page = self.api.get_blocked_users(pagination).await?;
        let addresses: Vec<String> = page.results.iter().map(|b| b.address.clone()).collect();
        let profiles = self.profiles.resolve(&addresses).await;
        Ok(Paginated {
            results: page
                .results
                .into_iter()
                .zip(profiles)
                .map(|(entry, profile)| BlockedUser {
                    profile,
                    blocked_at: entry.blocked_at,
                })
                .collect(),
            total: page.total,
        })
    }

    pub async fn get_blocking_status(&self) -> Result<BlockingStatus> {
        self.api.get_blocking_status().await
    }

    async fn with_profiles(
        &self,
        page: Paginated<FriendshipRequestEntry>,
    ) -> Paginated<FriendshipRequest> {
        let addresses: Vec<String> = page.results.iter().map(|r| r.address.clone()).collect();
        let profiles = self.profiles.resolve(&addresses).await;
        Paginated {
            results: page
                .results
                .into_iter()
                .zip(profiles)
                .map(|(entry, friend)| FriendshipRequest {
                    id: entry.id,
                    friend,
                    created_at: entry.created_at,
                    message: entry.message,
                })
                .collect(),
            total: page.total,
        }
    }

    // ========================================
    // Real-time updates
    // ========================================

    pub async fn subscribe_to_friendship_updates(
        &self,
    ) -> Result<UnboundedReceiver<FriendshipStreamEvent>> {
        self.subscribe(FRIENDSHIP_UPDATES).await
    }

    pub async fn subscribe_to_friend_connectivity_updates(
        &self,
    ) -> Result<UnboundedReceiver<FriendConnectivityStreamEvent>> {
        self.subscribe(FRIEND_CONNECTIVITY_UPDATES).await
    }

    pub async fn subscribe_to_block_updates(&self) -> Result<UnboundedReceiver<BlockStreamEvent>> {
        self.subscribe(BLOCK_UPDATES).await
    }

    /// Subscribes to `stream` and parses its events into `T`, skipping (and logging) events
    /// this client does not understand. The receiver ends when the subscription does.
    async fn subscribe<T: DeserializeOwned + Send + 'static>(
        &self,
        stream: &'static str,
    ) -> Result<UnboundedReceiver<T>> {
        let mut raw = self.stream.subscribe(stream).await?;
        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(data) = raw.recv().await {
                match serde_json::from_value::<T>(data.clone()) {
                    Ok(event) => {
                        if sender.send(event).is_err() {
                            break;
                        }
                    }
                    Err(e) => tracing::warn!("Skipping a {stream} event ({e}): {data}"),
                }
            }
        });

        Ok(receiver)
    }
}
