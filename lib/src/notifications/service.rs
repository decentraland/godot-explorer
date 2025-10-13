use super::types::{MarkReadRequest, MarkReadResponse, Notification};
use crate::auth::{ephemeral_auth_chain::EphemeralAuthChain, wallet::sign_request};
use http::Uri;
use serde::Deserialize;

/// Internal wrapper for the API response format
#[derive(Debug, Deserialize)]
struct NotificationsResponse {
    notifications: Vec<Notification>,
}

/// Notifications service for interacting with Decentraland's notifications API
pub struct NotificationsService {
    pub(crate) base_url: String,
    client: reqwest::Client,
}

impl NotificationsService {
    /// Create a new notifications service with the given base URL
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Fetch notifications from the API
    ///
    /// # Arguments
    /// * `auth_chain` - User's ephemeral auth chain for authentication
    /// * `from_timestamp` - Optional Unix timestamp to fetch notifications from (inclusive)
    /// * `limit` - Optional limit on number of notifications to fetch (max 50)
    /// * `only_unread` - If true, only fetch unread notifications
    ///
    /// # Returns
    /// A Result containing a vector of notifications or an error message
    pub async fn get_notifications(
        &self,
        auth_chain: &EphemeralAuthChain,
        from_timestamp: Option<i64>,
        limit: Option<u32>,
        only_unread: Option<bool>,
    ) -> Result<Vec<Notification>, String> {
        tracing::info!("NotificationsService::get_notifications: Starting");

        // Build query parameters
        let mut query_params = vec![];

        if let Some(from) = from_timestamp {
            query_params.push(format!("from={}", from));
        }

        if let Some(limit_val) = limit {
            let clamped_limit = limit_val.min(50); // Max 50 per API spec
            query_params.push(format!("limit={}", clamped_limit));
        }

        if let Some(true) = only_unread {
            query_params.push("onlyUnread=true".to_string());
        }

        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };

        let url = format!("{}/notifications{}", self.base_url, query_string);
        tracing::info!("NotificationsService::get_notifications: URL = {}", url);

        let uri: Uri = url
            .parse()
            .map_err(|e| format!("Invalid URL: {}", e))?;

        // Create signed fetch headers
        tracing::info!("NotificationsService::get_notifications: Creating signed headers");
        let headers = sign_request(
            "GET",
            &uri,
            auth_chain,
            serde_json::Map::new(), // Empty metadata for GET request
        )
        .await;

        tracing::info!("NotificationsService::get_notifications: Got {} headers", headers.len());

        // Build request with auth headers
        let mut request_builder = self.client.get(&url);

        for (key, value) in headers {
            request_builder = request_builder.header(&key, &value);
        }

        // Send request
        tracing::info!("NotificationsService::get_notifications: Sending HTTP request");
        let response = request_builder
            .send()
            .await
            .map_err(|e| {
                tracing::error!("NotificationsService::get_notifications: Network error: {}", e);
                format!("Network error: {}", e)
            })?;

        tracing::info!("NotificationsService::get_notifications: Got response");

        // Check status code
        let status = response.status();
        tracing::info!("NotificationsService::get_notifications: Status code: {}", status.as_u16());

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("NotificationsService::get_notifications: API error: {}", error_body);
            return Err(format!(
                "API error (status {}): {}",
                status.as_u16(),
                error_body
            ));
        }

        // Parse JSON response - first get as text to see what we're getting
        tracing::info!("NotificationsService::get_notifications: Parsing JSON");
        let response_text = response
            .text()
            .await
            .map_err(|e| {
                tracing::error!("NotificationsService::get_notifications: Failed to get response text: {}", e);
                format!("Failed to read response: {}", e)
            })?;

        tracing::info!("NotificationsService::get_notifications: Response body (first 500 chars): {}",
            if response_text.len() > 500 { &response_text[..500] } else { &response_text });

        let response: NotificationsResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                tracing::error!("NotificationsService::get_notifications: JSON parse error: {}", e);
                format!("Failed to parse response: {}", e)
            })?;

        tracing::info!("NotificationsService::get_notifications: Success, got {} notifications", response.notifications.len());
        Ok(response.notifications)
    }

    /// Mark notifications as read
    ///
    /// # Arguments
    /// * `auth_chain` - User's ephemeral auth chain for authentication
    /// * `notification_ids` - List of notification IDs to mark as read
    ///
    /// # Returns
    /// A Result containing the number of notifications updated or an error message
    pub async fn mark_as_read(
        &self,
        auth_chain: &EphemeralAuthChain,
        notification_ids: Vec<String>,
    ) -> Result<MarkReadResponse, String> {
        if notification_ids.is_empty() {
            return Err("No notification IDs provided".to_string());
        }

        let url = format!("{}/notifications/read", self.base_url);
        let uri: Uri = url
            .parse()
            .map_err(|e| format!("Invalid URL: {}", e))?;

        // Build request body
        let request_body = MarkReadRequest::new(notification_ids);
        let body_json =
            serde_json::to_string(&request_body).map_err(|e| format!("JSON error: {}", e))?;

        // Create signed fetch headers with empty metadata
        let headers = sign_request(
            "PUT",
            &uri,
            auth_chain,
            serde_json::Map::new(), // Empty metadata for PUT request
        )
        .await;

        // Build request with auth headers
        let mut request_builder = self
            .client
            .put(&url)
            .header("Content-Type", "application/json")
            .body(body_json);

        for (key, value) in headers {
            request_builder = request_builder.header(&key, &value);
        }

        // Send request
        let response = request_builder
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "API error (status {}): {}",
                status.as_u16(),
                error_body
            ));
        }

        // Parse JSON response
        let mark_read_response: MarkReadResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(mark_read_response)
    }
}
