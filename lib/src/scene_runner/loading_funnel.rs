//! Loading funnel — pure, well-typed accumulator for the whole loading pipeline
//! (realm resolution → scene discovery → spawn → per-scene tick / GLTF render →
//! loading-screen dismissal), emitted as a single Segment `"Loading Event"` per load.
//!
//! This is the Rust home of what used to be `godot/src/logic/loading_profiler.gd`. It holds
//! **no Godot objects** and does **no async/await**: it is a plain state machine that
//! `DclSceneManager` drives from the loading-session lifecycle it already owns, then hands the
//! built [`SegmentEventLoading`] payloads back to the caller to emit. That keeps it unit-testable
//! and off the GDScript↔Rust JSON round-trip the old profiler needed.
//!
//! One "load" == one loading-screen-bounded episode. Every event of a load shares its
//! `loading_id`, so the funnel reassembles in Segment by grouping on it. A load can span more
//! than one [`LoadingSession`](super::loading_session::LoadingSession) (a realm change mid-load
//! supersedes the previous one); the funnel accumulates across the whole episode.

use std::time::Instant;

use crate::analytics::data_definition::{LoadingPhaseBreakdown, SegmentEventLoading};

use super::loading_session::LoadingPhase;

/// Context captured at load begin (all the values that require Godot access, read once by the
/// caller and passed in so the funnel stays Godot-free).
pub struct LoadingBeginContext {
    /// Navigation entry point: "on_teleport" | "on_join_world" | "auto" | "".
    pub when: String,
    /// Full realm string (bucketed to a coarse class before it leaves the funnel — never sent raw).
    pub realm: String,
    /// `count_loaded_resources()` baseline — used for `cold_cache` and the `assets_loaded` delta.
    pub res_loaded: i64,
    /// `count_loading_resources()` baseline — used for the `assets_expected` delta.
    pub res_loading: i64,
    /// Current download speed estimate (Mbps).
    pub network_mbs: f64,
    /// Device CPU count.
    pub cpu_count: i32,
    /// OS name.
    pub platform: String,
    /// `Engine.get_process_frames()` at begin — baseline for the rendered-`frames` count.
    pub process_frames: i64,
}

/// A pure loading-funnel accumulator. See the module docs.
#[derive(Default)]
pub struct LoadingFunnel {
    /// Monotonic id source; each load gets the next value as its `loading_id`.
    next_id: u64,
    /// `loading_id` of the current (or most recent) load.
    id: u64,
    active: bool,
    start: Option<Instant>,
    start_frame: i64,
    when: String,
    realm: String,

    // Baselines captured at begin, for per-load asset deltas.
    res_loaded0: i64,
    res_loading0: i64,

    // Milestone timestamps: ms since load begin, first occurrence; -1 == never happened.
    first_spawn_ms: i64,
    first_ready_ms: i64,
    complete_ms: i64,
    scene_rendered_ms: i64,
    about_end_ms: i64,
    discovery_ms: i64,

    /// Entry ms of each loading phase (first entry), -1 == never reached.
    phase_breakdown: LoadingPhaseBreakdown,

    // 25–30% plateau band accumulation (the F-7 "looks frozen" signal).
    band_ms: i64,
    band_enter: Option<Instant>,

    /// Peak `loading - loaded` resource count seen during the load.
    peak_pending: i64,
    /// Asset-group failures reported during the load (pushed from the GLTF coordinator).
    asset_failures: i64,
    /// Whether a realm-change failure was observed during this load (feeds `dismissed_by=error`).
    saw_realm_fail: bool,
}

impl LoadingFunnel {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// `loading_id` of the current/most-recent load (0 before the first load begins).
    pub fn current_id(&self) -> u64 {
        self.id
    }

    /// Begin a new load. The caller must `end("superseded")` first if one is already active
    /// (mirrors the old profiler's auto-close). Returns the `type="started"` event to emit.
    pub fn begin(&mut self, ctx: LoadingBeginContext, now: Instant) -> SegmentEventLoading {
        self.next_id += 1;
        self.id = self.next_id;
        self.active = true;
        self.start = Some(now);
        self.start_frame = ctx.process_frames;
        self.when = if ctx.when.is_empty() {
            "-".to_string()
        } else {
            ctx.when.clone()
        };
        self.realm = ctx.realm.clone();
        self.res_loaded0 = ctx.res_loaded;
        self.res_loading0 = ctx.res_loading;

        self.first_spawn_ms = -1;
        self.first_ready_ms = -1;
        self.complete_ms = -1;
        self.scene_rendered_ms = -1;
        self.about_end_ms = -1;
        self.discovery_ms = -1;
        self.phase_breakdown = LoadingPhaseBreakdown::default();
        self.band_ms = 0;
        self.band_enter = None;
        self.peak_pending = 0;
        self.asset_failures = 0;
        self.saw_realm_fail = false;

        SegmentEventLoading {
            when: Some(self.when.clone()),
            is_world: Some(is_dcl_ens(&self.realm)),
            cold_cache: Some(ctx.res_loaded == 0),
            network_estimate_mbs: Some(ctx.network_mbs),
            cpu_count: Some(ctx.cpu_count),
            platform: Some(ctx.platform),
            ..SegmentEventLoading::base("started", self.id, realm_bucket(&self.realm))
        }
    }

