//! Notifications module for integrating with Decentraland's notifications service.
//!
//! This module provides:
//! - Data types for notifications (Notification, MarkReadRequest, MarkReadResponse)
//! - API client for fetching and managing notifications (NotificationsService)
//! - GDExtension interface for GDScript integration (NotificationsManager)

pub mod manager;
pub mod service;
pub mod types;

pub use manager::NotificationsManager;
pub use service::NotificationsService;
pub use types::{MarkReadRequest, MarkReadResponse, Notification};
