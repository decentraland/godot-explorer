use anyhow::{anyhow, Result};
use dcl_rpc::client::{ClientError, ClientResultError, RpcClient};
use dcl_rpc::transports::{
    web_sockets::{tungstenite::WebSocketClient, WebSocketTransport},
    Transport,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[allow(unused_imports)]
use futures_util::StreamExt;

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::dcl::components::proto_components::social_service::v2::*;
use crate::social::friends::build_auth_chain;
use crate::urls;
const CONNECTION_TIMEOUT_SECS: u64 = 10;
const RPC_TIMEOUT_SECS: u64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

use dcl_rpc::transports::web_sockets::tungstenite::TungsteniteWebSocket;

type SocialTransport = WebSocketTransport<TungsteniteWebSocket, ()>;

/// Holder for the RPC client and service module that must stay alive
struct ServiceConnection {
    #[allow(dead_code)]
    rpc_client: RpcClient<SocialTransport>,
    service: SocialServiceClient<SocialTransport>,
}

/// Internal state for the Social Service connection
struct SocialServiceState {
    connection_state: ConnectionState,
    last_friendship_updates: Vec<FriendshipUpdate>,
    connection: Option<ServiceConnection>,
    /// Generation counter - incremented each time connection is cleared
    /// Used to prevent multiple concurrent reconnection attempts
    connection_generation: u64,
    /// Flag to indicate a connection attempt is in progress
    connecting: bool,
    /// Last connection error message (if any)
    last_error: Option<String>,
}

impl SocialServiceState {
    fn new() -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            last_friendship_updates: Vec::new(),
            connection: None,
            connection_generation: 0,
            connecting: false,
            last_error: None,
        }
    }
}

/// Manager for the Social Service RPC connection
/// Handles WebSocket connection, authentication, and friendship operations
pub struct SocialServiceManager {
    wallet: Arc<EphemeralAuthChain>,
    state: Arc<RwLock<SocialServiceState>>,
}

/// Helper function to create a fresh Social Service client connection
/// This establishes a new WebSocket connection and authenticates
/// Returns both the RpcClient (which must stay alive) and the service module
async fn create_connection(wallet: &Arc<EphemeralAuthChain>) -> Result<ServiceConnection> {
    // Establish WebSocket connection with timeout
    let social_service_url = urls::social_service();
    let ws_connection = tokio::time::timeout(
        Duration::from_secs(CONNECTION_TIMEOUT_SECS),
        WebSocketClient::connect(&social_service_url),
    )
    .await
    .map_err(|_| {
        tracing::error!(
            "WebSocket connection timeout after {}s",
            CONNECTION_TIMEOUT_SECS
        );
        anyhow!(
            "Timeout connecting to Social Service after {}s",
            CONNECTION_TIMEOUT_SECS
        )
    })?
    .map_err(|e| {
        tracing::error!("WebSocket connect failed: {:?}", e);
        anyhow!("Failed to connect to Social Service: {:?}", e)
    })?;

    let transport = WebSocketTransport::new(ws_connection);

    // Build and send auth chain
    let auth_chain_message = build_auth_chain(wallet).await?;
    transport
        .send(auth_chain_message.as_bytes().to_vec())
        .await
        .map_err(|e| {
            tracing::error!("Failed to send auth chain: {:?}", e);
            anyhow!("Failed to send auth chain: {:?}", e)
        })?;

    // Create RPC client
    let mut rpc_client = RpcClient::new(transport).await.map_err(|e| {
        tracing::error!("Failed to create RPC client: {:?}", e);
        anyhow!("Failed to create RPC client: {:?}", e)
    })?;

    // Create port and load service module
    let port = rpc_client.create_port("SocialService").await.map_err(|e| {
        tracing::error!("Failed to create port: {:?}", e);
        anyhow!("Failed to create port: {:?}", e)
    })?;

    let service = port
        .load_module::<SocialServiceClient<_>>("SocialService")
        .await
        .map_err(|e| {
            tracing::error!("Failed to load SocialService module: {:?}", e);
            anyhow!("Failed to load module: {:?}", e)
        })?;

    Ok(ServiceConnection {
        rpc_client,
        service,
    })
}