    /// End the current load and produce its funnel event(s): the `type="completed"` event, plus a
    /// `type="asset_failure"` aggregate when any asset group failed. Empty if no load is active.
    pub fn end(
        &mut self,
        reason: &str,
        res_loaded: i64,
        res_loading: i64,
        process_frames: i64,
        now: Instant,
    ) -> Vec<SegmentEventLoading> {
        if !self.active {
            return Vec::new();
        }
        // Close an open plateau-band interval so its time is counted.
        if let Some(enter) = self.band_enter.take() {
            self.band_ms += ms_between(enter, now);
        }

        let duration_ms = self.elapsed_ms(now);
        let reached_ready = self.first_ready_ms >= 0;
        let dismissed_by = self.classify_dismissal(reason, reached_ready);
        let assets_expected = (res_loading - self.res_loading0).max(0);
        let assets_loaded = (res_loaded - self.res_loaded0).max(0);
        let assets_unfinished = (res_loading - res_loaded).max(0);
        let bucket = realm_bucket(&self.realm);

        let completed = SegmentEventLoading {
            duration_ms: Some(duration_ms),
            reached_ready: Some(reached_ready),
            dismissed_by: Some(dismissed_by),
            first_spawn_ms: Some(self.first_spawn_ms),
            first_ready_ms: Some(self.first_ready_ms),
            complete_ms: Some(self.complete_ms),
            scene_rendered_ms: Some(self.scene_rendered_ms),
            about_end_ms: Some(self.about_end_ms),
            discovery_ms: Some(self.discovery_ms),
            time_in_25_30_band_ms: Some(self.band_ms),
            phase_breakdown: Some(self.phase_breakdown.clone()),
            assets_expected: Some(assets_expected),
            assets_loaded: Some(assets_loaded),
            assets_unfinished: Some(assets_unfinished),
            assets_errored: Some(self.asset_failures),
            peak_pending_assets: Some(self.peak_pending),
            frames: Some((process_frames - self.start_frame).max(0)),
            ..SegmentEventLoading::base("completed", self.id, bucket.clone())
        };

        let mut out = vec![completed];
        if self.asset_failures > 0 {
            out.push(SegmentEventLoading {
                reason: Some("gltf_group_failed".to_string()),
                count: Some(self.asset_failures),
                ..SegmentEventLoading::base("asset_failure", self.id, bucket)
            });
        }

        self.active = false;
        out
    }

    /// Build a `type="realm_change_failed"` event (may fire outside an active load; correlated to
    /// the current/most-recent `loading_id`). Marks the active load so it ends as `error`.
    pub fn realm_change_failed(&mut self, realm: &str, reason: &str) -> SegmentEventLoading {
        if self.active {
            self.saw_realm_fail = true;
        }
        SegmentEventLoading {
            reason: Some(sanitize_reason(reason)),
            ..SegmentEventLoading::base("realm_change_failed", self.id, realm_bucket(realm))
        }
    }

    // --- session-driven marks (fed by DclSceneManager) ---------------------------------------

    pub fn on_phase(&mut self, phase: LoadingPhase, now: Instant) {
        if !self.active {
            return;
        }
        let ms = self.elapsed_ms(now);
        let pb = &mut self.phase_breakdown;
        let slot = match phase {
            LoadingPhase::Metadata => &mut pb.metadata,
            LoadingPhase::Spawning => &mut pb.spawning,
            LoadingPhase::Assets => &mut pb.assets,
            LoadingPhase::Ready => &mut pb.ready,
            LoadingPhase::FloatingIslands => &mut pb.floating_islands,
            LoadingPhase::Done => &mut pb.done,
            LoadingPhase::Idle => return,
        };
        set_once(slot, ms);
    }

    /// `percent` in 0..100, `ready` = scenes counted ready. Accumulates the 25–30% band and
    /// records the first time any scene is ready.
    pub fn on_progress(&mut self, percent: f32, ready: i32, now: Instant) {
        if !self.active {
            return;
        }
        let in_band = (25.0..30.0).contains(&percent);
        match (in_band, self.band_enter) {
            (true, None) => self.band_enter = Some(now),
            (false, Some(enter)) => {
                self.band_ms += ms_between(enter, now);
                self.band_enter = None;
            }
            _ => {}
        }
        if ready > 0 {
            let ms = self.elapsed_ms(now);
            set_once(&mut self.first_ready_ms, ms);
        }
    }

