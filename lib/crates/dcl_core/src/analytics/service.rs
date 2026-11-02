//! The Segment sender the whole app shares: events from the shell and the engine are stamped
//! when they happen, held until the user has accepted the EULA, and shipped in size-limited
//! batches. A batch that cannot be sent is kept for the next flush.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{SecondsFormat, Utc};
use tokio::{runtime::Handle, sync::watch, task::JoinHandle};
use uuid::Uuid;

use super::{
    data_definition::{
        build_segment_event_batch_item, build_segment_identify_body, PushIdentifyTraits,
        SegmentEvent, SegmentEventCommonExplorerFields, SegmentEventCustom,
        SegmentEventExplorerMoveToParcel, SegmentEventGuestWalletCreation,
        SegmentEventScreenViewed,
    },
    property::{to_object, Property},
    queue::{SEGMENT_BATCH_SIZE_LIMIT_BYTES, SEGMENT_EVENT_SIZE_LIMIT_BYTES},
    transport::SegmentTransport,
};
use crate::hub::{GuestLoginReport, SessionObserver};
use crate::session::identity::SessionEvent;

/// Segment write key of the explorer's source.
pub const DEFAULT_WRITE_KEY: &str = "EAdAcIyGP6lIQAfpFF2BXpNzpj7XNWMm";

pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_secs(10);
const MIN_FLUSH_INTERVAL: Duration = Duration::from_millis(500);

/// Bounds memory during a long outage or a user who never accepts: past it the oldest go.
const MAX_PENDING_EVENTS: usize = 5000;

#[derive(Clone, Debug)]
pub struct AnalyticsConfig {
    pub write_key: String,
    /// The per-install anonymous id (`session.json`), the Segment `userId` of every event.
    pub user_id: String,
    pub session_id: String,
    /// The platform name Godot reports ("Android", "iOS", "Linux", ...).
    pub os_name: String,
    pub renderer_version: String,
    pub utc_offset_minutes: i32,
}

struct State {
    user_id: String,
    write_key: String,
    common: SegmentEventCommonExplorerFields,
    eula_accepted: bool,
    /// Serialized batch items, oldest first; each already carries its time and message id.
    pending: Vec<String>,
}

pub struct Analytics {
    state: Mutex<State>,
    transport: Arc<dyn SegmentTransport>,
    interval: watch::Sender<Duration>,
}

