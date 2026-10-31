//! JSON shapes of the social service HTTP API and of its `/v1/stream` WebSocket events.
//! See `docs/openapi.yaml` and `docs/websocket-stream-protocol.md` in social-service-ea.
//!
//! The API and the stream identify users by address only; [`FriendProfile`] and the types that
//! embed it are built on the client by resolving those addresses (see `profiles.rs`).

use serde::Deserialize;

/// A user plus the profile fields the UI shows. `name` falls back to the address when the
/// profile could not be resolved.
#[derive(Debug, Clone)]
pub struct FriendProfile {
    pub address: String,
    pub name: String,
    pub has_claimed_name: bool,
    pub profile_picture_url: String,
}

impl FriendProfile {
    pub fn without_profile(address: String) -> Self {
        Self {
            name: address.clone(),
            address,
            has_claimed_name: false,
            profile_picture_url: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FriendshipRequest {
    pub id: String,
    pub friend: FriendProfile,
    pub created_at: i64,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BlockedUser {
    pub profile: FriendProfile,
    pub blocked_at: Option<i64>,
}

/// `GET /v2/friends` and `GET /v2/users/:address/mutual-friends` entry.
#[derive(Debug, Clone, Deserialize)]
pub struct Friend {
    pub address: String,
}

/// `GET /v2/friendship-requests` entry and `POST /v1/friendship-requests` response.
/// `address` is the other user.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendshipRequestEntry {
    pub id: String,
    pub address: String,
    pub created_at: i64,
    #[serde(default)]
    pub message: Option<String>,
}

/// `GET /v2/blocks` entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedUserEntry {
    pub address: String,
    #[serde(default)]
    pub blocked_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Paginated<T> {
    pub results: Vec<T>,
    #[serde(default)]
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockingStatus {
    pub blocked_users: Vec<String>,
    pub blocked_by_users: Vec<String>,
}

/// Status codes match the `FriendshipStatus` values the UI already uses
/// (REQUEST_SENT=0, REQUEST_RECEIVED=1, CANCELED=2, ACCEPTED=3, REJECTED=4, DELETED=5,
/// BLOCKED=6, NONE=7, BLOCKED_BY=8).
#[derive(Debug, Clone, Deserialize)]
pub struct FriendshipStatusResponse {
    pub status: i32,
}

/// `friendship_updates` stream event. `address` is always the other user.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum FriendshipStreamEvent {
    #[serde(rename_all = "camelCase")]
    Request {
        id: String,
        address: String,
        created_at: i64,
        #[serde(default)]
        message: Option<String>,
    },
    Accept {
        address: String,
    },
    Reject {
        address: String,
    },
    Cancel {
        address: String,
    },
    Delete {
        address: String,
    },
    Block {
        address: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectivityStatus {
    Online,
    Offline,
    Away,
}

impl ConnectivityStatus {
    /// The numeric status the UI uses (ONLINE=0, OFFLINE=1, AWAY=2).
    pub fn as_i32(self) -> i32 {
        match self {
            ConnectivityStatus::Online => 0,
            ConnectivityStatus::Offline => 1,
            ConnectivityStatus::Away => 2,
        }
    }
}

/// `friend_connectivity_updates` stream event.
#[derive(Debug, Clone, Deserialize)]
pub struct FriendConnectivityStreamEvent {
    pub address: String,
    pub status: ConnectivityStatus,
}

/// `block_updates` stream event: `address` blocked (or unblocked) the user.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockStreamEvent {
    pub address: String,
    pub is_blocked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_friendship_request_event() {
        let event: FriendshipStreamEvent = serde_json::from_str(
            r#"{"action":"request","id":"r1","address":"0xabc","createdAt":1000,"message":"hi"}"#,
        )
        .unwrap();

        match event {
            FriendshipStreamEvent::Request {
                id,
                address,
                created_at,
                message,
            } => {
                assert_eq!(id, "r1");
                assert_eq!(address, "0xabc");
                assert_eq!(created_at, 1000);
                assert_eq!(message.as_deref(), Some("hi"));
            }
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn parses_the_other_friendship_actions() {
        for action in ["accept", "reject", "cancel", "delete", "block"] {
            let json = format!(r#"{{"action":"{action}","address":"0xabc"}}"#);
            assert!(serde_json::from_str::<FriendshipStreamEvent>(&json).is_ok());
        }
    }

    #[test]
    fn maps_connectivity_to_the_ui_status_codes() {
        let event: FriendConnectivityStreamEvent =
            serde_json::from_str(r#"{"address":"0xabc","status":"away"}"#).unwrap();

        assert_eq!(event.address, "0xabc");
        assert_eq!(event.status.as_i32(), 2);
    }

    #[test]
    fn parses_a_block_event() {
        let event: BlockStreamEvent =
            serde_json::from_str(r#"{"address":"0xabc","isBlocked":false}"#).unwrap();

        assert_eq!(event.address, "0xabc");
        assert!(!event.is_blocked);
    }
}
