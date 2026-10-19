//! The `App Opened` event and the session chain that survives process deaths.
//!
//! The OS kills the app without running any of our code, so the end of a session is never
//! observed directly, only the last moment the app was known to be alive. That mark is written
//! to disk every few seconds and on leaving the foreground; an open measures its gap against it.
//! The shell drives the tracker (launch, foreground, background, a tick) and the core decides
//! when an open is real and what brought the player in.

use std::{fs, io, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};

use super::data_definition::{SegmentEvent, SegmentEventAppOpened};

/// Bounds how stale the liveness mark can be when the OS kills us mid-session.
pub const HEARTBEAT_MS: u64 = 30_000;
/// Warm returns quicker than this are the app coming back from its own dialogs (permission
/// prompt, browser hop, share sheet). Not the session cutoff: that lives in the warehouse query.
pub const WARM_OPEN_FLOOR_SECS: i64 = 30;
/// The launch intent can arrive after the app started, so an open waits this long for a deep
/// link before concluding that the icon brought the player in.
pub const DEEP_LINK_SETTLE_MS: u64 = 1_500;
/// How long a deep link waits for its open: long enough to cross the notification order, short
/// enough that a link tapped mid-session never attributes a later open.
pub const DEEP_LINK_STASH_MS: u64 = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Trigger {
    Icon,
    Deeplink,
    Push,
}

impl Trigger {
    fn name(self) -> &'static str {
        match self {
            Self::Icon => "icon",
            Self::Deeplink => "deeplink",
            Self::Push => "push",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Start {
    Cold,
    Warm,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub session_id: String,
    pub last_seen: i64,
}

/// Where the liveness mark lives.
pub trait MarkStore: Send + Sync {
    fn load(&self) -> Option<Mark>;
    fn save(&self, mark: &Mark) -> io::Result<()>;
}

/// The mark as a JSON file, written aside and renamed in so a kill mid-write never costs the
/// previous mark, the one thing the file exists to hold.
pub struct FileMarkStore {
    path: PathBuf,
}

impl FileMarkStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl MarkStore for FileMarkStore {
    fn load(&self) -> Option<Mark> {
        serde_json::from_str(&fs::read_to_string(&self.path).ok()?).ok()
    }

    fn save(&self, mark: &Mark) -> io::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string(mark)?)?;
        fs::rename(tmp, &self.path)
    }
}

struct Pending {
    start: Start,
    prev_session_id: String,
    seconds_since: i64,
    deadline_ms: u64,
}

struct Inner {
    prev: Mark,
    pending: Option<Pending>,
    stash: Option<(Trigger, u64)>,
    next_heartbeat_ms: u64,
}

pub struct SessionTracker {
    session_id: String,
    store: Box<dyn MarkStore>,
    inner: Mutex<Inner>,
}

