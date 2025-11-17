use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{anyhow, Result};
use dcl_rpc::client::RpcClient;
use dcl_rpc::transports::{Transport, web_sockets::{tungstenite::WebSocketClient, WebSocketTransport}};

#[allow(unused_imports)]
use futures_util::StreamExt;

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::dcl::components::proto_components::social_service::v2::*;
use crate::social::friends::build_auth_chain;

const SOCIAL_SERVICE_URL: &str = "wss://rpc-social-service-ea.decentraland.org";

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
}

impl SocialServiceState {
    fn new() -> Self {
        Self {
            connection_state: ConnectionState::Disconnected,
            last_friendship_updates: Vec::new(),
            connection: None,
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
    tracing::info!("Connecting to Social Service: {}", SOCIAL_SERVICE_URL);
    // Establish WebSocket connection
    let ws_connection = WebSocketClient::connect(SOCIAL_SERVICE_URL)
        .await
        .map_err(|e| {
            tracing::error!("Failed to connect to Social Service: {:?}", e);
            anyhow!("Failed to connect to Social Service: {:?}", e)
        })?;

    tracing::debug!("WebSocket connected, creating transport");
    let transport = WebSocketTransport::new(ws_connection);

    tracing::debug!("Building auth chain");
    // Build and send auth chain
    let auth_chain_message = build_auth_chain(wallet).await?;
    tracing::debug!("Sending auth chain");
    transport
        .send(auth_chain_message.as_bytes().to_vec())
        .await
        .map_err(|e| {
            tracing::error!("Failed to send auth chain: {:?}", e);
            anyhow!("Failed to send auth chain: {:?}", e)
        })?;

    tracing::debug!("Auth chain sent, creating RPC client");
    // Create RPC client
    let mut rpc_client = RpcClient::new(transport)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create RPC client: {:?}", e);
            anyhow!("Failed to create RPC client: {:?}", e)
        })?;

    tracing::debug!("RPC client created, creating port");
    // Create port and load service module
    let port = rpc_client
        .create_port("SocialService")
        .await
        .map_err(|e| {
            tracing::error!("Failed to create port: {:?}", e);
            anyhow!("Failed to create port: {:?}", e)
        })?;

    tracing::debug!("Port created, loading module");
    let service = port.load_module::<SocialServiceClient<_>>("SocialService")
        .await
        .map_err(|e| {
            tracing::error!("Failed to load module: {:?}", e);
            anyhow!("Failed to load module: {:?}", e)
        })?;

    tracing::info!("Social Service client ready");

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

    /// Get the current connection state
    pub async fn connection_state(&self) -> ConnectionState {
        self.state.read().await.connection_state
    }

    /// Get or create a connection, returning a reference to the service client
    async fn ensure_connection<'a>(&'a self, state: &'a mut SocialServiceState) -> Result<&'a SocialServiceClient<SocialTransport>> {
        if state.connection.is_none() {
            tracing::debug!("No existing connection, creating new one");
            let connection = create_connection(&self.wallet).await?;
            state.connection = Some(connection);
        }
        Ok(&state.connection.as_ref().unwrap().service)
    }

    /// Get the list of friends for the authenticated user
    pub async fn get_friends(
        &self,
        pagination: Option<Pagination>,
        _status: Option<i32>,
    ) -> Result<PaginatedFriendsProfilesResponse> {
        tracing::debug!("get_friends called, ensuring connection");

        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        tracing::debug!("Connection ready, calling get_friends RPC");
        let response = service
            .get_friends(GetFriendsPayload { pagination })
            .await
            .map_err(|e| {
                tracing::error!("get_friends RPC failed: {:?}", e);
                anyhow!("Failed to get friends: {:?}", e)
            })?;

        tracing::debug!("get_friends RPC succeeded");
        Ok(response)
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
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .get_pending_friendship_requests(GetFriendshipRequestsPayload { pagination })
            .await
            .map_err(|e| anyhow!("Failed to get pending requests: {:?}", e))?;

        Ok(response)
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
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .upsert_friendship(UpsertFriendshipPayload {
                action: Some(upsert_friendship_payload::Action::Request(
                    upsert_friendship_payload::RequestPayload {
                        user: Some(User {
                            address: user_address,
                        }),
                        message,
                    },
                )),
            })
            .await
            .map_err(|e| anyhow!("Failed to send friend request: {:?}", e))?;

        Ok(response)
    }

    /// Accept a friendship request from another user
    pub async fn accept_friend_request(&self, user_address: String) -> Result<UpsertFriendshipResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .upsert_friendship(UpsertFriendshipPayload {
                action: Some(upsert_friendship_payload::Action::Accept(
                    upsert_friendship_payload::AcceptPayload {
                        user: Some(User {
                            address: user_address,
                        }),
                    },
                )),
            })
            .await
            .map_err(|e| anyhow!("Failed to accept friend request: {:?}", e))?;

        Ok(response)
    }

    /// Reject a friendship request from another user
    pub async fn reject_friend_request(&self, user_address: String) -> Result<UpsertFriendshipResponse> {
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        let response = service
            .upsert_friendship(UpsertFriendshipPayload {
                action: Some(upsert_friendship_payload::Action::Reject(
                    upsert_friendship_payload::RejectPayload {
                        user: Some(User {
                            address: user_address,
                        }),
                    },
                )),
            })
            .await
            .map_err(|e| anyhow!("Failed to reject friend request: {:?}", e))?;

        Ok(response)
    }

    /// Cancel a sent friendship request
    pub async fn cancel_friend_request(&self, user_address: String) -> Result<UpsertFriendshipResponse> {
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
    pub async fn delete_friendship(&self, user_address: String) -> Result<UpsertFriendshipResponse> {
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
        let mut state = self.state.write().await;
        let service = self.ensure_connection(&mut state).await?;

        // Subscribe to the stream
        let mut stream = service
            .subscribe_to_friendship_updates()
            .await
            .map_err(|e| anyhow!("Failed to subscribe to friendship updates: {:?}", e))?;

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
}

#[cfg(test)]
mod tests {
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
