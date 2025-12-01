use anyhow::{anyhow, Result};
use dcl_rpc::client::{ClientError, ClientResultError, RpcClient};
use dcl_rpc::transports::{
    web_sockets::{tungstenite::WebSocketClient, WebSocketTransport},
    Transport,
};
use godot::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[allow(unused_imports)]
use futures_util::StreamExt;

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::dcl::components::proto_components::social_service::v2::*;
use crate::social::friends::build_auth_chain;

const SOCIAL_SERVICE_URL: &str = "wss://rpc-social-service-ea.decentraland.org";
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
    godot_print!(
        "[SocialServiceManager] create_connection: Connecting to {}",
        SOCIAL_SERVICE_URL
    );

    // Establish WebSocket connection with timeout
    godot_print!("[SocialServiceManager] create_connection: Starting WebSocket connect...");
    let ws_connect_start = std::time::Instant::now();
    let ws_connection = tokio::time::timeout(
        Duration::from_secs(CONNECTION_TIMEOUT_SECS),
        WebSocketClient::connect(SOCIAL_SERVICE_URL),
    )
    .await
    .map_err(|_| {
        godot_error!(
            "[SocialServiceManager] create_connection: WebSocket timeout after {}s (elapsed: {:?})",
            CONNECTION_TIMEOUT_SECS,
            ws_connect_start.elapsed()
        );
        anyhow!(
            "Timeout connecting to Social Service after {}s",
            CONNECTION_TIMEOUT_SECS
        )
    })?
    .map_err(|e| {
        godot_error!(
            "[SocialServiceManager] create_connection: WebSocket connect failed after {:?}: {:?}",
            ws_connect_start.elapsed(),
            e
        );
        anyhow!("Failed to connect to Social Service: {:?}", e)
    })?;

    godot_print!(
        "[SocialServiceManager] create_connection: WebSocket connected in {:?}, creating transport",
        ws_connect_start.elapsed()
    );
    let transport = WebSocketTransport::new(ws_connection);

    godot_print!("[SocialServiceManager] create_connection: Building auth chain");
    // Build and send auth chain
    let auth_chain_message = build_auth_chain(wallet).await?;
    godot_print!("[SocialServiceManager] create_connection: Sending auth chain");
    transport
        .send(auth_chain_message.as_bytes().to_vec())
        .await
        .map_err(|e| {
            godot_error!(
                "[SocialServiceManager] create_connection: Failed to send auth chain: {:?}",
                e
            );
            anyhow!("Failed to send auth chain: {:?}", e)
        })?;

    godot_print!("[SocialServiceManager] create_connection: Auth chain sent, creating RPC client");
    // Create RPC client
    let mut rpc_client = RpcClient::new(transport).await.map_err(|e| {
        godot_error!(
            "[SocialServiceManager] create_connection: Failed to create RPC client: {:?}",
            e
        );
        anyhow!("Failed to create RPC client: {:?}", e)
    })?;

    godot_print!("[SocialServiceManager] create_connection: RPC client created, creating port");
    // Create port and load service module
    let port = rpc_client.create_port("SocialService").await.map_err(|e| {
        godot_error!(
            "[SocialServiceManager] create_connection: Failed to create port: {:?}",
            e
        );
        anyhow!("Failed to create port: {:?}", e)
    })?;

    godot_print!("[SocialServiceManager] create_connection: Port created, loading module");
    let service = port
        .load_module::<SocialServiceClient<_>>("SocialService")
        .await
        .map_err(|e| {
            godot_error!(
                "[SocialServiceManager] create_connection: Failed to load module: {:?}",
                e
            );
            anyhow!("Failed to load module: {:?}", e)
        })?;

    godot_print!("[SocialServiceManager] create_connection: Social Service client ready!");

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
                // Another task is already connecting, wait for it to complete
                godot_print!("[SocialServiceManager] ensure_connected: Connection in progress, waiting...");
                drop(state);
                // Wait for the other connection attempt to complete
                // Use CONNECTION_TIMEOUT_SECS + 5 to allow for the full connection attempt plus margin
                let max_wait_iterations = ((CONNECTION_TIMEOUT_SECS + 5) * 10) as usize;
                for i in 0..max_wait_iterations {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let state = self.state.read().await;
                    if state.connection.is_some() {
                        godot_print!("[SocialServiceManager] ensure_connected: Connection ready after waiting");
                        return Ok(());
                    }
                    if !state.connecting {
                        // Connection attempt finished but failed
                        if let Some(ref err) = state.last_error {
                            godot_print!(
                                "[SocialServiceManager] ensure_connected: Connection failed while waiting: {}",
                                err
                            );
                            return Err(anyhow!("Connection failed: {}", err));
                        }
                        // No error means we can try to connect ourselves
                        godot_print!("[SocialServiceManager] ensure_connected: Previous connection attempt ended, will try ourselves");
                        break;
                    }
                    // Log every 2 seconds
                    if i > 0 && i % 20 == 0 {
                        godot_print!(
                            "[SocialServiceManager] ensure_connected: Still waiting for connection... ({:.1}s)",
                            (i as f32) / 10.0
                        );
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
                // Lost the race, another task started connecting - wait for it
                godot_print!("[SocialServiceManager] ensure_connected: Another task started connecting, returning error");
                return Err(anyhow!("Connection already in progress"));
            }
            state.connecting = true;
            state.connection_state = ConnectionState::Connecting;
            state.last_error = None;
        }

        // Now we have exclusive rights to connect (connecting = true)
        godot_print!("[SocialServiceManager] ensure_connected: Creating new connection...");
        let result = create_connection(&self.wallet).await;

        // Store the result
        let mut state = self.state.write().await;
        state.connecting = false;

        match result {
            Ok(connection) => {
                godot_print!("[SocialServiceManager] ensure_connected: Connection created successfully");
                state.connection = Some(connection);
                state.connection_state = ConnectionState::Connected;
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                godot_error!(
                    "[SocialServiceManager] ensure_connected: Connection failed: {}",
                    error_msg
                );
                state.last_error = Some(error_msg.clone());
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
            godot_print!("[SocialServiceManager] ensure_connection: Using existing connection");
        } else if state.connecting {
            // Another task is connecting - drop the lock and use ensure_connected instead
            godot_print!("[SocialServiceManager] ensure_connection: Connection in progress, will wait");
            // We can't wait while holding the mutable borrow, so just return an error
            // The caller should retry after the connection is established
            return Err(anyhow!("Connection in progress, please retry"));
        } else {
            // No connection and not connecting - create one
            // This path is used by methods that don't call ensure_connected first
            state.connecting = true;
            godot_print!("[SocialServiceManager] ensure_connection: No existing connection, creating new one");
            match create_connection(&self.wallet).await {
                Ok(connection) => {
                    godot_print!("[SocialServiceManager] ensure_connection: Connection created successfully");
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
            tracing::info!(
                "Clearing stale social service connection (gen {})",
                expected_generation
            );
            state.connection = None;
            state.connection_generation += 1;
            true
        } else {
            tracing::debug!(
                "Connection already cleared by another caller (expected gen {}, current gen {})",
                expected_generation,
                state.connection_generation
            );
            false
        }
    }

    /// Get the list of friends for the authenticated user
    pub async fn get_friends(
        &self,
        pagination: Option<Pagination>,
        _status: Option<i32>,
    ) -> Result<PaginatedFriendsProfilesResponse> {
        godot_print!("[SocialServiceManager] get_friends called");

        let payload = GetFriendsPayload {
            pagination: pagination.clone(),
        };

        // First attempt - capture generation before call
        godot_print!("[SocialServiceManager] get_friends: calling RPC (attempt 1)");
        let (result, generation) = self.call_get_friends(payload.clone()).await;

        match result {
            Ok(response) => {
                godot_print!(
                    "[SocialServiceManager] get_friends: RPC succeeded, {} friends",
                    response.friends.len()
                );
                Ok(response)
            }
            Err(e) => {
                godot_print!(
                    "[SocialServiceManager] get_friends: RPC failed (gen {}), will retry: {:?}",
                    generation,
                    e
                );
                // Try to clear connection - only if we're the first to detect failure
                {
                    let mut state = self.state.write().await;
                    Self::try_clear_connection(&mut state, generation);
                }
                // Retry with fresh connection (ensure_connection will create new one)
                godot_print!("[SocialServiceManager] get_friends: calling RPC (attempt 2)");
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
        // First ensure we have a connection (this handles concurrent connection attempts)
        if let Err(e) = self.ensure_connected().await {
            godot_error!(
                "[SocialServiceManager] call_get_friends: ensure_connected failed: {:?}",
                e
            );
            return (
                Err(ClientResultError::Client(ClientError::TransportError)),
                0,
            );
        }

        godot_print!("[SocialServiceManager] call_get_friends: acquiring state lock");
        let state = self.state.read().await;
        let generation = state.connection_generation;

        let service = match state.connection.as_ref() {
            Some(conn) => &conn.service,
            None => {
                godot_error!("[SocialServiceManager] call_get_friends: no connection after ensure_connected");
                return (
                    Err(ClientResultError::Client(ClientError::TransportError)),
                    generation,
                );
            }
        };

        godot_print!("[SocialServiceManager] call_get_friends: calling RPC with {}s timeout", RPC_TIMEOUT_SECS);
        // Add timeout to RPC call to prevent hanging
        let result = tokio::time::timeout(
            Duration::from_secs(RPC_TIMEOUT_SECS),
            service.get_friends(payload),
        )
        .await;

        match result {
            Ok(rpc_result) => {
                godot_print!(
                    "[SocialServiceManager] call_get_friends: RPC completed, success={}",
                    rpc_result.is_ok()
                );
                (rpc_result, generation)
            }
            Err(_) => {
                godot_error!(
                    "[SocialServiceManager] call_get_friends: RPC timeout after {}s",
                    RPC_TIMEOUT_SECS
                );
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
                tracing::warn!(
                    "get_pending_friendship_requests RPC failed (gen {}), will retry: {:?}",
                    generation,
                    e
                );
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
                tracing::warn!(
                    "send_friend_request RPC failed (gen {}), will retry: {:?}",
                    generation,
                    e
                );
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
                tracing::warn!(
                    "accept_friend_request RPC failed (gen {}), will retry: {:?}",
                    generation,
                    e
                );
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
                tracing::warn!(
                    "reject_friend_request RPC failed (gen {}), will retry: {:?}",
                    generation,
                    e
                );
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
        godot_print!("[SocialServiceManager] subscribe_to_friendship_updates: ensuring connection");
        // First ensure we have a connection (this handles concurrent connection attempts)
        self.ensure_connected().await?;

        godot_print!("[SocialServiceManager] subscribe_to_friendship_updates: acquiring state lock");
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        godot_print!("[SocialServiceManager] subscribe_to_friendship_updates: calling RPC subscribe");
        // Subscribe to the stream
        let mut stream = service
            .subscribe_to_friendship_updates()
            .await
            .map_err(|e| {
                godot_error!(
                    "[SocialServiceManager] subscribe_to_friendship_updates: RPC failed: {:?}",
                    e
                );
                anyhow!("Failed to subscribe to friendship updates: {:?}", e)
            })?;

        godot_print!("[SocialServiceManager] subscribe_to_friendship_updates: stream created, spawning listener");
        // Create a channel to forward updates
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn a task to consume the stream and forward updates
        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                tracing::debug!("Received friendship update: {:?}", update);
                if tx.send(update).is_err() {
                    tracing::warn!("Failed to send friendship update, receiver dropped");
                    break;
                }
            }
            tracing::info!("Friendship updates stream ended");
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

        // Subscribe to the stream
        let mut stream = service
            .subscribe_to_friend_connectivity_updates()
            .await
            .map_err(|e| anyhow!("Failed to subscribe to connectivity updates: {:?}", e))?;

        // Create a channel to forward updates
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn a task to consume the stream and forward updates
        tokio::spawn(async move {
            while let Some(update) = stream.next().await {
                tracing::debug!("Received connectivity update: {:?}", update);
                if tx.send(update).is_err() {
                    tracing::warn!("Failed to send connectivity update, receiver dropped");
                    break;
                }
            }
            tracing::info!("Connectivity updates stream ended");
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
