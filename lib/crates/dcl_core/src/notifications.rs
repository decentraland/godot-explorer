//! The in-app notification feed: what the notifications API returned, which of it is worth a
//! toast, and the queue of toasts waiting to be shown. Fetching is the host's job; this is the
//! state machine that decides what to keep.

use std::collections::{HashSet, VecDeque};

use serde_json::Value;

/// Only the types whose screens exist are shown.
pub const SUPPORTED_TYPES: [&str; 4] = [
    "events_started",
    "reward_assignment",
    "social_service_friendship_request",
    "social_service_friendship_accepted",
];

/// A notification older than this is kept in the list but never raises a toast.
pub const TOAST_MAX_AGE_MS: i64 = 5 * 60 * 1000;

/// What `ingest` changed in the toast queue.
#[derive(Debug, Clone, PartialEq)]
pub struct Ingest {
    /// New unread, recent notifications appended to the toast queue.
    pub queued: usize,
    /// The queue head after ingesting, when it was empty before: the toast to raise.
    pub raise: Option<Value>,
}

#[derive(Default)]
pub struct Feed {
    notifications: Vec<Value>,
    queue: VecDeque<Value>,
}

/// Godot's `int()` of a timestamp: numbers truncate, strings read their leading integer.
fn timestamp_of(notification: &Value) -> i64 {
    match notification.get("timestamp") {
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f as i64))
            .unwrap_or(0),
        Some(Value::String(text)) => {
            let text = text.trim_start();
            let (sign, digits) = match text.strip_prefix('-') {
                Some(rest) => (-1, rest),
                None => (1, text.strip_prefix('+').unwrap_or(text)),
            };
            let end = digits
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(digits.len());
            sign * digits[..end].parse::<i64>().unwrap_or(0)
        }
        _ => 0,
    }
}