impl Analytics {
    pub fn new(config: AnalyticsConfig, transport: Arc<dyn SegmentTransport>) -> Self {
        let common = SegmentEventCommonExplorerFields::new(
            config.session_id,
            &config.os_name,
            &config.renderer_version,
            config.utc_offset_minutes,
        );
        Self {
            state: Mutex::new(State {
                user_id: config.user_id,
                write_key: config.write_key,
                common,
                eula_accepted: false,
                pending: Vec::new(),
            }),
            transport,
            interval: watch::channel(DEFAULT_FLUSH_INTERVAL).0,
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The session this process runs, the one every event carries.
    pub fn session_id(&self) -> String {
        self.state().common.session_id.clone()
    }

    /// Queues an event the shell names. Keys that would overwrite a field every event carries
    /// (`session_id`, `realm`, ...) are dropped.
    pub fn track(&self, name: &str, properties: &[Property]) {
        let mut object = to_object(properties);
        let reserved = self.state().common_keys();
        object.retain(|key, _| !reserved.contains(key));
        self.queue_event(SegmentEvent::Custom(SegmentEventCustom {
            name: name.to_string(),
            properties: object.into(),
        }));
    }

    /// Queues a "Screen Viewed"; the properties travel as the event's `extra_properties` JSON,
    /// the shape the warehouse already has.
    pub fn track_screen(&self, name: &str, properties: &[Property]) {
        let object = to_object(properties);
        let extra_properties =
            (!object.is_empty()).then(|| serde_json::Value::from(object).to_string());
        self.queue_event(SegmentEvent::ScreenViewed(SegmentEventScreenViewed {
            screen_name: name.to_string(),
            extra_properties,
        }));
    }

    /// Queues an event built by the engine or the core (loading funnel, performance, friends).
    pub fn queue_event(&self, event: SegmentEvent) {
        let mut state = self.state();
        let item = build_segment_event_batch_item(
            state.user_id.clone(),
            &state.common,
            event,
            Utc::now(),
            Uuid::new_v4().to_string(),
        );
        match serde_json::to_string(&item) {
            Ok(json) => state.push(json),
            Err(e) => tracing::error!("analytics event not serializable: {e}"),
        }
    }

    /// The push `identify`: the user's current push state, one row per user in the warehouse.
    pub fn queue_push_identify(&self, traits: PushIdentifyTraits) {
        let mut state = self.state();
        let body = build_segment_identify_body(
            state.user_id.clone(),
            traits,
            Utc::now(),
            Uuid::new_v4().to_string(),
        );
        match serde_json::to_string(&body) {
            Ok(json) => state.push(json),
            Err(e) => tracing::error!("analytics identify not serializable: {e}"),
        }
    }

    pub fn update_realm(&self, realm: &str) {
        self.state().common.realm = realm.to_string();
    }

    /// Moves the player to `position` ("x,y") and queues the "Explorer Move To Parcel" event.
    pub fn update_position(&self, position: &str) {
        let old_parcel = self.state().common.position.clone();
        self.queue_event(SegmentEvent::ExplorerMoveToParcel(
            position.to_string(),
            SegmentEventExplorerMoveToParcel { old_parcel },
        ));
        self.state().common.position = position.to_string();
    }

    pub fn update_identity(&self, address: &str, is_guest: bool) {
        let mut state = self.state();
        state.common.dcl_eth_address = address.to_string();
        state.common.dcl_is_guest = is_guest;
    }

    /// Nothing is shipped before the user accepts the EULA; accepting ships what waited.
    pub async fn set_eula_accepted(&self, accepted: bool) {
        let opened = {
            let mut state = self.state();
            let opened = accepted && !state.eula_accepted;
            state.eula_accepted = accepted;
            opened
        };
        if opened {
            self.flush().await;
        }
    }

    pub fn eula_accepted(&self) -> bool {
        self.state().eula_accepted
    }

    pub fn pending_events(&self) -> usize {
        self.state().pending.len()
    }

    /// Ships everything pending, in batches within Segment's size limit. A transport failure
    /// puts the unsent events back in front of the ones queued meanwhile and stops.
    pub async fn flush(&self) {
        let (write_key, items) = {
            let mut state = self.state();
            if !state.eula_accepted || state.pending.is_empty() {
                return;
            }
            (state.write_key.clone(), std::mem::take(&mut state.pending))
        };
        let mut batches = split_batches(items).into_iter();
        while let Some(batch) = batches.next() {
            if let Err(error) = self
                .transport
                .send_batch(batch_body(&write_key, &batch))
                .await
            {
                let unsent: Vec<String> = std::iter::once(batch)
                    .chain(batches.by_ref())
                    .flatten()
                    .collect();
                tracing::warn!(
                    "Segment batch not sent, keeping {} events: {error}",
                    unsent.len()
                );
                self.state().restore(unsent);
                return;
            }
        }
    }

    /// Changes the periodic flush cadence, applied immediately; below half a second is clamped.
    pub fn set_flush_interval(&self, interval: Duration) {
        self.interval.send_replace(interval.max(MIN_FLUSH_INTERVAL));
    }

    /// Flushes every interval on `handle` until the service is dropped.
    pub fn start_flush_loop(self: &Arc<Self>, handle: &Handle) -> JoinHandle<()> {
        let service = Arc::downgrade(self);
        let mut interval = self.interval.subscribe();
        handle.spawn(async move {
            loop {
                let wait = *interval.borrow_and_update();
                tokio::select! {
                    _ = tokio::time::sleep(wait) => {
                        let Some(service) = service.upgrade() else { return };
                        service.flush().await;
                    }
                    changed = interval.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
            }
        })
    }
}

impl State {
    fn common_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = serde_json::to_value(&self.common)
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
        keys.push("client_timestamp".to_string());
        keys
    }

    fn push(&mut self, item: String) {
        if item.len() > SEGMENT_EVENT_SIZE_LIMIT_BYTES {
            tracing::error!("analytics event too large: {} bytes", item.len());
            return;
        }
        self.pending.push(item);
        self.trim();
    }

    /// Puts events that failed to send back in front of the newer ones.
    fn restore(&mut self, mut unsent: Vec<String>) {
        unsent.append(&mut self.pending);
        self.pending = unsent;
        self.trim();
    }

    fn trim(&mut self) {
        if self.pending.len() > MAX_PENDING_EVENTS {
            let overflow = self.pending.len() - MAX_PENDING_EVENTS;
            self.pending.drain(0..overflow);
            tracing::warn!("analytics queue over {MAX_PENDING_EVENTS}: dropped {overflow} oldest");
        }
    }
}

fn split_batches(items: Vec<String>) -> Vec<Vec<String>> {
    let mut batches: Vec<Vec<String>> = Vec::new();
    let mut size = 0;
    for item in items {
        if batches.is_empty() || size + item.len() > SEGMENT_BATCH_SIZE_LIMIT_BYTES {
            batches.push(Vec::new());
            size = 0;
        }
        size += item.len();
        if let Some(batch) = batches.last_mut() {
            batch.push(item);
        }
    }
    batches
}

/// `sentAt` is stamped at the moment of sending: Segment pairs it with each event's own
/// `timestamp` to correct device clock skew.
fn batch_body(write_key: &str, items: &[String]) -> String {
    let sent_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    format!(
        "{{\"writeKey\":\"{write_key}\",\"sentAt\":\"{sent_at}\",\"batch\":[{}]}}",
        items.join(",")
    )
}

/// Keeps the identity every event carries in step with the session, and reports the silent
/// guest login.
impl SessionObserver for Analytics {
    fn session_event(&self, event: &SessionEvent) {
        match event {
            SessionEvent::WalletConnected {
                address, is_guest, ..
            } => self.update_identity(address, *is_guest),
            SessionEvent::Logout => self.update_identity("unauthenticated", false),
        }
    }

    fn guest_login(&self, report: &GuestLoginReport) {
        self.queue_event(SegmentEvent::GuestWalletCreation(
            SegmentEventGuestWalletCreation {
                outcome: if report.success { "success" } else { "failure" }.to_string(),
                is_new_user: report.is_new_user,
                failure_reason: (!report.failure_reason.is_empty())
                    .then(|| report.failure_reason.clone()),
                http_status: report.http_status.map(u32::from),
                duration_ms: report.duration_ms.min(u32::MAX as u64) as u32,
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use async_trait::async_trait;
    use serde_json::Value;

    use super::*;
    use crate::analytics::property::PropertyValue;

    #[derive(Default)]
    struct FakeTransport {
        bodies: Mutex<Vec<String>>,
        offline: AtomicBool,
    }

    impl FakeTransport {
        fn batches(&self) -> Vec<Value> {
            self.bodies
                .lock()
                .unwrap()
                .iter()
                .map(|b| serde_json::from_str(b).unwrap())
                .collect()
        }

        fn items(&self) -> Vec<Value> {
            self.batches()
                .iter()
                .flat_map(|b| b["batch"].as_array().unwrap().clone())
                .collect()
        }
    }

    #[async_trait]
    impl SegmentTransport for FakeTransport {
        async fn send_batch(&self, body: String) -> Result<(), String> {
            if self.offline.load(Ordering::SeqCst) {
                return Err("offline".into());
            }
            self.bodies.lock().unwrap().push(body);
            Ok(())
        }
    }

    fn setup() -> (Arc<Analytics>, Arc<FakeTransport>) {
        let transport = Arc::new(FakeTransport::default());
        let analytics = Arc::new(Analytics::new(
            AnalyticsConfig {
                write_key: "key".into(),
                user_id: "user-1".into(),
                session_id: "session-1".into(),
                os_name: "Android".into(),
                renderer_version: "2.0.0".into(),
                utc_offset_minutes: -180,
            },
            transport.clone(),
        ));
        (analytics, transport)
    }

    fn text(key: &str, value: &str) -> Property {
        Property {
            key: key.into(),
            value: PropertyValue::Text {
                value: value.into(),
            },
        }
    }

    #[tokio::test]
    async fn nothing_ships_before_the_eula_and_accepting_ships_it_all() {
        let (analytics, transport) = setup();
        analytics.track("Started", &[]);
        analytics.flush().await;
        assert!(transport.batches().is_empty());
        assert_eq!(analytics.pending_events(), 1);

        analytics.set_eula_accepted(true).await;
        assert_eq!(transport.items().len(), 1);
        assert_eq!(analytics.pending_events(), 0);
    }

    #[tokio::test]
    async fn an_event_carries_identity_stamp_and_common_fields() {
        let (analytics, transport) = setup();
        analytics.update_realm("https://realm");
        analytics.update_identity("0xabc", true);
        analytics.track("Tapped", &[text("what", "play")]);
        analytics.set_eula_accepted(true).await;

        let batch = &transport.batches()[0];
        assert_eq!(batch["writeKey"], "key");
        assert!(batch["sentAt"].as_str().unwrap().ends_with('Z'));
        let item = &batch["batch"][0];
        assert_eq!(item["type"], "track");
        assert_eq!(item["event"], "Tapped");
        assert_eq!(item["userId"], "user-1");
        assert_eq!(item["messageId"].as_str().unwrap().len(), 36);
        assert_eq!(item["properties"]["what"], "play");
        assert_eq!(item["properties"]["session_id"], "session-1");
        assert_eq!(item["properties"]["realm"], "https://realm");
        assert_eq!(item["properties"]["dcl_eth_address"], "0xabc");
        assert_eq!(item["properties"]["dcl_is_guest"], true);
        assert_eq!(item["properties"]["dcl_renderer_type"], "dao-godot-Android");
        assert_eq!(item["timestamp"], item["properties"]["client_timestamp"]);
    }

    #[tokio::test]
    async fn the_event_keeps_the_identity_it_had_when_it_happened() {
        let (analytics, transport) = setup();
        analytics.update_identity("0xaaa", false);
        analytics.track("First", &[]);
        analytics.update_identity("0xbbb", false);
        analytics.set_eula_accepted(true).await;
        assert_eq!(
            transport.items()[0]["properties"]["dcl_eth_address"],
            "0xaaa"
        );
    }

    #[tokio::test]
    async fn properties_cannot_overwrite_the_common_fields() {
        let (analytics, transport) = setup();
        analytics.track(
            "Sneaky",
            &[text("session_id", "forged"), text("client_timestamp", "x")],
        );
        analytics.set_eula_accepted(true).await;
        let props = &transport.items()[0]["properties"];
        assert_eq!(props["session_id"], "session-1");
        assert_ne!(props["client_timestamp"], "x");
    }

    #[tokio::test]
    async fn screens_keep_the_extra_properties_json_shape() {
        let (analytics, transport) = setup();
        analytics.track_screen("HOME", &[]);
        analytics.track_screen("LOADING_END", &[text("reason", "done")]);
        analytics.set_eula_accepted(true).await;
        let items = transport.items();
        assert_eq!(items[0]["event"], "Screen Viewed");
        assert_eq!(items[0]["properties"]["screen_name"], "HOME");
        assert!(items[0]["properties"].get("extra_properties").is_none());
        let extra = items[1]["properties"]["extra_properties"].as_str().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(extra).unwrap(),
            serde_json::json!({"reason": "done"})
        );
    }

    #[tokio::test]
    async fn moving_queues_the_event_with_the_old_parcel() {
        let (analytics, transport) = setup();
        analytics.update_position("1,2");
        analytics.update_position("3,4");
        analytics.set_eula_accepted(true).await;
        let items = transport.items();
        assert_eq!(items[0]["event"], "Explorer Move To Parcel");
        assert_eq!(items[0]["properties"]["old_parcel"], "no-position");
        assert_eq!(items[0]["properties"]["position"], "1,2");
        assert_eq!(items[1]["properties"]["old_parcel"], "1,2");
        assert_eq!(items[1]["properties"]["position"], "3,4");
    }

    #[tokio::test]
    async fn every_event_has_its_own_message_id() {
        let (analytics, transport) = setup();
        analytics.track("A", &[]);
        analytics.track("A", &[]);
        analytics.set_eula_accepted(true).await;
        let items = transport.items();
        assert_ne!(items[0]["messageId"], items[1]["messageId"]);
    }

    #[tokio::test]
    async fn a_failed_send_is_kept_and_retried_with_the_same_ids_in_order() {
        let (analytics, transport) = setup();
        analytics.set_eula_accepted(true).await;
        transport.offline.store(true, Ordering::SeqCst);
        analytics.track("One", &[]);
        analytics.flush().await;
        assert_eq!(analytics.pending_events(), 1);

        analytics.track("Two", &[]);
        transport.offline.store(false, Ordering::SeqCst);
        analytics.flush().await;

        let items = transport.items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["event"], "One");
        assert_eq!(items[1]["event"], "Two");
        assert_eq!(analytics.pending_events(), 0);
    }

    #[tokio::test]
    async fn a_burst_is_split_into_batches_within_the_size_limit() {
        let (analytics, transport) = setup();
        let big = "x".repeat(20_000);
        for _ in 0..40 {
            analytics.track("Big", &[text("blob", &big)]);
        }
        analytics.set_eula_accepted(true).await;
        let bodies = transport.bodies.lock().unwrap().clone();
        assert!(bodies.len() > 1);
        assert!(bodies.iter().all(|b| b.len() < 520_000));
        assert_eq!(transport.items().len(), 40);
    }

    #[tokio::test]
    async fn an_oversized_event_is_dropped() {
        let (analytics, _) = setup();
        analytics.track("Huge", &[text("blob", &"x".repeat(40_000))]);
        assert_eq!(analytics.pending_events(), 0);
    }

    #[tokio::test]
    async fn the_queue_drops_the_oldest_past_its_bound() {
        let (analytics, transport) = setup();
        for i in 0..(MAX_PENDING_EVENTS + 3) {
            analytics.track(
                "E",
                &[Property {
                    key: "i".into(),
                    value: PropertyValue::Int { value: i as i64 },
                }],
            );
        }
        assert_eq!(analytics.pending_events(), MAX_PENDING_EVENTS);
        analytics.set_eula_accepted(true).await;
        assert_eq!(transport.items()[0]["properties"]["i"], 3);
    }

    #[test]
    fn session_events_set_the_identity_and_guest_logins_are_reported() {
        let (analytics, _) = setup();
        analytics.session_event(&SessionEvent::WalletConnected {
            address: "0xabc".into(),
            chain_id: 1,
            is_guest: true,
        });
        assert_eq!(analytics.state().common.dcl_eth_address, "0xabc");
        assert!(analytics.state().common.dcl_is_guest);
        analytics.session_event(&SessionEvent::Logout);
        assert_eq!(analytics.state().common.dcl_eth_address, "unauthenticated");

        analytics.guest_login(&GuestLoginReport {
            success: false,
            is_new_user: false,
            failure_reason: "timeout".into(),
            http_status: Some(504),
            duration_ms: 1200,
        });
        assert_eq!(analytics.pending_events(), 1);
    }

    #[tokio::test]
    async fn the_push_identify_rides_the_same_batch() {
        use crate::analytics::data_definition::PushPlatform;
        let (analytics, transport) = setup();
        analytics.queue_push_identify(PushIdentifyTraits {
            platform: PushPlatform::Android,
            token: "tok".into(),
            push_permission: "granted".into(),
            app_version: "2.0.0".into(),
            locale: "es_AR".into(),
            timezone: "America/Argentina/Buenos_Aires".into(),
        });
        analytics.set_eula_accepted(true).await;
        let item = &transport.items()[0];
        assert_eq!(item["type"], "identify");
        assert_eq!(item["userId"], "user-1");
        assert_eq!(item["traits"]["fcm_token"], "tok");
    }

    #[tokio::test(start_paused = true)]
    async fn the_loop_flushes_every_interval_and_follows_a_new_one() {
        let (analytics, transport) = setup();
        analytics.set_eula_accepted(true).await;
        let handle = analytics.start_flush_loop(&Handle::current());

        analytics.track("A", &[]);
        tokio::time::sleep(Duration::from_secs(9)).await;
        assert!(transport.items().is_empty(), "not before the 10 s tick");
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_eq!(transport.items().len(), 1);

        analytics.set_flush_interval(Duration::from_secs(2));
        analytics.track("B", &[]);
        tokio::time::sleep(Duration::from_millis(2500)).await;
        assert_eq!(transport.items().len(), 2);

        analytics.set_flush_interval(Duration::ZERO);
        assert_eq!(*analytics.interval.borrow(), MIN_FLUSH_INTERVAL);

        drop(analytics);
        handle.await.unwrap();
    }
}