impl SocialServiceManager {
    /// Create a new SocialServiceManager with the given wallet
    pub fn new(wallet: Arc<EphemeralAuthChain>) -> Self {
        Self {
            wallet,
            state: Arc::new(RwLock::new(SocialServiceState::new())),
        }
    }

    /// Get the current connection state (async)
    pub async fn connection_state(&self) -> ConnectionState {
        self.state.read().await.connection_state
    }

    /// Try to get the current connection state synchronously (non-blocking)
    /// Returns None if the lock couldn't be acquired
    pub fn try_get_connection_state(&self) -> Option<ConnectionState> {
        self.state.try_read().ok().map(|s| s.connection_state)
    }

    /// Ensure we have a connection, creating one if needed
    /// This method handles the connection lifecycle properly to avoid multiple concurrent attempts
    async fn ensure_connected(&self) -> Result<()> {
        // First check if we already have a connection or if one is in progress
        {
            let state = self.state.read().await;
            if state.connection.is_some() {
                return Ok(());
            }
            if state.connecting {
                drop(state);
                // Wait for the other connection attempt to complete
                let max_wait_iterations = ((CONNECTION_TIMEOUT_SECS + 5) * 10) as usize;
                for _ in 0..max_wait_iterations {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let state = self.state.read().await;
                    if state.connection.is_some() {
                        return Ok(());
                    }
                    if !state.connecting {
                        if let Some(ref err) = state.last_error {
                            return Err(anyhow!("Connection failed: {}", err));
                        }
                        break;
                    }
                }
            }
        }

        // Try to acquire the connecting flag
        {
            let mut state = self.state.write().await;
            if state.connection.is_some() {
                return Ok(());
            }
            if state.connecting {
                // Lost the race, another task started connecting
                return Err(anyhow!("Connection already in progress"));
            }
            state.connecting = true;
            state.connection_state = ConnectionState::Connecting;
            state.last_error = None;
        }

        // Now we have exclusive rights to connect (connecting = true)
        let result = create_connection(&self.wallet).await;

        // Store the result
        let mut state = self.state.write().await;
        state.connecting = false;

        match result {
            Ok(connection) => {
                state.connection = Some(connection);
                state.connection_state = ConnectionState::Connected;
                Ok(())
            }
            Err(e) => {
                tracing::error!("Social Service connection failed: {}", e);
                state.last_error = Some(format!("{}", e));
                state.connection_state = ConnectionState::Disconnected;
                Err(e)
            }
        }
    }

