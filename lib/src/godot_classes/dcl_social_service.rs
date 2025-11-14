use godot::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::dcl::components::proto_components::social_service::v2::*;
use crate::godot_classes::promise::Promise;
use crate::scene_runner::tokio_runtime::TokioRuntime;
use crate::social::social_service_manager::SocialServiceManager;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclSocialService {
    manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for DclSocialService {
    fn init(base: Base<Node>) -> Self {
        Self {
            manager: Arc::new(RwLock::new(None)),
            base,
        }
    }
}

#[godot_api]
impl DclSocialService {
    /// Signal emitted when a new friendship request is received
    #[signal]
    pub fn friendship_request_received(address: GString, message: GString);

    /// Signal emitted when a friendship request is accepted
    #[signal]
    pub fn friendship_request_accepted(address: GString);

    /// Signal emitted when a friendship request is rejected
    #[signal]
    pub fn friendship_request_rejected(address: GString);

    /// Signal emitted when a friendship is deleted
    #[signal]
    pub fn friendship_deleted(address: GString);

    /// Signal emitted when a friendship request is cancelled
    #[signal]
    pub fn friendship_request_cancelled(address: GString);

    /// Initialize the service with DclPlayerIdentity
    #[func]
    pub fn initialize_from_player_identity(&mut self, player_identity: Gd<crate::auth::dcl_player_identity::DclPlayerIdentity>) {
        godot_print!("[RUST] initialize_from_player_identity called");
        let wallet_option = player_identity.bind().try_get_ephemeral_auth_chain();

        let Some(wallet) = wallet_option else {
            godot_error!("DclSocialService: Player identity has no wallet - cannot initialize");
            return;
        };

        godot_print!("[RUST] Wallet obtained, creating manager");
        let manager = Arc::new(SocialServiceManager::new(Arc::new(wallet)));

        // Set the manager synchronously to avoid race conditions
        // Note: We use blocking_lock here since we're in a sync context
        if let Ok(mut guard) = self.manager.try_write() {
            *guard = Some(manager);
            godot_print!("[RUST] ✅ DclSocialService initialized successfully");
        } else {
            godot_error!("[RUST] ❌ DclSocialService: Failed to acquire write lock during initialization");
        }
    }

    /// Initialize with a direct wallet reference (for internal use)
    pub fn initialize_with_wallet(&mut self, wallet: Arc<EphemeralAuthChain>) {
        let manager = Arc::new(SocialServiceManager::new(wallet));

        // Set the manager synchronously to avoid race conditions
        if let Ok(mut guard) = self.manager.try_write() {
            *guard = Some(manager);
            tracing::info!("DclSocialService initialized with wallet");
        } else {
            tracing::error!("DclSocialService: Failed to acquire write lock during wallet initialization");
        }
    }

    /// Get the list of friends
    #[func]
    pub fn get_friends(&mut self, limit: i32, offset: i32, status: i32) -> Gd<Promise> {
        godot_print!("[RUST] get_friends called with limit={}, offset={}, status={}", limit, offset, status);
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();

        godot_print!("[RUST] Spawning async task for get_friends");
        TokioRuntime::spawn(async move {
            godot_print!("[RUST] Async task started for get_friends");
            let result = Self::async_get_friends(manager, limit, offset, status).await;
            godot_print!("[RUST] async_get_friends completed, resolving promise");
            Self::resolve_friends_promise(get_promise, result);
        });

        godot_print!("[RUST] Returning promise from get_friends");
        promise
    }

    /// Get mutual friends with another user
    #[func]
    pub fn get_mutual_friends(&mut self, user_address: GString, limit: i32, offset: i32) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let user_address = user_address.to_string();

        TokioRuntime::spawn(async move {
            let result = Self::async_get_mutual_friends(manager, user_address, limit, offset).await;
            Self::resolve_friends_promise(get_promise, result);
        });

        promise
    }

    /// Get pending friendship requests
    #[func]
    pub fn get_pending_requests(&mut self, limit: i32, offset: i32) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();

        TokioRuntime::spawn(async move {
            let result = Self::async_get_pending_requests(manager, limit, offset).await;
            Self::resolve_requests_promise(get_promise, result);
        });

        promise
    }

    /// Get sent friendship requests
    #[func]
    pub fn get_sent_requests(&mut self, limit: i32, offset: i32) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();

        TokioRuntime::spawn(async move {
            let result = Self::async_get_sent_requests(manager, limit, offset).await;
            Self::resolve_requests_promise(get_promise, result);
        });

        promise
    }

    /// Send a friend request
    #[func]
    pub fn send_friend_request(&mut self, address: GString, message: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let address = address.to_string();
        let message = if message.is_empty() { None } else { Some(message.to_string()) };

        TokioRuntime::spawn(async move {
            let result = Self::async_send_friend_request(manager, address, message).await;
            Self::resolve_simple_promise(get_promise, result);
        });

        promise
    }

    /// Accept a friend request
    #[func]
    pub fn accept_friend_request(&mut self, address: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let address = address.to_string();

        TokioRuntime::spawn(async move {
            let result = Self::async_accept_friend_request(manager, address).await;
            Self::resolve_simple_promise(get_promise, result);
        });

        promise
    }

    /// Reject a friend request
    #[func]
    pub fn reject_friend_request(&mut self, address: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let address = address.to_string();

        TokioRuntime::spawn(async move {
            let result = Self::async_reject_friend_request(manager, address).await;
            Self::resolve_simple_promise(get_promise, result);
        });

        promise
    }

    /// Cancel a sent friend request
    #[func]
    pub fn cancel_friend_request(&mut self, address: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let address = address.to_string();

        TokioRuntime::spawn(async move {
            let result = Self::async_cancel_friend_request(manager, address).await;
            Self::resolve_simple_promise(get_promise, result);
        });

        promise
    }

    /// Delete a friendship
    #[func]
    pub fn delete_friendship(&mut self, address: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let address = address.to_string();

        TokioRuntime::spawn(async move {
            let result = Self::async_delete_friendship(manager, address).await;
            Self::resolve_simple_promise(get_promise, result);
        });

        promise
    }

    /// Get friendship status with a user
    #[func]
    pub fn get_friendship_status(&mut self, address: GString) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let address = address.to_string();

        TokioRuntime::spawn(async move {
            let result = Self::async_get_friendship_status(manager, address).await;
            Self::resolve_status_promise(get_promise, result);
        });

        promise
    }

    /// Subscribe to friendship updates (real-time streaming)
    #[func]
    pub fn subscribe_to_updates(&mut self) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let manager = self.manager.clone();
        let instance_id = self.base().instance_id();

        TokioRuntime::spawn(async move {
            let result = Self::async_subscribe_to_updates(manager, instance_id).await;
            Self::resolve_simple_promise(get_promise, result);
        });

        promise
    }
}

