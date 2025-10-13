use godot::prelude::*;

use crate::{
    godot_classes::{dcl_global::DclGlobal, promise::Promise},
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::service::NotificationsService;

/// GDExtension class that exposes the notifications service to GDScript
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct NotificationsManager {
    service: NotificationsService,
}

#[godot_api]
impl IRefCounted for NotificationsManager {
    fn init(_base: Base<RefCounted>) -> Self {
        // Production URL by default
        let base_url = "https://notifications.decentraland.org".to_string();
        Self {
            service: NotificationsService::new(base_url),
        }
    }
}

#[godot_api]
impl NotificationsManager {
    /// Fetch notifications from the API
    ///
    /// # Arguments
    /// * `from_timestamp` - Unix timestamp to fetch from (use -1 for None)
    /// * `limit` - Max notifications to fetch (use -1 for None, max 50)
    /// * `only_unread` - If true, only fetch unread notifications
    ///
    /// # Returns
    /// A Promise that resolves with a JSON string containing the notifications array
    #[func]
    pub fn fetch_notifications(
        &self,
        from_timestamp: i64,
        limit: i32,
        only_unread: bool,
    ) -> Gd<Promise> {
        let service = self.service.clone();
        let (ret_promise, get_promise) = Promise::make_to_async();

        // Get the auth chain BEFORE spawning async task (Godot calls must be on main thread)
        tracing::info!("fetch_notifications: Getting auth chain on main thread");
        let auth_chain = {
            let player_identity = DclGlobal::singleton().bind().player_identity.clone();
            let player_identity_bind = player_identity.bind();
            player_identity_bind.get_ephemeral_auth_chain().cloned()
        };

        let Some(auth_chain) = auth_chain else {
            // No auth chain available (user not logged in)
            tracing::error!("fetch_notifications: No auth chain available");
            let mut promise = ret_promise.clone();
            promise.bind_mut().reject("User not authenticated".into());
            return ret_promise;
        };

        tracing::info!("fetch_notifications: Auth chain obtained, spawning async task");

        TokioRuntime::spawn(async move {
            tracing::info!("fetch_notifications: Starting async task");

            // Convert parameters (-1 means None)
            let from_ts = if from_timestamp >= 0 {
                Some(from_timestamp)
            } else {
                None
            };

            let limit_val = if limit > 0 {
                Some(limit as u32)
            } else {
                None
            };

            let only_unread_val = if only_unread { Some(true) } else { None };

            // Fetch notifications
            tracing::info!("fetch_notifications: Calling service");
            let result = service
                .get_notifications(&auth_chain, from_ts, limit_val, only_unread_val)
                .await;

            tracing::info!("fetch_notifications: Service call completed, result: {:?}", result.is_ok());

            // Resolve or reject the promise
            let Some(mut promise) = get_promise() else {
                tracing::error!("fetch_notifications: Promise no longer valid");
                return;
            };

            match result {
                Ok(notifications) => {
                    tracing::info!("fetch_notifications: Success, got {} notifications", notifications.len());
                    // Serialize to JSON string
                    match serde_json::to_string(&notifications) {
                        Ok(json_str) => {
                            tracing::info!("fetch_notifications: Resolving promise with JSON");
                            promise.bind_mut().resolve_with_data(json_str.to_variant());
                        }
                        Err(e) => {
                            tracing::error!("fetch_notifications: JSON serialization error: {}", e);
                            promise.bind_mut().reject(format!("JSON error: {}", e).into());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("fetch_notifications: Service error: {}", e);
                    promise.bind_mut().reject(e.into());
                }
            }
        });

        ret_promise
    }

    /// Mark notifications as read
    ///
    /// # Arguments
    /// * `notification_ids` - Array of notification ID strings to mark as read
    ///
    /// # Returns
    /// A Promise that resolves with the number of notifications updated
    #[func]
    pub fn mark_notifications_read(&self, notification_ids: PackedStringArray) -> Gd<Promise> {
        let service = self.service.clone();
        let ids: Vec<String> = notification_ids.to_vec().iter().map(|s| s.to_string()).collect();
        let (ret_promise, get_promise) = Promise::make_to_async();

        // Get the auth chain BEFORE spawning async task (Godot calls must be on main thread)
        let auth_chain = {
            let player_identity = DclGlobal::singleton().bind().player_identity.clone();
            let player_identity_bind = player_identity.bind();
            player_identity_bind.get_ephemeral_auth_chain().cloned()
        };

        let Some(auth_chain) = auth_chain else {
            // No auth chain available (user not logged in)
            let mut promise = ret_promise.clone();
            promise.bind_mut().reject("User not authenticated".into());
            return ret_promise;
        };

        TokioRuntime::spawn(async move {
            // Mark notifications as read
            let result = service.mark_as_read(&auth_chain, ids).await;

            // Resolve or reject the promise
            let Some(mut promise) = get_promise() else {
                return;
            };

            match result {
                Ok(response) => {
                    // Return the updated count as an integer
                    promise.bind_mut().resolve_with_data((response.updated as i32).to_variant());
                }
                Err(e) => promise.bind_mut().reject(e.into()),
            }
        });

        ret_promise
    }
}

// Implement Clone for NotificationsService so we can move it into async blocks
impl Clone for NotificationsService {
    fn clone(&self) -> Self {
        Self::new(self.base_url.clone())
    }
}