impl SessionTracker {
    pub fn new(session_id: String, store: Box<dyn MarkStore>) -> Self {
        let prev = store.load().unwrap_or_default();
        Self {
            session_id,
            store,
            inner: Mutex::new(Inner {
                prev,
                pending: None,
                stash: None,
                next_heartbeat_ms: 0,
            }),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The process started: an open is pending until a link or the settle window resolves it.
    pub fn launched(&self, now_unix: i64, now_ms: u64) -> Option<SegmentEvent> {
        let opened = self.begin(Start::Cold, now_unix, now_ms);
        self.write_mark(now_unix, now_ms);
        opened
    }

    /// The app came back from the background.
    pub fn foregrounded(&self, now_unix: i64, now_ms: u64) -> Option<SegmentEvent> {
        self.begin(Start::Warm, now_unix, now_ms)
    }

    /// The app left the foreground: pins the mark to the moment we actually left and drops a
    /// stashed link, which belongs to the session that just ended.
    pub fn backgrounded(&self, now_unix: i64, now_ms: u64) {
        self.write_mark(now_unix, now_ms);
        self.inner().stash = None;
    }

    /// A deep link was routed. It resolves the pending open, or waits briefly for the open it
    /// belongs to (a link that beat it by a notification slot).
    pub fn deep_link(&self, trigger: Trigger, now_ms: u64) -> Option<SegmentEvent> {
        let mut inner = self.inner();
        if inner.pending.is_none() {
            inner.stash = Some((trigger, now_ms));
            return None;
        }
        Self::resolve(&mut inner, trigger)
    }

    /// Call every few seconds while running: ends the settle window and keeps the mark fresh.
    pub fn tick(&self, now_unix: i64, now_ms: u64) -> Option<SegmentEvent> {
        let (opened, beat) = {
            let mut inner = self.inner();
            if inner
                .stash
                .is_some_and(|(_, at)| now_ms.saturating_sub(at) >= DEEP_LINK_STASH_MS)
            {
                inner.stash = None;
            }
            let due = inner
                .pending
                .as_ref()
                .is_some_and(|pending| now_ms >= pending.deadline_ms);
            let opened = due
                .then(|| Self::resolve(&mut inner, Trigger::Icon))
                .flatten();
            (opened, now_ms >= inner.next_heartbeat_ms)
        };
        if beat {
            self.write_mark(now_unix, now_ms);
        }
        opened
    }

    fn begin(&self, start: Start, now_unix: i64, now_ms: u64) -> Option<SegmentEvent> {
        let mut inner = self.inner();
        if inner.pending.is_some() {
            return None;
        }
        let seconds_since = if inner.prev.last_seen <= 0 {
            -1
        } else {
            // A clock that moved backwards reads as unknown, not as zero.
            let delta = now_unix - inner.prev.last_seen;
            if delta >= 0 {
                delta
            } else {
                -1
            }
        };
        if start == Start::Warm && seconds_since < WARM_OPEN_FLOOR_SECS {
            return None;
        }
        inner.pending = Some(Pending {
            start,
            prev_session_id: inner.prev.session_id.clone(),
            seconds_since,
            deadline_ms: now_ms + DEEP_LINK_SETTLE_MS,
        });
        let stashed = inner
            .stash
            .take()
            .filter(|(_, at)| now_ms.saturating_sub(*at) < DEEP_LINK_STASH_MS);
        stashed.and_then(|(trigger, _)| Self::resolve(&mut inner, trigger))
    }

    fn resolve(inner: &mut Inner, trigger: Trigger) -> Option<SegmentEvent> {
        let pending = inner.pending.take()?;
        Some(SegmentEvent::AppOpened(SegmentEventAppOpened {
            start_kind: match pending.start {
                Start::Cold => "cold",
                Start::Warm => "warm",
            }
            .to_string(),
            trigger: trigger.name().to_string(),
            prev_session_id: pending.prev_session_id,
            seconds_since_last_seen: pending.seconds_since,
        }))
    }

    fn write_mark(&self, now_unix: i64, now_ms: u64) {
        let mark = Mark {
            session_id: self.session_id.clone(),
            last_seen: now_unix,
        };
        if let Err(error) = self.store.save(&mark) {
            tracing::warn!("session mark not saved: {error}");
        }
        let mut inner = self.inner();
        inner.prev = mark;
        inner.next_heartbeat_ms = now_ms + HEARTBEAT_MS;
    }
}

/// What a routed deep link says brought the player in; a push wins over a plain link.
pub fn trigger_of(effects: &[crate::deep_link_route::Effect]) -> Option<Trigger> {
    use crate::deep_link_route::{AnalyticsEvent, Effect};
    let mut trigger = None;
    for effect in effects {
        match effect {
            Effect::Analytics {
                event: AnalyticsEvent::PushOpened { .. },
            } => return Some(Trigger::Push),
            Effect::Analytics {
                event: AnalyticsEvent::DeepLinkReceived { .. },
            } => trigger = Some(Trigger::Deeplink),
            _ => {}
        }
    }
    trigger
}

/// Milliseconds on a clock that only moves forward, for the settle and stash windows.
pub fn monotonic_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The process-wide tracker, made on first use from the installed analytics and the core's data
/// dir; `None` before the analytics is installed.
pub fn global(core: &crate::Core) -> Option<&'static SessionTracker> {
    static TRACKER: std::sync::OnceLock<SessionTracker> = std::sync::OnceLock::new();
    if let Some(tracker) = TRACKER.get() {
        return Some(tracker);
    }
    let analytics = super::global::analytics()?;
    Some(TRACKER.get_or_init(|| {
        SessionTracker::new(
            analytics.session_id(),
            Box::new(FileMarkStore::new(
                core.data_dir().join("session_state.json"),
            )),
        )
    }))
}

/// Queues the event on the installed analytics.
pub fn emit(event: Option<SegmentEvent>) {
    if let (Some(event), Some(analytics)) = (event, super::global::analytics()) {
        analytics.queue_event(event);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[derive(Default)]
    struct Memory(Mutex<Option<Mark>>);

    impl MarkStore for Arc<Memory> {
        fn load(&self) -> Option<Mark> {
            self.0.lock().unwrap().clone()
        }
        fn save(&self, mark: &Mark) -> io::Result<()> {
            *self.0.lock().unwrap() = Some(mark.clone());
            Ok(())
        }
    }

    fn tracker(previous: Option<Mark>) -> (SessionTracker, Arc<Memory>) {
        let memory = Arc::new(Memory(Mutex::new(previous)));
        (
            SessionTracker::new("now".into(), Box::new(memory.clone())),
            memory,
        )
    }

    fn opened(event: Option<SegmentEvent>) -> (String, String, String, i64) {
        let Some(SegmentEvent::AppOpened(e)) = event else {
            panic!("expected an App Opened event");
        };
        (
            e.start_kind,
            e.trigger,
            e.prev_session_id,
            e.seconds_since_last_seen,
        )
    }

    fn previous(last_seen: i64) -> Option<Mark> {
        Some(Mark {
            session_id: "before".into(),
            last_seen,
        })
    }

    #[test]
    fn a_cold_launch_with_no_link_is_an_icon_open_after_the_settle_window() {
        let (tracker, _) = tracker(previous(1_000));
        assert!(tracker.launched(4_600, 0).is_none());
        assert!(tracker.tick(4_600, DEEP_LINK_SETTLE_MS - 1).is_none());
        assert_eq!(
            opened(tracker.tick(4_601, DEEP_LINK_SETTLE_MS)),
            ("cold".into(), "icon".into(), "before".into(), 3_600)
        );
        assert!(
            tracker.tick(4_602, DEEP_LINK_SETTLE_MS + 1).is_none(),
            "once"
        );
    }

    #[test]
    fn a_link_during_the_open_names_what_brought_the_player_in() {
        let (tracker, _) = tracker(previous(1_000));
        tracker.launched(2_000, 0);
        assert_eq!(opened(tracker.deep_link(Trigger::Push, 400)).1, "push");
        assert!(tracker.tick(2_005, 5_000).is_none(), "already resolved");
    }

    #[test]
    fn a_link_that_beat_the_open_by_a_moment_still_counts() {
        let (tracker, _) = tracker(previous(1_000));
        assert!(tracker.deep_link(Trigger::Deeplink, 100).is_none());
        assert_eq!(
            opened(tracker.launched(2_000, 300)).1,
            "deeplink",
            "stashed 200 ms before"
        );
    }

    #[test]
    fn a_stale_link_never_attributes_a_later_open() {
        let (tracker, _) = tracker(previous(1_000));
        tracker.deep_link(Trigger::Deeplink, 0);
        tracker.tick(1_500, 600);
        assert!(tracker.launched(2_000, 700).is_none());
        assert_eq!(
            opened(tracker.tick(2_002, 700 + DEEP_LINK_SETTLE_MS)).1,
            "icon"
        );
    }

    #[test]
    fn a_warm_return_inside_the_floor_is_not_an_open_and_after_it_is() {
        let (tracker, _) = tracker(None);
        tracker.launched(10_000, 0);
        tracker.tick(10_002, DEEP_LINK_SETTLE_MS);
        tracker.backgrounded(10_010, 10_000);
        assert!(
            tracker.foregrounded(10_020, 20_000).is_none(),
            "10 s: its own dialog"
        );
        tracker.backgrounded(10_020, 20_000);
        assert!(
            tracker.foregrounded(10_100, 90_000).is_none(),
            "waits for a link first"
        );
        let (start, trigger, prev, since) =
            opened(tracker.tick(10_102, 90_000 + DEEP_LINK_SETTLE_MS));
        assert_eq!((start.as_str(), trigger.as_str()), ("warm", "icon"));
        assert_eq!(
            prev, "now",
            "a warm open measures against this same session"
        );
        assert_eq!(since, 80);
    }

    #[test]
    fn the_first_launch_ever_and_a_clock_that_went_back_read_as_unknown() {
        let (first, _) = tracker(None);
        first.launched(5_000, 0);
        assert_eq!(opened(first.tick(5_001, DEEP_LINK_SETTLE_MS)).3, -1);
        let (back, _) = tracker(previous(9_000));
        back.launched(5_000, 0);
        assert_eq!(opened(back.tick(5_001, DEEP_LINK_SETTLE_MS)).3, -1);
    }

    #[test]
    fn the_mark_is_written_at_launch_on_the_heartbeat_and_when_leaving() {
        let (tracker, memory) = tracker(previous(1_000));
        tracker.launched(2_000, 0);
        assert_eq!(memory.load().unwrap().last_seen, 2_000);
        tracker.tick(2_010, HEARTBEAT_MS - 1);
        assert_eq!(
            memory.load().unwrap().last_seen,
            2_000,
            "not before the beat"
        );
        tracker.tick(2_031, HEARTBEAT_MS);
        assert_eq!(memory.load().unwrap().last_seen, 2_031);
        tracker.backgrounded(2_100, HEARTBEAT_MS + 5);
        let mark = memory.load().unwrap();
        assert_eq!((mark.session_id.as_str(), mark.last_seen), ("now", 2_100));
    }

    #[test]
    fn a_push_wins_over_a_link_and_other_effects_say_nothing() {
        use crate::deep_link_route::{AnalyticsEvent, Effect, StartKind};
        let link = Effect::Analytics {
            event: AnalyticsEvent::DeepLinkReceived { params: vec![] },
        };
        let push = Effect::Analytics {
            event: AnalyticsEvent::PushOpened {
                campaign_id: "c".into(),
                push_id: "p".into(),
                start_kind: StartKind::Cold,
            },
        };
        assert_eq!(
            trigger_of(std::slice::from_ref(&link)),
            Some(Trigger::Deeplink)
        );
        assert_eq!(trigger_of(&[link, push]), Some(Trigger::Push));
        assert_eq!(trigger_of(&[Effect::MarketplaceReturn]), None);
        assert_eq!(trigger_of(&[]), None);
    }

    #[test]
    fn the_file_store_round_trips_and_ignores_a_torn_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session_state.json");
        let store = FileMarkStore::new(path.clone());
        assert!(store.load().is_none());
        store
            .save(&Mark {
                session_id: "s".into(),
                last_seen: 7,
            })
            .unwrap();
        assert_eq!(store.load().unwrap().last_seen, 7);
        fs::write(&path, "{ torn").unwrap();
        assert!(store.load().is_none());
    }
}