// Private async helper methods
impl DclSocialService {
    async fn async_get_friends(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        limit: i32,
        offset: i32,
        status: i32,
    ) -> Result<Vec<String>, String> {
        godot_print!("[RUST] async_get_friends: Acquiring manager lock...");
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or_else(|| {
            godot_error!("[RUST] async_get_friends: Social service not initialized!");
            "Social service not initialized".to_string()
        })?;

        godot_print!("[RUST] async_get_friends: Manager lock acquired, calling API...");
        let pagination = if limit > 0 { Some(Pagination { limit, offset }) } else { None };
        let status_filter = if status >= 0 { Some(status) } else { None };

        let response = mgr.get_friends(pagination, status_filter)
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to get friends: {}", e);
                godot_error!("[RUST] async_get_friends: {}", error_msg);
                error_msg
            })?;

        let friends: Vec<String> = response.users.into_iter()
            .map(|user| user.address)
            .collect();

        godot_print!("[RUST] async_get_friends: ✅ Successfully fetched {} friends", friends.len());
        Ok(friends)
    }

    async fn async_get_mutual_friends(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        user_address: String,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<String>, String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        let pagination = if limit > 0 { Some(Pagination { limit, offset }) } else { None };

        let response = mgr.get_mutual_friends(user_address, pagination)
            .await
            .map_err(|e| format!("Failed to get mutual friends: {}", e))?;

        let friends: Vec<String> = response.users.into_iter()
            .map(|user| user.address)
            .collect();

        Ok(friends)
    }

    async fn async_get_pending_requests(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<(String, String, i64)>, String> {
        tracing::debug!("async_get_pending_requests: Acquiring manager lock...");
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or_else(|| {
            tracing::error!("async_get_pending_requests: Social service not initialized");
            "Social service not initialized".to_string()
        })?;

        tracing::debug!("async_get_pending_requests: Calling API...");
        let pagination = if limit > 0 { Some(Pagination { limit, offset }) } else { None };

        let response = mgr.get_pending_friendship_requests(pagination)
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to get pending requests: {}", e);
                tracing::error!("async_get_pending_requests: {}", error_msg);
                error_msg
            })?;

        let requests = Self::extract_friendship_requests(response);
        tracing::info!("async_get_pending_requests: Successfully fetched {} requests", requests.len());
        Ok(requests)
    }

    async fn async_get_sent_requests(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<(String, String, i64)>, String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        let pagination = if limit > 0 { Some(Pagination { limit, offset }) } else { None };

        let response = mgr.get_sent_friendship_requests(pagination)
            .await
            .map_err(|e| format!("Failed to get sent requests: {}", e))?;

        let requests = Self::extract_friendship_requests(response);
        Ok(requests)
    }

    async fn async_send_friend_request(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        address: String,
        message: Option<String>,
    ) -> Result<(), String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        mgr.send_friend_request(address, message)
            .await
            .map_err(|e| format!("Failed to send friend request: {}", e))?;

        Ok(())
    }

    async fn async_accept_friend_request(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        address: String,
    ) -> Result<(), String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        mgr.accept_friend_request(address)
            .await
            .map_err(|e| format!("Failed to accept friend request: {}", e))?;

        Ok(())
    }

    async fn async_reject_friend_request(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        address: String,
    ) -> Result<(), String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        mgr.reject_friend_request(address)
            .await
            .map_err(|e| format!("Failed to reject friend request: {}", e))?;

        Ok(())
    }

    async fn async_cancel_friend_request(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        address: String,
    ) -> Result<(), String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        mgr.cancel_friend_request(address)
            .await
            .map_err(|e| format!("Failed to cancel friend request: {}", e))?;

        Ok(())
    }

    async fn async_delete_friendship(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        address: String,
    ) -> Result<(), String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        mgr.delete_friendship(address)
            .await
            .map_err(|e| format!("Failed to delete friendship: {}", e))?;

        Ok(())
    }

    async fn async_get_friendship_status(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        address: String,
    ) -> Result<(i32, String), String> {
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or("Social service not initialized")?;

        let response = mgr.get_friendship_status(address)
            .await
            .map_err(|e| format!("Failed to get friendship status: {}", e))?;

        let (status, message) = Self::extract_friendship_status(response);
        Ok((status, message))
    }

    async fn async_subscribe_to_updates(
        manager: Arc<RwLock<Option<Arc<SocialServiceManager>>>>,
        instance_id: InstanceId,
    ) -> Result<(), String> {
        tracing::debug!("async_subscribe_to_updates: Acquiring manager lock...");
        let manager_guard = manager.read().await;
        let mgr = manager_guard.as_ref().ok_or_else(|| {
            tracing::error!("async_subscribe_to_updates: Social service not initialized");
            "Social service not initialized".to_string()
        })?;

        tracing::debug!("async_subscribe_to_updates: Subscribing to friendship updates...");
        let mut rx = mgr.subscribe_to_friendship_updates()
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to subscribe to updates: {}", e);
                tracing::error!("async_subscribe_to_updates: {}", error_msg);
                error_msg
            })?;

        tracing::info!("async_subscribe_to_updates: Successfully subscribed, spawning listener task");
        // Spawn update listener task
        tokio::spawn(async move {
            Self::handle_friendship_updates(&mut rx, instance_id).await;
        });

        Ok(())
    }

    async fn handle_friendship_updates(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<FriendshipUpdate>,
        instance_id: InstanceId,
    ) {
        while let Some(update) = rx.recv().await {
            let Some(mut node) = Gd::<DclSocialService>::try_from_instance_id(instance_id).ok() else {
                tracing::warn!("DclSocialService node dropped, stopping update listener");
                break;
            };

            Self::emit_friendship_update_signal(&mut node, update);
        }
    }

    fn emit_friendship_update_signal(node: &mut Gd<DclSocialService>, update: FriendshipUpdate) {
        match update.update {
            Some(friendship_update::Update::Request(req)) => {
                if let Some(user) = req.user {
                    let message = req.message.unwrap_or_default();
                    node.emit_signal(
                        "friendship_request_received".into(),
                        &[user.address.to_variant(), message.to_variant()],
                    );
                }
            }
            Some(friendship_update::Update::Accept(accept)) => {
                if let Some(user) = accept.user {
                    node.emit_signal("friendship_request_accepted".into(), &[user.address.to_variant()]);
                }
            }
            Some(friendship_update::Update::Reject(reject)) => {
                if let Some(user) = reject.user {
                    node.emit_signal("friendship_request_rejected".into(), &[user.address.to_variant()]);
                }
            }
            Some(friendship_update::Update::Delete(delete)) => {
                if let Some(user) = delete.user {
                    node.emit_signal("friendship_deleted".into(), &[user.address.to_variant()]);
                }
            }
            Some(friendship_update::Update::Cancel(cancel)) => {
                if let Some(user) = cancel.user {
                    node.emit_signal("friendship_request_cancelled".into(), &[user.address.to_variant()]);
                }
            }
            None => {}
        }
    }

    fn extract_friendship_requests(response: PaginatedFriendshipRequestsResponse) -> Vec<(String, String, i64)> {
        let Some(paginated_friendship_requests_response::Response::Requests(requests)) = response.response else {
            return Vec::new();
        };

        requests.requests.into_iter()
            .filter_map(|req| {
                let address = req.user?.address;
                let message = req.message.unwrap_or_default();
                let created_at = req.created_at;
                Some((address, message, created_at))
            })
            .collect()
    }

    fn extract_friendship_status(response: GetFriendshipStatusResponse) -> (i32, String) {
        match response.response {
            Some(get_friendship_status_response::Response::Accepted(ok)) => {
                (ok.status, ok.message.unwrap_or_default())
            }
            _ => (-1, "Unknown".to_string()),
        }
    }

    fn resolve_friends_promise(
        get_promise: impl Fn() -> Option<Gd<Promise>>,
        result: Result<Vec<String>, String>,
    ) {
        godot_print!("[RUST] resolve_friends_promise: Getting promise...");
        let Some(mut promise) = get_promise() else {
            godot_warn!("[RUST] resolve_friends_promise: Promise was dropped before resolution");
            return;
        };

        match result {
            Ok(friends) => {
                godot_print!("[RUST] resolve_friends_promise: Resolving with {} friends", friends.len());
                let mut array = Array::new();
                for friend in &friends {
                    array.push(friend.to_variant());
                }
                godot_print!("[RUST] resolve_friends_promise: Calling resolve_with_data");
                promise.bind_mut().resolve_with_data(array.to_variant());
                godot_print!("[RUST] resolve_friends_promise: ✅ Promise resolved!");
            }
            Err(e) => {
                godot_error!("[RUST] resolve_friends_promise: ❌ Rejecting with error: {}", e);
                promise.bind_mut().reject(e.into());
            }
        }
    }

    fn resolve_requests_promise(
        get_promise: impl Fn() -> Option<Gd<Promise>>,
        result: Result<Vec<(String, String, i64)>, String>,
    ) {
        let Some(mut promise) = get_promise() else {
            tracing::warn!("resolve_requests_promise: Promise was dropped before resolution");
            return;
        };

        match result {
            Ok(requests) => {
                tracing::debug!("resolve_requests_promise: Resolving with {} requests", requests.len());
                let mut array = Array::new();
                for (address, message, created_at) in requests {
                    let mut dict = Dictionary::new();
                    dict.set("address", address);
                    dict.set("message", message);
                    dict.set("created_at", created_at);
                    array.push(dict.to_variant());
                }
                promise.bind_mut().resolve_with_data(array.to_variant());
            }
            Err(e) => {
                tracing::error!("resolve_requests_promise: Rejecting with error: {}", e);
                promise.bind_mut().reject(e.into());
            }
        }
    }

    fn resolve_status_promise(
        get_promise: impl Fn() -> Option<Gd<Promise>>,
        result: Result<(i32, String), String>,
    ) {
        let Some(mut promise) = get_promise() else { return };

        match result {
            Ok((status, message)) => {
                let mut dict = Dictionary::new();
                dict.set("status", status);
                dict.set("message", message);
                promise.bind_mut().resolve_with_data(dict.to_variant());
            }
            Err(e) => {
                promise.bind_mut().reject(e.into());
            }
        }
    }

    fn resolve_simple_promise(
        get_promise: impl Fn() -> Option<Gd<Promise>>,
        result: Result<(), String>,
    ) {
        let Some(mut promise) = get_promise() else {
            tracing::warn!("resolve_simple_promise: Promise was dropped before resolution");
            return;
        };

        match result {
            Ok(()) => {
                tracing::debug!("resolve_simple_promise: Resolving successfully");
                promise.bind_mut().resolve();
            }
            Err(e) => {
                tracing::error!("resolve_simple_promise: Rejecting with error: {}", e);
                promise.bind_mut().reject(e.into());
            }
        }
    }
}
