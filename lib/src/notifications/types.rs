use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single notification from the Decentraland notifications service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Unique identifier for the notification (UUID)
    pub id: String,

    /// Type of notification (e.g., "item_sold", "bid_accepted", "governance_announcement")
    #[serde(rename = "type")]
    pub notification_type: String,

    /// User's Ethereum address (lowercase with 0x prefix)
    pub address: String,

    /// Notification-specific data (title, description, image, link, etc.)
    pub metadata: HashMap<String, serde_json::Value>,

    /// Unix timestamp (milliseconds) when the notification was created - stored as string from API
    pub timestamp: String,

    /// Whether the notification has been read
    pub read: bool,
}

impl Notification {
    /// Get the notification title from metadata
    pub fn get_title(&self) -> Option<String> {
        self.metadata
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get the notification description from metadata
    pub fn get_description(&self) -> Option<String> {
        self.metadata
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get the notification image URL from metadata
    pub fn get_image_url(&self) -> Option<String> {
        self.metadata
            .get("image")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get the notification action link from metadata
    pub fn get_link(&self) -> Option<String> {
        self.metadata
            .get("link")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

/// Request body for marking notifications as read
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadRequest {
    #[serde(rename = "notificationIds")]
    pub notification_ids: Vec<String>,
}

impl MarkReadRequest {
    pub fn new(notification_ids: Vec<String>) -> Self {
        Self { notification_ids }
    }
}

/// Response from marking notifications as read
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkReadResponse {
    /// Number of notifications that were updated
    pub updated: u32,
}