    pub fn on_scene_spawned(&mut self, now: Instant) {
        self.mark(MilestoneSpawn, now);
    }

    pub fn on_complete(&mut self, now: Instant) {
        self.mark(MilestoneComplete, now);
    }

    pub fn on_scene_rendered(&mut self, now: Instant) {
        self.mark(MilestoneRendered, now);
    }

    pub fn mark_about_end(&mut self, now: Instant) {
        self.mark(MilestoneAboutEnd, now);
    }

    pub fn mark_discovery(&mut self, now: Instant) {
        self.mark(MilestoneDiscovery, now);
    }

    /// Track the peak `loading - loaded` resource count (polled each tick while active).
    pub fn note_pending(&mut self, pending: i64) {
        if self.active && pending > self.peak_pending {
            self.peak_pending = pending;
        }
    }

    /// Accumulate asset-group failures reported by the GLTF coordinator.
    pub fn note_asset_failure(&mut self, count: i64) {
        if self.active && count > 0 {
            self.asset_failures += count;
        }
    }

    // --- internals ---------------------------------------------------------------------------

    fn mark(&mut self, which: Milestone, now: Instant) {
        if !self.active {
            return;
        }
        let ms = self.elapsed_ms(now);
        let slot = match which {
            MilestoneSpawn => &mut self.first_spawn_ms,
            MilestoneComplete => &mut self.complete_ms,
            MilestoneRendered => &mut self.scene_rendered_ms,
            MilestoneAboutEnd => &mut self.about_end_ms,
            MilestoneDiscovery => &mut self.discovery_ms,
        };
        set_once(slot, ms);
    }

    fn elapsed_ms(&self, now: Instant) -> i64 {
        match self.start {
            Some(start) => ms_between(start, now),
            None => 0,
        }
    }

    /// How the load ended. `wall_clock_timeout` with `reached_ready=false` is the infinite-loading
    /// signal. Mirrors the old profiler's `_classify_dismissal`.
    fn classify_dismissal(&self, reason: &str, reached_ready: bool) -> String {
        let r = reason.to_lowercase();
        if r.contains("supersed") {
            "superseded".to_string()
        } else if r.contains("cancel") {
            "cancelled".to_string()
        } else if self.saw_realm_fail || r.contains("error") || r.contains("fail") {
            "error".to_string()
        } else if r.contains("timeout") {
            "wall_clock_timeout".to_string()
        } else if reached_ready {
            "completion".to_string()
        } else {
            "wall_clock_timeout".to_string()
        }
    }
}

// A tiny milestone selector so the several one-shot marks share one implementation.
#[derive(Clone, Copy)]
enum Milestone {
    Spawn,
    Complete,
    Rendered,
    AboutEnd,
    Discovery,
}
use Milestone::{
    AboutEnd as MilestoneAboutEnd, Complete as MilestoneComplete, Discovery as MilestoneDiscovery,
    Rendered as MilestoneRendered, Spawn as MilestoneSpawn,
};

fn set_once(slot: &mut i64, ms: i64) {
    if *slot < 0 {
        *slot = ms;
    }
}

fn ms_between(a: Instant, b: Instant) -> i64 {
    b.saturating_duration_since(a).as_millis() as i64
}

/// `^[a-zA-Z0-9]+\.dcl\.eth$` — matches `Realm.is_dcl_ens` in realm.gd.
fn is_dcl_ens(realm: &str) -> bool {
    match realm.strip_suffix(".dcl.eth") {
        Some(name) => !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric()),
        None => false,
    }
}

/// Coarse realm class (no full URL / PII): "world" | "genesis" | "other" | "unknown".
fn realm_bucket(realm: &str) -> String {
    if realm.is_empty() || realm == "-" {
        return "unknown".to_string();
    }
    if is_dcl_ens(realm) {
        return "world".to_string();
    }
    let low = realm.to_lowercase();
    if low.contains("genesis") || low.contains("peer.decentraland") || low == "main" {
        return "genesis".to_string();
    }
    "other".to_string()
}

