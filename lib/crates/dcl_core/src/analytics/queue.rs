//! The outgoing Segment queue: events wait here, are serialized into size-limited batches and
//! shipped, and a batch that fails to send is kept for the next flush.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, SecondsFormat, Utc};
use uuid::Uuid;

use super::data_definition::{
    build_segment_event_batch_item, SegmentEvent, SegmentEventCommonExplorerFields,
};
use crate::{
    http::{
        request_response::{RequestOption, ResponseType},
        HttpQueueRequester,
    },
    Core,
};

pub(super) const SEGMENT_EVENT_SIZE_LIMIT_BYTES: usize = 32000;
pub(super) const SEGMENT_BATCH_SIZE_LIMIT_BYTES: usize = 500000;
pub(super) const SEGMENT_BATCH_URL: &str = "https://api.segment.io/v1/batch";

// Upper bound on how many failed-send events we hold for retry. Bounds memory during a long
// outage; once exceeded we drop the oldest (they're the most likely to be stale) and log it.
const MAX_RETRY_BUFFER_EVENTS: usize = 1000;

/// A Segment event together with the metadata captured the moment it was queued. Holding the
/// creation time (and a stable message id) here — rather than stamping it at flush — is what
/// keeps per-event chronology: a batch no longer collapses to a single flush instant.
struct QueuedSegmentEvent {
    event: SegmentEvent,
    created_at: DateTime<Utc>,
    // Stable per-event id (UUID v4). Kept identical across retries so Segment can deduplicate.
    message_id: String,
}

pub struct EventQueue {
    pub user_id: String,
    pub write_key: String,
    /// Fields every event carries (realm, position, wallet, …); the host updates them in place.
    pub common: SegmentEventCommonExplorerFields,
    events: Vec<QueuedSegmentEvent>,
    serialized_events: Vec<String>,
    // Events from a batch whose HTTP send failed. Drained back into `serialized_events` on the
    // next flush so transient network failures (common on mobile) don't silently drop data. The
    // JSON here already carries the original `timestamp`/`messageId`, so re-sends keep the true
    // event time and Segment deduplicates them. Shared with the detached send task.
    retry_buffer: Arc<Mutex<Vec<String>>>,
}