fn is_read(notification: &Value) -> bool {
    notification
        .get("read")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn id_key(notification: &Value) -> Option<String> {
    notification.get("id").map(|id| match id {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    })
}

fn sender_address(notification: &Value) -> &str {
    notification
        .pointer("/metadata/sender/address")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// Keeps the supported types, and drops friend requests from senders `is_blocked` rejects.
/// With the filter off every notification passes (testing aid).
pub fn filter(
    notifications: Vec<Value>,
    enabled: bool,
    is_blocked: &dyn Fn(&str) -> bool,
) -> Vec<Value> {
    if !enabled {
        return notifications;
    }
    notifications
        .into_iter()
        .filter(|notification| {
            let Some(kind) = notification.get("type").and_then(Value::as_str) else {
                return false;
            };
            if !SUPPORTED_TYPES.contains(&kind) {
                return false;
            }
            if kind == "social_service_friendship_request" {
                let sender = sender_address(notification);
                if !sender.is_empty() && is_blocked(sender) {
                    return false;
                }
            }
            true
        })
        .collect()
}

impl Feed {
    /// Merges `incoming` (already filtered) into the list, skipping ids already known, and queues
    /// the unread, recent ones for a toast.
    pub fn ingest(&mut self, incoming: Vec<Value>, now_ms: i64) -> Ingest {
        let mut known: HashSet<String> = self.notifications.iter().filter_map(id_key).collect();
        let mut queued = 0;
        for notification in incoming {
            let Some(id) = id_key(&notification) else {
                continue;
            };
            if !known.insert(id) {
                continue;
            }
            if !is_read(&notification) && timestamp_of(&notification) > now_ms - TOAST_MAX_AGE_MS {
                self.queue.push_back(notification.clone());
                queued += 1;
            }
            self.notifications.push(notification);
        }
        let raise = (queued > 0 && self.queue.len() == queued)
            .then(|| self.queue.front().cloned())
            .flatten();
        Ingest { queued, raise }
    }

    pub fn mark_read(&mut self, ids: &[String]) {
        for notification in &mut self.notifications {
            let matches = id_key(notification).is_some_and(|id| ids.contains(&id));
            if let (true, Some(object)) = (matches, notification.as_object_mut()) {
                object.insert("read".into(), Value::Bool(true));
            }
        }
    }

    pub fn all(&self) -> &[Value] {
        &self.notifications
    }

    /// Newest first.
    pub fn sorted(&self) -> Vec<Value> {
        let mut sorted = self.notifications.clone();
        sorted.sort_by_key(|notification| std::cmp::Reverse(timestamp_of(notification)));
        sorted
    }

    /// Forgets the list and the pending toasts, e.g. on sign-out.
    pub fn clear(&mut self) {
        self.notifications.clear();
        self.queue.clear();
    }

    // ---- toast queue

    /// Appends a toast and returns the queue length afterwards.
    pub fn enqueue(&mut self, notification: Value) -> usize {
        self.queue.push_back(notification);
        self.queue.len()
    }

    pub fn peek(&self) -> Option<&Value> {
        self.queue.front()
    }

    pub fn pop(&mut self) -> Option<Value> {
        self.queue.pop_front()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Drops transient system toasts waiting behind the head. The head may be on screen, and the
    /// consumer's on-close pop takes index 0, so it is never touched.
    pub fn clear_system_toasts(&mut self) {
        let Some(head) = self.queue.pop_front() else {
            return;
        };
        self.queue
            .retain(|n| n.get("type").and_then(Value::as_str) != Some("system"));
        self.queue.push_front(head);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: i64 = 1_700_000_000_000;

    fn note(id: &str, kind: &str, age_ms: i64, read: bool) -> Value {
        json!({"id": id, "type": kind, "timestamp": NOW - age_ms, "read": read})
    }

    fn no_block(_: &str) -> bool {
        false
    }

    #[test]
    fn only_supported_types_pass_the_filter() {
        let kept = filter(
            vec![
                note("a", "events_started", 0, false),
                note("b", "item_sold", 0, false),
                json!({"id": "c"}),
            ],
            true,
            &no_block,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["id"], "a");
        assert_eq!(
            filter(vec![note("b", "item_sold", 0, false)], false, &no_block).len(),
            1
        );
    }

    #[test]
    fn friend_requests_from_blocked_senders_are_dropped() {
        let request = json!({"id": "r", "type": "social_service_friendship_request",
            "metadata": {"sender": {"address": "0xbad"}}});
        let accepted = json!({"id": "s", "type": "social_service_friendship_accepted",
            "metadata": {"sender": {"address": "0xbad"}}});
        let blocked = |address: &str| address == "0xbad";
        let kept = filter(vec![request, accepted], true, &blocked);
        assert_eq!(
            kept.len(),
            1,
            "only the request is checked against the blacklist"
        );
        assert_eq!(kept[0]["id"], "s");
    }

    #[test]
    fn duplicates_are_skipped_and_only_recent_unread_ones_queue() {
        let mut feed = Feed::default();
        let first = feed.ingest(
            vec![
                note("fresh", "events_started", 1_000, false),
                note("old", "events_started", TOAST_MAX_AGE_MS + 1, false),
                note("seen", "events_started", 1_000, true),
            ],
            NOW,
        );
        assert_eq!(first.queued, 1);
        assert_eq!(first.raise.unwrap()["id"], "fresh");
        assert_eq!(feed.all().len(), 3);

        let again = feed.ingest(vec![note("fresh", "events_started", 1_000, false)], NOW);
        assert_eq!(
            again,
            Ingest {
                queued: 0,
                raise: None
            }
        );
        assert_eq!(feed.all().len(), 3);
    }

    #[test]
    fn a_toast_is_raised_only_when_the_queue_was_empty() {
        let mut feed = Feed::default();
        feed.ingest(vec![note("a", "events_started", 0, false)], NOW);
        let second = feed.ingest(vec![note("b", "events_started", 0, false)], NOW);
        assert_eq!(second.queued, 1);
        assert_eq!(second.raise, None, "the first toast is still showing");
        assert_eq!(feed.queue_len(), 2);
    }

    #[test]
    fn mark_read_flips_only_the_named_ones() {
        let mut feed = Feed::default();
        feed.ingest(
            vec![
                note("a", "events_started", 0, false),
                note("b", "events_started", 0, false),
            ],
            NOW,
        );
        feed.mark_read(&["a".to_string()]);
        assert_eq!(feed.all()[0]["read"], true);
        assert_eq!(feed.all()[1]["read"], false);
    }

    #[test]
    fn sorting_is_newest_first_and_reads_string_timestamps() {
        let mut feed = Feed::default();
        feed.ingest(
            vec![
                note("old", "events_started", 5_000, true),
                json!({"id": "text", "type": "events_started", "timestamp": format!("{}", NOW), "read": true}),
                note("new", "events_started", 1_000, true),
            ],
            NOW,
        );
        let order: Vec<_> = feed
            .sorted()
            .iter()
            .map(|n| n["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(order, ["text", "new", "old"]);
    }

    #[test]
    fn clearing_system_toasts_never_touches_the_head() {
        let mut feed = Feed::default();
        feed.enqueue(json!({"id": "head", "type": "system"}));
        feed.enqueue(json!({"id": "x", "type": "system"}));
        feed.enqueue(json!({"id": "real", "type": "events_started"}));
        feed.clear_system_toasts();
        assert_eq!(feed.queue_len(), 2);
        assert_eq!(feed.peek().unwrap()["id"], "head");
        feed.pop();
        assert_eq!(feed.peek().unwrap()["id"], "real");
    }

    #[test]
    fn clear_forgets_everything() {
        let mut feed = Feed::default();
        feed.ingest(vec![note("a", "events_started", 0, false)], NOW);
        feed.clear();
        assert!(feed.all().is_empty() && feed.queue_len() == 0);
    }
}