/// Bound a diagnostic reason string so a load event can never carry an unbounded blob.
fn sanitize_reason(reason: &str) -> String {
    const MAX: usize = 160;
    let trimmed = reason.trim();
    if trimmed.chars().count() <= MAX {
        trimmed.to_string()
    } else {
        trimmed.chars().take(MAX).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ctx(when: &str, realm: &str, res_loaded: i64, res_loading: i64) -> LoadingBeginContext {
        LoadingBeginContext {
            when: when.to_string(),
            realm: realm.to_string(),
            res_loaded,
            res_loading,
            network_mbs: 10.0,
            cpu_count: 8,
            platform: "macOS".to_string(),
            process_frames: 100,
        }
    }

    #[test]
    fn realm_bucket_classes() {
        assert_eq!(realm_bucket("myname.dcl.eth"), "world");
        assert_eq!(realm_bucket("https://peer.decentraland.org"), "genesis");
        assert_eq!(realm_bucket("main"), "genesis");
        assert_eq!(realm_bucket("https://custom.example/realm"), "other");
        assert_eq!(realm_bucket(""), "unknown");
        assert_eq!(realm_bucket("-"), "unknown");
    }

    #[test]
    fn is_dcl_ens_matches_regex() {
        assert!(is_dcl_ens("abc123.dcl.eth"));
        assert!(!is_dcl_ens(".dcl.eth"));
        assert!(!is_dcl_ens("has-dash.dcl.eth"));
        assert!(!is_dcl_ens("sub.name.dcl.eth"));
        assert!(!is_dcl_ens("name.dcl.eth.evil"));
    }

    #[test]
    fn started_event_shape() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let ev = f.begin(ctx("on_teleport", "genesis city", 0, 3), t0);
        assert_eq!(ev.event_type, "started");
        assert_eq!(ev.loading_id, 1);
        assert_eq!(ev.realm_bucket, "genesis");
        assert_eq!(ev.when.as_deref(), Some("on_teleport"));
        assert_eq!(ev.cold_cache, Some(true));
        assert_eq!(ev.is_world, Some(false));
        assert!(f.is_active());
    }

    #[test]
    fn completed_funnel_and_ids_correlate() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let started = f.begin(ctx("auto", "world.dcl.eth", 5, 5), t0);

        f.on_phase(LoadingPhase::Metadata, t0 + Duration::from_millis(10));
        f.on_scene_spawned(t0 + Duration::from_millis(50));
        f.on_phase(LoadingPhase::Assets, t0 + Duration::from_millis(80));
        f.on_progress(27.0, 0, t0 + Duration::from_millis(100)); // enter band
        f.on_progress(28.0, 1, t0 + Duration::from_millis(600)); // still in band, first ready
        f.on_progress(55.0, 2, t0 + Duration::from_millis(900)); // leave band (+800ms)
        f.on_complete(t0 + Duration::from_millis(1000));

        let events = f.end("hidden", 12, 12, 260, t0 + Duration::from_millis(1200));
        assert_eq!(events.len(), 1); // no asset failures
        let c = &events[0];
        assert_eq!(c.event_type, "completed");
        assert_eq!(c.loading_id, started.loading_id); // same load correlates
        assert_eq!(c.reached_ready, Some(true));
        assert_eq!(c.dismissed_by.as_deref(), Some("completion"));
        assert_eq!(c.duration_ms, Some(1200));
        assert_eq!(c.first_spawn_ms, Some(50));
        assert_eq!(c.first_ready_ms, Some(600));
        assert_eq!(c.time_in_25_30_band_ms, Some(800));
        assert_eq!(c.assets_expected, Some(7)); // 12 - 5
        assert_eq!(c.assets_loaded, Some(7)); // 12 - 5
        assert_eq!(c.frames, Some(160)); // 260 - 100
        assert!(!f.is_active());
    }

    #[test]
    fn timeout_without_ready_is_infinite_loading_signal() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("auto", "main", 0, 0), t0);
        let events = f.end("timeout", 0, 4, 100, t0 + Duration::from_secs(90));
        let c = &events[0];
        assert_eq!(c.reached_ready, Some(false));
        assert_eq!(c.dismissed_by.as_deref(), Some("wall_clock_timeout"));
    }

    #[test]
    fn asset_failures_emit_second_event_with_same_id() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let started = f.begin(ctx("auto", "main", 0, 0), t0);
        f.note_asset_failure(2);
        f.note_asset_failure(1);
        let events = f.end("hidden", 1, 1, 10, t0 + Duration::from_millis(500));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].assets_errored, Some(3));
        assert_eq!(events[1].event_type, "asset_failure");
        assert_eq!(events[1].count, Some(3));
        assert_eq!(events[1].loading_id, started.loading_id);
    }

    #[test]
    fn realm_fail_marks_error_dismissal() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("auto", "main", 0, 0), t0);
        let rf = f.realm_change_failed("bad.example", "invalid /about response");
        assert_eq!(rf.event_type, "realm_change_failed");
        assert_eq!(rf.loading_id, 1);
        let events = f.end("hidden", 0, 0, 10, t0 + Duration::from_millis(200));
        assert_eq!(events[0].dismissed_by.as_deref(), Some("error"));
    }

    #[test]
    fn end_without_begin_is_noop() {
        let mut f = LoadingFunnel::default();
        assert!(f.end("hidden", 0, 0, 0, Instant::now()).is_empty());
    }
}