impl EventQueue {
    pub fn new(
        user_id: String,
        write_key: String,
        common: SegmentEventCommonExplorerFields,
    ) -> Self {
        Self {
            user_id,
            write_key,
            common,
            events: Vec::new(),
            serialized_events: Vec::new(),
            retry_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Stamps the event with its creation time and a stable message id and queues it for the
    /// next flush. Returns the stamp so the host can log exactly what will be sent.
    pub fn queue_event(&mut self, event: SegmentEvent) -> (DateTime<Utc>, String) {
        let created_at = Utc::now();
        let message_id = Uuid::new_v4().to_string();
        self.events.push(QueuedSegmentEvent {
            event,
            created_at,
            message_id: message_id.clone(),
        });
        (created_at, message_id)
    }

    /// Queues an already serialized batch item (the push identify call).
    pub fn push_serialized(&mut self, json_body: String) {
        self.serialized_events.push(json_body);
    }

    /// The event as it would be sent to Segment, for debug output.
    pub fn batch_item_json(
        &self,
        event: &SegmentEvent,
        created_at: DateTime<Utc>,
        message_id: &str,
    ) -> String {
        let item = build_segment_event_batch_item(
            self.user_id.clone(),
            &self.common,
            event.clone(),
            created_at,
            message_id.to_string(),
        );
        serde_json::to_string_pretty(&item)
            .unwrap_or_else(|e| format!("<serialization error: {e}>"))
    }

    pub fn process_and_send_events(
        &mut self,
        http_requester: Arc<HttpQueueRequester>,
        ignore_batch_limit: bool,
    ) {
        // Re-enqueue anything a previous send failed to deliver, BEFORE the empty-check below, so
        // a retry still fires even when no new events were produced this tick. The buffered JSON
        // keeps its original timestamp/messageId, so re-sends preserve event time and dedup.
        {
            let mut retry = self.retry_buffer.lock().unwrap();
            if !retry.is_empty() {
                tracing::debug!("Re-queuing {} events from retry buffer", retry.len());
                self.serialized_events.append(&mut retry);
            }
        }

        tracing::debug!(
            "process_and_send_events: events={}, serialized={}, ignore_limit={}",
            self.events.len(),
            self.serialized_events.len(),
            ignore_batch_limit
        );

        if self.events.is_empty() && self.serialized_events.is_empty() {
            tracing::debug!("No events to process, returning early");
            return;
        }

        let mut accumulated_length: usize = self.serialized_events.iter().map(|s| s.len()).sum();

        tracing::debug!("Starting event processing loop");
        while let Some(queued) = self.events.pop() {
            let raw_event = build_segment_event_batch_item(
                self.user_id.clone(),
                &self.common,
                queued.event,
                queued.created_at,
                queued.message_id,
            );

            let json_body =
                serde_json::to_string(&raw_event).expect("Failed to serialize event body");

            if json_body.len() > SEGMENT_EVENT_SIZE_LIMIT_BYTES {
                tracing::error!("Event too large: {}", json_body.len());
                continue;
            }

            if !ignore_batch_limit
                && accumulated_length + json_body.len() > SEGMENT_BATCH_SIZE_LIMIT_BYTES
            {
                self.spawn_send(http_requester);

                // This event is queued until the next time is available to send events
                self.serialized_events.push(json_body);
                return;
            }

            accumulated_length += json_body.len();
            self.serialized_events.push(json_body);
        }

        tracing::debug!(
            "Event processing loop complete. Serialized events: {}",
            self.serialized_events.len()
        );

        if self.serialized_events.is_empty() {
            tracing::debug!("No serialized events to send");
        } else {
            self.spawn_send(http_requester);
        }
    }

    fn spawn_send(&mut self, http_requester: Arc<HttpQueueRequester>) {
        let write_key = self.write_key.clone();
        let serialized_events = std::mem::take(&mut self.serialized_events);
        let retry_buffer = self.retry_buffer.clone();
        tracing::debug!(
            "Spawning async task to send {} events",
            serialized_events.len()
        );
        Core::spawn(async move {
            send_segment_batch(http_requester, &write_key, serialized_events, retry_buffer).await;
        });
    }
}

async fn send_segment_batch(
    http_requester: Arc<HttpQueueRequester>,
    write_key: &str,
    events: Vec<String>,
    retry_buffer: Arc<Mutex<Vec<String>>>,
) {
    tracing::debug!("Sending segment batch with {} events", events.len());

    for (idx, event) in events.iter().enumerate() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(event) {
            if let Some(event_name) = parsed.get("event").and_then(|v| v.as_str()) {
                tracing::debug!("  Event {}: {}", idx + 1, event_name);
            }
        }
    }

    // Batch-level `sentAt`, stamped at the actual moment of sending. Segment pairs this with
    // each event's `timestamp` (stored as `originalTimestamp`) to correct device clock skew:
    // canonical `timestamp` = receivedAt - (sentAt - originalTimestamp).
    let sent_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let json_body = format!(
        "{{\"writeKey\":\"{}\",\"sentAt\":\"{}\",\"batch\":[{}]}}",
        write_key,
        sent_at,
        events.join(",")
    );

    let request = RequestOption::new(
        0,
        SEGMENT_BATCH_URL.into(),
        http::Method::POST,
        ResponseType::AsString,
        Some(json_body.as_bytes().to_vec()),
        Some(HashMap::from([(
            "Content-Type".to_string(),
            "application/json".to_string(),
        )])),
        None,
    );
    if let Err(err) = http_requester.request(request, 0).await {
        // Transport failure (offline, timeout, DNS…). Buffer the events so the next flush
        // retries them instead of dropping data. HTTP 4xx/5xx responses arrive as Ok here and
        // are NOT retried, which avoids spinning forever on a permanently-rejected payload.
        let count = events.len();
        let mut retry = retry_buffer.lock().unwrap();
        retry.extend(events);
        if retry.len() > MAX_RETRY_BUFFER_EVENTS {
            let overflow = retry.len() - MAX_RETRY_BUFFER_EVENTS;
            retry.drain(0..overflow);
            tracing::warn!(
                "Segment retry buffer exceeded {} events; dropped {} oldest",
                MAX_RETRY_BUFFER_EVENTS,
                overflow
            );
        }
        tracing::warn!(
            "Failed to send segment batch ({} events); buffered for retry (buffer now {}): {:?}",
            count,
            retry.len(),
            err
        );
    } else {
        tracing::debug!("Segment batch sent successfully");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::data_definition::SegmentEventChatMessageSent;

    fn queue() -> EventQueue {
        EventQueue::new(
            "user-1".into(),
            "key".into(),
            SegmentEventCommonExplorerFields::new("session-1".into(), "Linux", "1.0.0", 0),
        )
    }

    fn chat() -> SegmentEvent {
        SegmentEvent::ChatMessageSent(SegmentEventChatMessageSent {
            length: 5,
            channel: "Public".into(),
            is_command: false,
            is_private: false,
            community_id: None,
            is_mention: false,
            screen_name: None,
        })
    }

    #[test]
    fn every_event_gets_its_own_stable_message_id() {
        let mut queue = queue();
        let (_, first) = queue.queue_event(chat());
        let (_, second) = queue.queue_event(chat());
        assert_ne!(first, second);
        assert_eq!(first.len(), 36, "a UUID v4, well under Segment's 100 chars");
    }

    #[test]
    fn the_debug_json_carries_the_stamp_and_the_user() {
        let mut queue = queue();
        let event = chat();
        let (created_at, message_id) = queue.queue_event(event.clone());
        let json = queue.batch_item_json(&event, created_at, &message_id);
        assert!(json.contains(&message_id));
        assert!(json.contains("user-1"));
    }
}
