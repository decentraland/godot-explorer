//! The notifications service: the user's feed and marking it read.

use serde_json::{json, Value};

use super::signed::{build_url, SignedClient, SignedError};
use crate::urls;

/// The service never returns more than this per page.
const MAX_LIMIT: u32 = 50;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NotificationSender {
    pub address: String,
    pub name: String,
    pub profile_image_url: String,
    pub has_claimed_name: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum NotificationKind {
    FriendshipRequest {
        sender: NotificationSender,
    },
    FriendshipAccepted {
        sender: NotificationSender,
    },
    /// An event the user follows started; `event_id` is the `id` of its jump link.
    EventStarted {
        title: String,
        image: String,
        link: String,
        event_id: Option<String>,
    },
    Reward {
        token_name: String,
        token_image: String,
        title: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Notification {
    pub id: String,
    /// Unix milliseconds.
    pub timestamp: i64,
    pub read: bool,
    pub kind: NotificationKind,
}

/// What `notifications` asks for. `limit` is clamped to 50; `None` leaves each to the service.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub from: Option<i64>,
    pub limit: Option<u32>,
    pub only_unread: bool,
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The service sends the timestamp as a number or as a numeric string.
fn timestamp_of(raw: &Value) -> i64 {
    match raw.get("timestamp") {
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f as i64))
            .unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn sender_of(metadata: &Value) -> NotificationSender {
    let sender = metadata.get("sender").unwrap_or(&Value::Null);
    NotificationSender {
        address: text(sender, "address"),
        name: text(sender, "name"),
        profile_image_url: text(sender, "profileImageUrl"),
        has_claimed_name: sender.get("hasClaimedName").and_then(Value::as_bool) == Some(true),
    }
}

fn event_id_of(link: &str) -> Option<String> {
    url::Url::parse(link)
        .ok()?
        .query_pairs()
        .find(|(key, _)| key == "id")
        .map(|(_, value)| value.into_owned())
        .filter(|id| !id.is_empty())
}

/// `None` for a kind without a screen: new server-side kinds are dropped, as the feed always did.
fn parse_kind(raw: &Value) -> Option<NotificationKind> {
    let metadata = raw.get("metadata").unwrap_or(&Value::Null);
    let kind = match text(raw, "type").as_str() {
        "social_service_friendship_request" => NotificationKind::FriendshipRequest {
            sender: sender_of(metadata),
        },
        "social_service_friendship_accepted" => NotificationKind::FriendshipAccepted {
            sender: sender_of(metadata),
        },
        "events_started" => {
            let link = text(metadata, "link");
            NotificationKind::EventStarted {
                title: text(metadata, "title"),
                image: text(metadata, "image"),
                event_id: event_id_of(&link),
                link,
            }
        }
        "reward_assignment" => NotificationKind::Reward {
            token_name: text(metadata, "tokenName"),
            token_image: text(metadata, "tokenImage"),
            title: text(metadata, "title"),
        },
        _ => return None,
    };
    Some(kind)
}

/// `None` for an entry without an id (the shell cannot mark it read) or of an unsupported kind.
fn parse_notification(raw: &Value) -> Option<Notification> {
    let id = match raw.get("id")? {
        Value::String(id) if !id.is_empty() => id.clone(),
        _ => return None,
    };
    Some(Notification {
        id,
        timestamp: timestamp_of(raw),
        read: raw.get("read").and_then(Value::as_bool) == Some(true),
        kind: parse_kind(raw)?,
    })
}

/// The user's notifications, newest first. Kinds without a screen are dropped, and so are friend
/// requests from senders `is_blocked` rejects.
pub async fn notifications(
    client: &SignedClient,
    query: Query,
    is_blocked: &(dyn Fn(&str) -> bool + Sync),
) -> Result<Vec<Notification>, SignedError> {
    let mut params = Vec::new();
    if let Some(from) = query.from.filter(|from| *from >= 0) {
        params.push(("from", from.to_string()));
    }
    if let Some(limit) = query.limit.filter(|limit| *limit > 0) {
        params.push(("limit", limit.min(MAX_LIMIT).to_string()));
    }
    if query.only_unread {
        params.push(("onlyUnread", "true".to_owned()));
    }
    let url = build_url(&urls::notifications_api(), &["notifications"], &params)?;
    let body = client.request_json("GET", &url, None).await?;
    let items = body
        .get("notifications")
        .and_then(Value::as_array)
        .ok_or_else(|| SignedError::Malformed("no notifications array".into()))?;
    let mut parsed: Vec<Notification> = items
        .iter()
        .filter_map(parse_notification)
        .filter(|notification| match &notification.kind {
            NotificationKind::FriendshipRequest { sender } => {
                sender.address.is_empty() || !is_blocked(&sender.address)
            }
            _ => true,
        })
        .collect();
    parsed.sort_by_key(|n| std::cmp::Reverse(n.timestamp));
    Ok(parsed)
}

/// Marks `ids` read and returns how many the service updated.
pub async fn mark_notifications_read(
    client: &SignedClient,
    ids: &[String],
) -> Result<u32, SignedError> {
    if ids.is_empty() {
        return Err(SignedError::InvalidArgument(
            "No notification IDs provided".into(),
        ));
    }
    let url = build_url(&urls::notifications_api(), &["notifications", "read"], &[])?;
    let body = client
        .request_json("PUT", &url, Some(&json!({ "notificationIds": ids })))
        .await?;
    body.get("updated")
        .and_then(Value::as_u64)
        .map(|n| n as u32)
        .ok_or_else(|| SignedError::Malformed("no updated count".into()))
}

#[cfg(test)]
mod tests {
    use super::super::signed::testing::*;
    use super::*;

    fn nobody(_: &str) -> bool {
        false
    }

    fn feed() -> String {
        json!({"notifications": [
            {"id": "a", "type": "events_started", "timestamp": "100", "read": false,
             "metadata": {"title": "Party", "image": "i.png",
                          "link": "https://decentraland.org/jump/events?id=e1"}},
            {"id": "b", "type": "social_service_friendship_request", "timestamp": 300,
             "metadata": {"sender": {"address": "0x1", "name": "Ann", "hasClaimedName": true,
                                     "profileImageUrl": "f.png"}}},
            {"id": "c", "type": "reward_assignment", "timestamp": 200, "read": true,
             "metadata": {"tokenName": "Hat", "tokenImage": "h.png", "title": "T"}},
            {"id": "d", "type": "item_sold", "timestamp": 400},
            {"type": "events_started"}
        ]})
        .to_string()
    }

    #[tokio::test]
    async fn the_feed_is_typed_filtered_and_newest_first() {
        let mock = Mock::ok(&feed());
        let list = notifications(&client(&mock), Query::default(), &nobody)
            .await
            .unwrap();
        let ids: Vec<_> = list.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["b", "c", "a"]);
        assert_eq!(
            list[0].kind,
            NotificationKind::FriendshipRequest {
                sender: NotificationSender {
                    address: "0x1".into(),
                    name: "Ann".into(),
                    profile_image_url: "f.png".into(),
                    has_claimed_name: true
                }
            }
        );
        assert!(list[1].read);
        assert_eq!(
            list[2].kind,
            NotificationKind::EventStarted {
                title: "Party".into(),
                image: "i.png".into(),
                link: "https://decentraland.org/jump/events?id=e1".into(),
                event_id: Some("e1".into())
            }
        );
        assert_eq!(list[2].timestamp, 100);
    }

    #[tokio::test]
    async fn friend_requests_from_blocked_senders_are_dropped() {
        let mock = Mock::ok(&feed());
        let list = notifications(&client(&mock), Query::default(), &|a| a == "0x1")
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn the_query_is_signed_get_with_clamped_limit() {
        let mock = Mock::ok(r#"{"notifications":[]}"#);
        let query = Query {
            from: Some(5),
            limit: Some(500),
            only_unread: true,
        };
        notifications(&client(&mock), query, &nobody).await.unwrap();
        let sent = mock.only_sent();
        assert_eq!(sent.method, "GET");
        assert_eq!(
            sent.url,
            format!(
                "{}/notifications?from=5&limit=50&onlyUnread=true",
                urls::notifications_api()
            )
        );
        assert_eq!(sent.header("x-identity-metadata"), Some("{}"));
    }

    #[tokio::test]
    async fn an_empty_query_has_no_parameters() {
        let mock = Mock::ok(r#"{"notifications":[]}"#);
        notifications(&client(&mock), Query::default(), &nobody)
            .await
            .unwrap();
        assert!(!mock.only_sent().url.contains('?'));
    }

    #[tokio::test]
    async fn a_body_without_the_array_is_malformed() {
        let mock = Mock::ok(r#"{"oops":1}"#);
        assert!(matches!(
            notifications(&client(&mock), Query::default(), &nobody).await,
            Err(SignedError::Malformed(_))
        ));
    }

    #[test]
    fn unsupported_kinds_and_entries_without_an_id_are_dropped() {
        assert_eq!(
            parse_notification(&json!({"id": "x", "type": "item_sold"})),
            None
        );
        assert_eq!(parse_notification(&json!({"type": "events_started"})), None);
    }

    #[tokio::test]
    async fn marking_read_puts_the_ids_and_signs_the_body() {
        let mock = Mock::ok(r#"{"updated":2}"#);
        let ids = vec!["a".to_owned(), "b".to_owned()];
        let updated = mark_notifications_read(&client(&mock), &ids).await.unwrap();
        let sent = mock.only_sent();
        assert_eq!(updated, 2);
        assert_eq!(sent.method, "PUT");
        assert_eq!(
            sent.url,
            format!("{}/notifications/read", urls::notifications_api())
        );
        assert_eq!(
            sent.body.as_deref(),
            Some(r#"{"notificationIds":["a","b"]}"#)
        );
        assert_eq!(
            sent.header("x-identity-metadata"),
            Some(r#"{"notificationIds":["a","b"]}"#)
        );
    }

    #[tokio::test]
    async fn marking_nothing_read_is_refused_without_a_request() {
        let mock = Mock::answering(vec![]);
        assert!(matches!(
            mark_notifications_read(&client(&mock), &[]).await,
            Err(SignedError::InvalidArgument(_))
        ));
        assert!(mock.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_reply_without_the_count_is_malformed() {
        let mock = Mock::ok("{}");
        assert!(matches!(
            mark_notifications_read(&client(&mock), &["a".into()]).await,
            Err(SignedError::Malformed(_))
        ));
    }
}