    /// Get or create a connection, returning a reference to the service client
    /// NOTE: Prefer calling ensure_connected() first, then use get_service_from_state()
    /// This method is kept for backward compatibility with methods that hold mutable state
    async fn ensure_connection<'a>(
        &'a self,
        state: &'a mut SocialServiceState,
    ) -> Result<&'a SocialServiceClient<SocialTransport>> {
        if state.connection.is_some() {
            // Using existing connection
        } else if state.connecting {
            // Another task is connecting - we can't wait while holding the mutable borrow
            return Err(anyhow!("Connection in progress, please retry"));
        } else {
            // No connection and not connecting - create one
            state.connecting = true;
            match create_connection(&self.wallet).await {
                Ok(connection) => {
                    state.connection = Some(connection);
                    state.connecting = false;
                }
                Err(e) => {
                    state.connecting = false;
                    state.last_error = Some(format!("{}", e));
                    return Err(e);
                }
            }
        }
        Ok(&state.connection.as_ref().unwrap().service)
    }

    /// Try to clear connection only if generation matches (prevents duplicate clears)
    /// Returns true if connection was cleared, false if already cleared by another caller
    fn try_clear_connection(state: &mut SocialServiceState, expected_generation: u64) -> bool {
        if state.connection_generation == expected_generation && state.connection.is_some() {
            state.connection = None;
            state.connection_generation += 1;
            true
        } else {
            false
        }
    }

    /// Get the list of friends for the authenticated user
    pub async fn get_friends(
        &self,
        pagination: Option<Pagination>,
        _status: Option<i32>,
    ) -> Result<PaginatedFriendsProfilesResponse> {
        let payload = GetFriendsPayload {
            pagination: pagination.clone(),
        };

        // First attempt - capture generation before call
        let (result, generation) = self.call_get_friends(payload.clone()).await;

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("get_friends RPC failed, retrying: {:?}", e);
                {
                    let mut state = self.state.write().await;
                    Self::try_clear_connection(&mut state, generation);
                }
                let (retry_result, _) = self.call_get_friends(payload).await;
                retry_result.map_err(|e| anyhow!("Failed to get friends: {:?}", e))
            }
        }
    }

    /// Internal helper to make the get_friends RPC call
    /// Returns (result, connection_generation) so caller can do generation-aware retry
    async fn call_get_friends(
        &self,
        payload: GetFriendsPayload,
    ) -> (
        Result<PaginatedFriendsProfilesResponse, ClientResultError>,
        u64,
    ) {
        if let Err(e) = self.ensure_connected().await {
            tracing::error!("call_get_friends: connection failed: {:?}", e);
            return (
                Err(ClientResultError::Client(ClientError::TransportError)),
                0,
            );
        }

        let state = self.state.read().await;
        let generation = state.connection_generation;

        let service = match state.connection.as_ref() {
            Some(conn) => &conn.service,
            None => {
                tracing::error!("call_get_friends: no connection after ensure_connected");
                return (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                );
            }
        };

        let result = tokio::time::timeout(
            Duration::from_secs(RPC_TIMEOUT_SECS),
            service.get_friends(payload),
        )
        .await;

        match result {
            Ok(rpc_result) => (rpc_result, generation),
            Err(_) => {
                tracing::error!("call_get_friends: RPC timeout after {}s", RPC_TIMEOUT_SECS);
                (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                )
            }
        }
    }

    /// Get the list of mutual friends between the authenticated user and another user
    pub async fn get_mutual_friends(
        &self,
        user_address: String,
        pagination: Option<Pagination>,
    ) -> Result<PaginatedFriendsProfilesResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_mutual_friends(GetMutualFriendsPayload {
                user: Some(User {
                    address: user_address,
                }),
                pagination,
            })
            .await
            .map_err(|e| anyhow!("Failed to get mutual friends: {:?}", e))?;

        Ok(response)
    }

    /// Get the pending friendship requests for the authenticated user
    pub async fn get_pending_friendship_requests(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<PaginatedFriendshipRequestsResponse> {
        let payload = GetFriendshipRequestsPayload {
            pagination: pagination.clone(),
        };

        // First attempt
        let (result, generation) = self.call_get_pending_requests(payload.clone()).await;

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("get_pending_requests RPC failed, retrying: {:?}", e);
                {
                    let mut state = self.state.write().await;
                    Self::try_clear_connection(&mut state, generation);
                }
                let (retry_result, _) = self.call_get_pending_requests(payload).await;
                retry_result.map_err(|e| anyhow!("Failed to get pending requests: {:?}", e))
            }
        }
    }

    async fn call_get_pending_requests(
        &self,
        payload: GetFriendshipRequestsPayload,
    ) -> (
        Result<PaginatedFriendshipRequestsResponse, ClientResultError>,
        u64,
    ) {
        let mut state = self.state.write().await;
        let generation = state.connection_generation;
        let service = match self.ensure_connection(&mut state).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("call_get_pending_requests: connection failed: {:?}", e);
                return (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                );
            }
        };

        let result = tokio::time::timeout(
            Duration::from_secs(RPC_TIMEOUT_SECS),
            service.get_pending_friendship_requests(payload),
        )
        .await;

        match result {
            Ok(rpc_result) => (rpc_result, generation),
            Err(_) => {
                tracing::error!(
                    "call_get_pending_requests: RPC timeout after {}s",
                    RPC_TIMEOUT_SECS
                );
                (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                )
            }
        }
    }

    /// Get the sent friendship requests for the authenticated user
    pub async fn get_sent_friendship_requests(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<PaginatedFriendshipRequestsResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_sent_friendship_requests(GetFriendshipRequestsPayload { pagination })
            .await
            .map_err(|e| anyhow!("Failed to get sent requests: {:?}", e))?;

        Ok(response)
    }

    /// Send a friendship request to another user
    pub async fn send_friend_request(
        &self,
        user_address: String,
        message: Option<String>,
    ) -> Result<UpsertFriendshipResponse> {
        let payload = UpsertFriendshipPayload {
            action: Some(upsert_friendship_payload::Action::Request(
                upsert_friendship_payload::RequestPayload {
                    user: Some(User {
                        address: user_address.clone(),
                    }),
                    message: message.clone(),
                },
            )),
        };

        let (result, generation) = self.call_upsert_friendship(payload.clone()).await;

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("send_friend_request RPC failed, retrying: {:?}", e);
                {
                    let mut state = self.state.write().await;
                    Self::try_clear_connection(&mut state, generation);
                }
                let (retry_result, _) = self.call_upsert_friendship(payload).await;
                retry_result.map_err(|e| anyhow!("Failed to send friend request: {:?}", e))
            }
        }
    }

    /// Accept a friendship request from another user
    pub async fn accept_friend_request(
        &self,
        user_address: String,
    ) -> Result<UpsertFriendshipResponse> {
        let payload = UpsertFriendshipPayload {
            action: Some(upsert_friendship_payload::Action::Accept(
                upsert_friendship_payload::AcceptPayload {
                    user: Some(User {
                        address: user_address.clone(),
                    }),
                },
            )),
        };

        let (result, generation) = self.call_upsert_friendship(payload.clone()).await;

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("accept_friend_request RPC failed, retrying: {:?}", e);
                {
                    let mut state = self.state.write().await;
                    Self::try_clear_connection(&mut state, generation);
                }
                let (retry_result, _) = self.call_upsert_friendship(payload).await;
                retry_result.map_err(|e| anyhow!("Failed to accept friend request: {:?}", e))
            }
        }
    }

    /// Reject a friendship request from another user
    pub async fn reject_friend_request(
        &self,
        user_address: String,
    ) -> Result<UpsertFriendshipResponse> {
        let payload = UpsertFriendshipPayload {
            action: Some(upsert_friendship_payload::Action::Reject(
                upsert_friendship_payload::RejectPayload {
                    user: Some(User {
                        address: user_address.clone(),
                    }),
                },
            )),
        };

        let (result, generation) = self.call_upsert_friendship(payload.clone()).await;

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!("reject_friend_request RPC failed, retrying: {:?}", e);
                {
                    let mut state = self.state.write().await;
                    Self::try_clear_connection(&mut state, generation);
                }
                let (retry_result, _) = self.call_upsert_friendship(payload).await;
                retry_result.map_err(|e| anyhow!("Failed to reject friend request: {:?}", e))
            }
        }
    }

    /// Internal helper for upsert_friendship RPC calls
    async fn call_upsert_friendship(
        &self,
        payload: UpsertFriendshipPayload,
    ) -> (Result<UpsertFriendshipResponse, ClientResultError>, u64) {
        let mut state = self.state.write().await;
        let generation = state.connection_generation;
        let service = match self.ensure_connection(&mut state).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("call_upsert_friendship: connection failed: {:?}", e);
                return (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                );
            }
        };

        let result = tokio::time::timeout(
            Duration::from_secs(RPC_TIMEOUT_SECS),
            service.upsert_friendship(payload),
        )
        .await;

        match result {
            Ok(rpc_result) => (rpc_result, generation),
            Err(_) => {
                tracing::error!(
                    "call_upsert_friendship: RPC timeout after {}s",
                    RPC_TIMEOUT_SECS
                );
                (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                )
            }
        }
    }

    /// Cancel a sent friendship request
    pub async fn cancel_friend_request(
        &self,
        user_address: String,
    ) -> Result<UpsertFriendshipResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .upsert_friendship(UpsertFriendshipPayload {
                action: Some(upsert_friendship_payload::Action::Cancel(
                    upsert_friendship_payload::CancelPayload {
                        user: Some(User {
                            address: user_address,
                        }),
                    },
                )),
            })
            .await
            .map_err(|e| anyhow!("Failed to cancel friend request: {:?}", e))?;

        Ok(response)
    }

    /// Delete an existing friendship
    pub async fn delete_friendship(
        &self,
        user_address: String,
    ) -> Result<UpsertFriendshipResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .upsert_friendship(UpsertFriendshipPayload {
                action: Some(upsert_friendship_payload::Action::Delete(
                    upsert_friendship_payload::DeletePayload {
                        user: Some(User {
                            address: user_address,
                        }),
                    },
                )),
            })
            .await
            .map_err(|e| anyhow!("Failed to delete friendship: {:?}", e))?;

        Ok(response)
    }

    /// Get the friendship status with a specific user
    pub async fn get_friendship_status(
        &self,
        user_address: String,
    ) -> Result<GetFriendshipStatusResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_friendship_status(GetFriendshipStatusPayload {
                user: Some(User {
                    address: user_address,
                }),
            })
            .await
            .map_err(|e| anyhow!("Failed to get friendship status: {:?}", e))?;

        Ok(response)
    }

    /// Subscribe to friendship updates stream
    /// This is a STREAM (iterator) that pushes updates as they happen
    /// Returns a channel receiver for consuming updates
    pub async fn subscribe_to_friendship_updates(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<FriendshipUpdate>> {
        // First ensure we have a connection (this handles concurrent connection attempts)
        self.ensure_connected().await?;

        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let mut stream = service
            .subscribe_to_friendship_updates()
            .await
            .map_err(|e| {
                tracing::error!("Failed to subscribe to friendship updates: {:?}", e);
                anyhow!("Failed to subscribe to friendship updates: {:?}", e)
            })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                if tx.send(update).is_err() {
                    tracing::warn!("Friendship updates receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }

    /// Get the last received friendship updates from internal cache
    pub async fn get_cached_friendship_updates(&self) -> Vec<FriendshipUpdate> {
        self.state.read().await.last_friendship_updates.clone()
    }

    /// Subscribe to friend connectivity updates stream (ONLINE, OFFLINE, AWAY)
    /// Returns a channel receiver for consuming updates
    pub async fn subscribe_to_friend_connectivity_updates(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<FriendConnectivityUpdate>> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let mut stream = service
            .subscribe_to_friend_connectivity_updates()
            .await
            .map_err(|e| {
                tracing::error!("Failed to subscribe to connectivity updates: {:?}", e);
                anyhow!("Failed to subscribe to connectivity updates: {:?}", e)
            })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                if tx.send(update).is_err() {
                    tracing::warn!("Connectivity updates receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }

    // ========================================
    // Blocking Features
    // ========================================

    /// Block a user
    pub async fn block_user(&self, user_address: String) -> Result<BlockUserResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .block_user(BlockUserPayload {
                user: Some(User {
                    address: user_address,
                }),
            })
            .await
            .map_err(|e| anyhow!("Failed to block user: {:?}", e))?;

        Ok(response)
    }

    /// Unblock a user
    pub async fn unblock_user(&self, user_address: String) -> Result<UnblockUserResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .unblock_user(UnblockUserPayload {
                user: Some(User {
                    address: user_address,
                }),
            })
            .await
            .map_err(|e| anyhow!("Failed to unblock user: {:?}", e))?;

        Ok(response)
    }

    /// Get the list of blocked users
    pub async fn get_blocked_users(
        &self,
        pagination: Option<Pagination>,
    ) -> Result<GetBlockedUsersResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_blocked_users(GetBlockedUsersPayload { pagination })
            .await
            .map_err(|e| anyhow!("Failed to get blocked users: {:?}", e))?;

        Ok(response)
    }

    /// Get blocking status (who you blocked and who blocked you)
    pub async fn get_blocking_status(&self) -> Result<GetBlockingStatusResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_blocking_status()
            .await
            .map_err(|e| anyhow!("Failed to get blocking status: {:?}", e))?;

        Ok(response)
    }

    /// Subscribe to block updates stream
    pub async fn subscribe_to_block_updates(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<BlockUpdate>> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let mut stream = service.subscribe_to_block_updates().await.map_err(|e| {
            tracing::error!("Failed to subscribe to block updates: {:?}", e);
            anyhow!("Failed to subscribe to block updates: {:?}", e)
        })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                if tx.send(update).is_err() {
                    tracing::warn!("Block updates receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }

    // ========================================
    // Social Settings
    // ========================================

    /// Get social settings for the authenticated user
    pub async fn get_social_settings(&self) -> Result<GetSocialSettingsResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_social_settings()
            .await
            .map_err(|e| anyhow!("Failed to get social settings: {:?}", e))?;

        Ok(response)
    }

    /// Update social settings for the authenticated user
    pub async fn upsert_social_settings(
        &self,
        private_messages_privacy: Option<i32>,
        blocked_users_messages_visibility: Option<i32>,
    ) -> Result<UpsertSocialSettingsResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .upsert_social_settings(UpsertSocialSettingsPayload {
                private_messages_privacy,
                blocked_users_messages_visibility,
            })
            .await
            .map_err(|e| anyhow!("Failed to upsert social settings: {:?}", e))?;

        Ok(response)
    }

    /// Get private messages settings for specific users
    pub async fn get_private_messages_settings(
        &self,
        user_addresses: Vec<String>,
    ) -> Result<GetPrivateMessagesSettingsResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let users = user_addresses
            .into_iter()
            .map(|address| User { address })
            .collect();

        let response = service
            .get_private_messages_settings(GetPrivateMessagesSettingsPayload { user: users })
            .await
            .map_err(|e| anyhow!("Failed to get private messages settings: {:?}", e))?;

        Ok(response)
    }

    // ========================================
    // Community Connectivity
    // ========================================

    /// Subscribe to community member connectivity updates
    pub async fn subscribe_to_community_member_connectivity_updates(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<CommunityMemberConnectivityUpdate>> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let mut stream = service
            .subscribe_to_community_member_connectivity_updates()
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to subscribe to community member connectivity updates: {:?}",
                    e
                );
                anyhow!(
                    "Failed to subscribe to community member connectivity updates: {:?}",
                    e
                )
            })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                if tx.send(update).is_err() {
                    tracing::warn!("Community member connectivity updates receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }

    // ========================================
    // Private Voice Chat
    // ========================================

    /// Start a private voice chat with another user
    pub async fn start_private_voice_chat(
        &self,
        callee_address: String,
    ) -> Result<StartPrivateVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .start_private_voice_chat(StartPrivateVoiceChatPayload {
                callee: Some(User {
                    address: callee_address,
                }),
            })
            .await
            .map_err(|e| anyhow!("Failed to start private voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Accept a private voice chat
    pub async fn accept_private_voice_chat(
        &self,
        call_id: String,
    ) -> Result<AcceptPrivateVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .accept_private_voice_chat(AcceptPrivateVoiceChatPayload { call_id })
            .await
            .map_err(|e| anyhow!("Failed to accept private voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Reject a private voice chat
    pub async fn reject_private_voice_chat(
        &self,
        call_id: String,
    ) -> Result<RejectPrivateVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .reject_private_voice_chat(RejectPrivateVoiceChatPayload { call_id })
            .await
            .map_err(|e| anyhow!("Failed to reject private voice chat: {:?}", e))?;

        Ok(response)
    }

    /// End a private voice chat
    pub async fn end_private_voice_chat(
        &self,
        call_id: String,
    ) -> Result<EndPrivateVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .end_private_voice_chat(EndPrivateVoiceChatPayload { call_id })
            .await
            .map_err(|e| anyhow!("Failed to end private voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Get incoming private voice chat request
    pub async fn get_incoming_private_voice_chat_request(
        &self,
    ) -> Result<GetIncomingPrivateVoiceChatRequestResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_incoming_private_voice_chat_request()
            .await
            .map_err(|e| anyhow!("Failed to get incoming private voice chat request: {:?}", e))?;

        Ok(response)
    }

    /// Subscribe to private voice chat updates
    pub async fn subscribe_to_private_voice_chat_updates(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<PrivateVoiceChatUpdate>> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let mut stream = service
            .subscribe_to_private_voice_chat_updates()
            .await
            .map_err(|e| {
                tracing::error!("Failed to subscribe to private voice chat updates: {:?}", e);
                anyhow!("Failed to subscribe to private voice chat updates: {:?}", e)
            })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                if tx.send(update).is_err() {
                    tracing::warn!("Private voice chat updates receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }

    // ========================================
    // Community Voice Chat
    // ========================================

    /// Start a community voice chat (moderator/owner only)
    pub async fn start_community_voice_chat(
        &self,
        community_id: String,
    ) -> Result<StartCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .start_community_voice_chat(StartCommunityVoiceChatPayload { community_id })
            .await
            .map_err(|e| anyhow!("Failed to start community voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Join a community voice chat
    pub async fn join_community_voice_chat(
        &self,
        community_id: String,
    ) -> Result<JoinCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .join_community_voice_chat(JoinCommunityVoiceChatPayload { community_id })
            .await
            .map_err(|e| anyhow!("Failed to join community voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Request to speak in community voice chat
    pub async fn request_to_speak_in_community_voice_chat(
        &self,
        community_id: String,
        is_raising_hand: bool,
    ) -> Result<RequestToSpeakInCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .request_to_speak_in_community_voice_chat(
                RequestToSpeakInCommunityVoiceChatPayload {
                    community_id,
                    is_raising_hand,
                },
            )
            .await
            .map_err(|e| anyhow!("Failed to request to speak in community voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Promote speaker in community voice chat (moderator only)
    pub async fn promote_speaker_in_community_voice_chat(
        &self,
        community_id: String,
        user_address: String,
    ) -> Result<PromoteSpeakerInCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .promote_speaker_in_community_voice_chat(PromoteSpeakerInCommunityVoiceChatPayload {
                community_id,
                user_address,
            })
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to promote speaker in community voice chat: {:?}",
                    e
                )
            })?;

        Ok(response)
    }

    /// Demote speaker in community voice chat (moderator only)
    pub async fn demote_speaker_in_community_voice_chat(
        &self,
        community_id: String,
        user_address: String,
    ) -> Result<DemoteSpeakerInCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .demote_speaker_in_community_voice_chat(DemoteSpeakerInCommunityVoiceChatPayload {
                community_id,
                user_address,
            })
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to demote speaker in community voice chat: {:?}",
                    e
                )
            })?;

        Ok(response)
    }

    /// Kick player from community voice chat (moderator only)
    pub async fn kick_player_from_community_voice_chat(
        &self,
        community_id: String,
        user_address: String,
    ) -> Result<KickPlayerFromCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .kick_player_from_community_voice_chat(KickPlayerFromCommunityVoiceChatPayload {
                community_id,
                user_address,
            })
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to kick player from community voice chat: {:?}",
                    e
                )
            })?;

        Ok(response)
    }

    /// Reject speak request in community voice chat (moderator only)
    pub async fn reject_speak_request_in_community_voice_chat(
        &self,
        community_id: String,
        user_address: String,
    ) -> Result<RejectSpeakRequestInCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .reject_speak_request_in_community_voice_chat(
                RejectSpeakRequestInCommunityVoiceChatPayload {
                    community_id,
                    user_address,
                },
            )
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to reject speak request in community voice chat: {:?}",
                    e
                )
            })?;

        Ok(response)
    }

    /// Mute or unmute a speaker in community voice chat (moderator only)
    pub async fn mute_speaker_from_community_voice_chat(
        &self,
        community_id: String,
        user_address: String,
        muted: bool,
    ) -> Result<MuteSpeakerFromCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .mute_speaker_from_community_voice_chat(MuteSpeakerFromCommunityVoiceChatPayload {
                community_id,
                user_address,
                muted,
            })
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to mute speaker in community voice chat: {:?}",
                    e
                )
            })?;

        Ok(response)
    }

    /// End community voice chat (moderator/owner only)
    pub async fn end_community_voice_chat(
        &self,
        community_id: String,
    ) -> Result<EndCommunityVoiceChatResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .end_community_voice_chat(EndCommunityVoiceChatPayload { community_id })
            .await
            .map_err(|e| anyhow!("Failed to end community voice chat: {:?}", e))?;

        Ok(response)
    }

    /// Subscribe to community voice chat updates
    pub async fn subscribe_to_community_voice_chat_updates(
        &self,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<CommunityVoiceChatUpdate>> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let mut stream = service
            .subscribe_to_community_voice_chat_updates()
            .await
            .map_err(|e| {
                tracing::error!("Failed to subscribe to community voice chat updates: {:?}", e);
                anyhow!(
                    "Failed to subscribe to community voice chat updates: {:?}",
                    e
                )
            })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                if tx.send(update).is_err() {
                    tracing::warn!("Community voice chat updates receiver dropped");
                    break;
                }
            }
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_create_manager() {
        // Create a mock wallet for testing
        // This test doesn't require network connectivity

        // Note: We'd need a real EphemeralAuthChain to fully test,
        // but we can at least verify the basic structure compiles
        // For now, this serves as a compilation check

        // let wallet = Arc::new(mock_wallet());
        // let manager = SocialServiceManager::new(wallet);
        // assert_eq!(manager.connection_state().await, ConnectionState::Disconnected);

        // Test passes if it compiles
    }
}
