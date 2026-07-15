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

/// Below this throughput a tick with assets outstanding counts as a stalled download. The
/// ContentProvider reports whole-MB volume per second, so anything under a few KB/s is "nothing
/// is arriving" rather than "slow but progressing".
const NET_STALL_MB_S: f64 = 0.005;

/// Context captured at load begin (all the values that require Godot access, read once by the
/// caller and passed in so the funnel stays Godot-free).
pub struct LoadingBeginContext {
    /// Navigation entry point: "on_teleport" | "on_join_world" | "auto" | "".
    pub when: String,
    /// Full realm string (bucketed to a coarse class before it leaves the funnel — never sent raw).
    pub realm: String,
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

    // Milestone timestamps: ms since load begin, first occurrence; -1 == never happened.
    first_spawn_ms: i64,
    complete_ms: i64,
    scene_rendered_ms: i64,
    about_end_ms: i64,
    discovery_ms: i64,

    /// Entry ms of each loading phase (first entry), -1 == never reached.
    phase_breakdown: LoadingPhaseBreakdown,

    // 25–30% plateau band accumulation (the F-7 "looks frozen" signal).
    band_ms: i64,
    band_enter: Option<Instant>,

    /// Peak process-wide `loading - loaded` resource backlog seen during the load (not per-load —
    /// the ContentProvider counters are global; see `process_assets_pending_peak` in the schema).
    peak_pending: i64,
    // Network throughput accumulated across the load (MB/s samples from the ContentProvider's
    // 1-second download window). The begin-time sample alone is blind to the actual conditions:
    // a load that starts before any fetch is in flight reads 0 (observed on-device).
    net_sum_mb_s: f64,
    net_samples: i64,
    net_peak_mb_s: f64,
    net_stall_ms: i64,
    last_tick: Option<Instant>,
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

        self.first_spawn_ms = -1;
        self.complete_ms = -1;
        self.scene_rendered_ms = -1;
        self.about_end_ms = -1;
        self.discovery_ms = -1;
        self.phase_breakdown = LoadingPhaseBreakdown::default();
        self.band_ms = 0;
        self.band_enter = None;
        self.peak_pending = 0;
        self.net_sum_mb_s = 0.0;
        self.net_samples = 0;
        self.net_peak_mb_s = 0.0;
        self.net_stall_ms = 0;
        self.last_tick = None;
        self.asset_failures = 0;
        self.saw_realm_fail = false;

        SegmentEventLoading {
            when: Some(self.when.clone()),
            is_world: Some(is_world_realm(&self.realm)),
            cpu_count: Some(ctx.cpu_count),
            platform: Some(ctx.platform),
            ..SegmentEventLoading::base("started", self.id, realm_bucket(&self.realm))
        }
    }

    /// End the current load and produce its funnel event(s): the `type="completed"` event, plus a
    /// `type="asset_failure"` aggregate when any asset group failed. Empty if no load is active.
    ///
    /// `realm` is re-read by the caller at end time and is what `realm_bucket` is computed from.
    /// The realm known at begin is the one being *left*: the loading screen only forwards an
    /// intended realm for worlds (a dcl ENS), so any other destination — Genesis above all — falls
    /// back to the current realm and would bucket as wherever the user came from. Verified
    /// on-device: a deadsurge.dcl.eth -> Genesis teleport bucketed as `world`. By end time the
    /// realm has resolved to the destination, so this is the only trustworthy source.
    pub fn end(
        &mut self,
        reason: &str,
        realm: &str,
        res_loaded: i64,
        res_loading: i64,
        process_frames: i64,
        now: Instant,
    ) -> Vec<SegmentEventLoading> {
        if !self.active {
            return Vec::new();
        }
        if !realm.is_empty() {
            self.realm = realm.to_string();
        }
        // Close an open plateau-band interval so its time is counted.
        if let Some(enter) = self.band_enter.take() {
            self.band_ms += ms_between(enter, now);
        }

        let duration_ms = self.elapsed_ms(now);
        // Reaching the `done` phase is the only trustworthy completion signal: the session is
        // dropped the instant it completes, so any milestone that depends on a later tick is
        // unreliable. `complete_ms` is set from the phase transition itself and was -1 on exactly
        // the 7 non-completed loads across every on-device capture.
        let completed = self.complete_ms >= 0;
        let dismissed_by = self.classify_dismissal(reason, completed);
        // Process-wide, not this load's: both counters are cumulative `fetch_add`-only totals for
        // the whole process, so their difference is the current global backlog rather than a delta
        // this episode owns. Named for what it is; see the schema docs.
        let pending_at_end = (res_loading - res_loaded).max(0);
        let bucket = realm_bucket(&self.realm);

        let completed = SegmentEventLoading {
            // Carried here too (not just on `started`) because this is the one computed from the
            // resolved destination realm — on `started` it can only describe where the load came
            // from. This is the authoritative pair for segmenting a load.
            is_world: Some(is_world_realm(&self.realm)),
            // Repeated from `started` so this event stands alone: `when="auto"` marks a background
            // streaming episode (walking into a parcel, no loading screen), which is not a
            // user-visible load and would skew any completion-rate aggregate. Without it here,
            // excluding those means joining back to `started` on `loading_id`.
            when: Some(self.when.clone()),
            duration_ms: Some(duration_ms),
            dismissed_by: Some(dismissed_by),
            first_spawn_ms: Some(self.first_spawn_ms),
            complete_ms: Some(self.complete_ms),
            scene_rendered_ms: Some(self.scene_rendered_ms),
            about_end_ms: Some(self.about_end_ms),
            discovery_ms: Some(self.discovery_ms),
            time_in_25_30_band_ms: Some(self.band_ms),
            phase_breakdown: Some(self.phase_breakdown.clone()),
            process_assets_pending_at_end: Some(pending_at_end),
            process_assets_pending_peak: Some(self.peak_pending),
            assets_errored: Some(self.asset_failures),
            network_avg_mb_s: Some(self.net_avg_mb_s()),
            network_peak_mb_s: Some(self.net_peak_mb_s),
            network_stall_ms: Some(self.net_stall_ms),
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
    pub fn on_progress(&mut self, percent: f32, now: Instant) {
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

    /// Poll from the scene tick while a load is active: peak pending assets plus network
    /// throughput (mean/peak) and stalled-download time. `speed_mb_s` is the ContentProvider's
    /// last-second download volume in MB/s; `pending` is `loading - loaded`.
    pub fn note_tick(&mut self, pending: i64, speed_mb_s: f64, now: Instant) {
        if !self.active {
            self.last_tick = None;
            return;
        }
        if pending > self.peak_pending {
            self.peak_pending = pending;
        }
        self.net_sum_mb_s += speed_mb_s;
        self.net_samples += 1;
        if speed_mb_s > self.net_peak_mb_s {
            self.net_peak_mb_s = speed_mb_s;
        }
        // Assets outstanding but nothing arriving == stalled fetches. Attribute the elapsed tick
        // to stall time; the first tick of a load has no predecessor to measure against.
        if let Some(prev) = self.last_tick {
            if pending > 0 && speed_mb_s < NET_STALL_MB_S {
                self.net_stall_ms += ms_between(prev, now);
            }
        }
        self.last_tick = Some(now);
    }

    fn net_avg_mb_s(&self) -> f64 {
        if self.net_samples > 0 {
            self.net_sum_mb_s / self.net_samples as f64
        } else {
            0.0
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

    /// How the load ended. `wall_clock_timeout` == the loading screen was dismissed while the
    /// pipeline never reached `done` (the infinite / abandoned-load signal). A load that reached
    /// `done` is `completion` even if the ready-progress milestone was skipped. Precedence:
    /// superseded/cancelled/error override completion. Corrected after on-device validation.
    fn classify_dismissal(&self, reason: &str, completed: bool) -> String {
        let r = reason.to_lowercase();
        if r.contains("supersed") {
            "superseded".to_string()
        } else if r.contains("cancel") {
            "cancelled".to_string()
        } else if self.saw_realm_fail || r.contains("error") || r.contains("fail") {
            "error".to_string()
        } else if completed {
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

/// Coarse realm class (no full URL / PII): "world" | "genesis" | "other" | "unknown".
/// Whether a realm string names a world. Both shapes the funnel sees end the same way once the
/// trailing slash is gone — the resolved URL
/// (`https://worlds-content-server.decentraland.org/world/deadsurge.dcl.eth/`) and the bare ENS
/// the loading screen forwards (`deadsurge.dcl.eth`) — so the suffix decides it without picking
/// the string apart.
fn is_world_realm(realm: &str) -> bool {
    realm
        .trim()
        .trim_end_matches('/')
        .to_lowercase()
        .ends_with(".dcl.eth")
}

/// Coarse realm class, matched against the realm strings actually observed on-device rather than
/// the short forms: Genesis resolves to `https://realm-provider-ea.decentraland.org/main/`, which
/// contains neither "genesis" nor "peer.decentraland" nor equals "main".
fn realm_bucket(realm: &str) -> String {
    let low = realm.trim().trim_end_matches('/').to_lowercase();
    if low.is_empty() || low == "-" || low == "no-realm" {
        return "unknown".to_string();
    }
    if is_world_realm(&low) {
        return "world".to_string();
    }
    if low.contains("genesis")
        || low.contains("peer.decentraland")
        || low.contains("realm-provider")
        || low == "main"
        || low.ends_with("/main")
    {
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

    fn ctx(when: &str, realm: &str) -> LoadingBeginContext {
        LoadingBeginContext {
            when: when.to_string(),
            realm: realm.to_string(),
            cpu_count: 8,
            platform: "macOS".to_string(),
            process_frames: 100,
        }
    }

    /// The throttled-connection case: downloads crawl, then stop arriving entirely while work is
    /// still outstanding. The averaged/stall fields must show it, and stalled fetches must not be
    /// mistaken for errors — they never fail, so `assets_errored` stays 0.
    #[test]
    fn throttled_load_reports_avg_peak_and_stall() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("on_teleport", "genesis city"), t0);

        // A tick with data arriving, then two seconds with work outstanding and nothing coming.
        f.note_tick(10, 0.5, t0 + Duration::from_millis(100));
        f.note_tick(10, 0.1, t0 + Duration::from_millis(1100));
        f.note_tick(10, 0.0, t0 + Duration::from_millis(2100));
        f.note_tick(10, 0.0, t0 + Duration::from_millis(3100));

        let out = f.end("hidden", "", 4, 10, 200, t0 + Duration::from_millis(4000));
        let c = &out[0];
        assert_eq!(c.network_peak_mb_s, Some(0.5));
        assert_eq!(c.network_avg_mb_s, Some(0.15)); // (0.5 + 0.1 + 0 + 0) / 4
        assert_eq!(c.network_stall_ms, Some(2000)); // the two zero-throughput ticks
        assert_eq!(c.process_assets_pending_peak, Some(10));
        assert_eq!(c.process_assets_pending_at_end, Some(6)); // 10 requested - 4 completed
        assert_eq!(c.assets_errored, Some(0));
    }

    /// Stall time must only accrue while something is actually outstanding — an idle-but-complete
    /// load reports zero throughput too, and must not be smeared as a stall.
    #[test]
    fn no_pending_assets_is_not_a_stall() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("on_teleport", "genesis city"), t0);
        f.note_tick(0, 0.0, t0 + Duration::from_millis(1000));
        f.note_tick(0, 0.0, t0 + Duration::from_millis(2000));
        let out = f.end("hidden", "", 0, 0, 200, t0 + Duration::from_millis(3000));
        assert_eq!(out[0].network_stall_ms, Some(0));
    }

    /// `when` must ride along on `completed` so background streaming episodes are excludable
    /// without joining back to `started` on `loading_id`.
    #[test]
    fn completed_carries_when_for_filtering_auto_episodes() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(
            ctx("auto", "https://realm-provider-ea.decentraland.org/main/"),
            t0,
        );
        let out = f.end("superseded", "", 1, 1, 10, t0 + Duration::from_millis(500));
        assert_eq!(out[0].when.as_deref(), Some("auto"));
        assert_eq!(out[0].dismissed_by.as_deref(), Some("superseded"));
    }

    /// Counters must not bleed across loads: a throttled load followed by a fast one.
    #[test]
    fn network_counters_reset_between_loads() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("on_teleport", "genesis city"), t0);
        f.note_tick(5, 0.0, t0 + Duration::from_millis(1000));
        f.note_tick(5, 0.0, t0 + Duration::from_millis(2000));
        f.end("hidden", "", 0, 5, 200, t0 + Duration::from_millis(3000));

        let t1 = t0 + Duration::from_millis(4000);
        f.begin(ctx("on_teleport", "genesis city"), t1);
        f.note_tick(1, 2.0, t1 + Duration::from_millis(100));
        let out = f.end("hidden", "", 1, 1, 300, t1 + Duration::from_millis(500));
        assert_eq!(out[0].network_stall_ms, Some(0));
        assert_eq!(out[0].network_peak_mb_s, Some(2.0));
        assert_eq!(out[0].network_avg_mb_s, Some(2.0));
    }

    /// On-device regression: a deadsurge.dcl.eth -> Genesis teleport began while the realm was
    /// still the world being left, and bucketed as `world`. The end-time realm must win.
    #[test]
    fn end_realm_overrides_the_realm_being_left() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let started = f.begin(ctx("on_teleport", "deadsurge.dcl.eth"), t0);
        assert_eq!(started.realm_bucket, "world"); // all begin can know: where we came from
        assert_eq!(started.is_world, Some(true));

        let out = f.end(
            "hidden",
            "https://realm-provider-ea.decentraland.org/main/",
            1,
            1,
            200,
            t0 + Duration::from_millis(1000),
        );
        assert_eq!(out[0].realm_bucket, "genesis");
        assert_eq!(out[0].is_world, Some(false));
    }

    /// The reverse direction must keep working: Genesis -> world resolves to `world` at end.
    #[test]
    fn end_realm_resolves_world_destination() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("on_teleport", "https://peer.decentraland.org"), t0);
        let out = f.end(
            "hidden",
            "https://worlds-content-server.decentraland.org/world/towerofmadness.dcl.eth/",
            1,
            1,
            200,
            t0 + Duration::from_millis(1000),
        );
        assert_eq!(out[0].realm_bucket, "world");
        assert_eq!(out[0].is_world, Some(true));
    }

    /// An empty end-time realm must not wipe what begin knew (realm singleton gone / teardown).
    #[test]
    fn empty_end_realm_keeps_begin_realm() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("on_teleport", "deadsurge.dcl.eth"), t0);
        let out = f.end("hidden", "", 1, 1, 200, t0 + Duration::from_millis(1000));
        assert_eq!(out[0].realm_bucket, "world");
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

    /// The exact realm strings captured on-device. The short forms above are what the loading
    /// screen forwards; these are what the realm actually resolves to, and every one of them
    /// bucketed as `other` before this was fixed.
    #[test]
    fn realm_bucket_handles_real_device_realm_strings() {
        assert_eq!(
            realm_bucket("https://realm-provider-ea.decentraland.org/main/"),
            "genesis"
        );
        assert_eq!(
            realm_bucket("https://worlds-content-server.decentraland.org/world/deadsurge.dcl.eth/"),
            "world"
        );
        assert_eq!(
            realm_bucket(
                "https://worlds-content-server.decentraland.org/world/towerofmadness.dcl.eth/"
            ),
            "world"
        );
        assert_eq!(realm_bucket("no-realm"), "unknown");
    }

    #[test]
    fn is_world_realm_matches_both_shapes() {
        assert!(is_world_realm("deadsurge.dcl.eth"));
        assert!(is_world_realm(
            "https://worlds-content-server.decentraland.org/world/deadsurge.dcl.eth/"
        ));
        // Worlds may carry a dash; realm.gd's stricter `^[a-zA-Z0-9]+\.dcl\.eth$` rejects those,
        // which is fine for validating typed input but would misbucket a real world here.
        assert!(is_world_realm("has-dash.dcl.eth"));
        assert!(!is_world_realm(
            "https://realm-provider-ea.decentraland.org/main/"
        ));
        assert!(!is_world_realm("name.dcl.eth.evil"));
        assert!(!is_world_realm(""));
    }

    #[test]
    fn started_event_shape() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let ev = f.begin(ctx("on_teleport", "genesis city"), t0);
        assert_eq!(ev.event_type, "started");
        assert_eq!(ev.loading_id, 1);
        assert_eq!(ev.realm_bucket, "genesis");
        assert_eq!(ev.when.as_deref(), Some("on_teleport"));
        assert_eq!(ev.is_world, Some(false));
        assert!(f.is_active());
    }

    #[test]
    fn completed_funnel_and_ids_correlate() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let started = f.begin(ctx("auto", "world.dcl.eth"), t0);

        f.on_phase(LoadingPhase::Metadata, t0 + Duration::from_millis(10));
        f.on_scene_spawned(t0 + Duration::from_millis(50));
        f.on_phase(LoadingPhase::Assets, t0 + Duration::from_millis(80));
        f.on_progress(27.0, t0 + Duration::from_millis(100)); // enter band
        f.on_progress(28.0, t0 + Duration::from_millis(600)); // still in band
        f.on_progress(55.0, t0 + Duration::from_millis(900)); // leave band (+800ms)
        f.on_complete(t0 + Duration::from_millis(1000));

        let events = f.end("hidden", "", 12, 12, 260, t0 + Duration::from_millis(1200));
        assert_eq!(events.len(), 1); // no asset failures
        let c = &events[0];
        assert_eq!(c.event_type, "completed");
        assert_eq!(c.loading_id, started.loading_id); // same load correlates
        assert_eq!(c.dismissed_by.as_deref(), Some("completion"));
        assert_eq!(c.duration_ms, Some(1200));
        assert_eq!(c.first_spawn_ms, Some(50));
        assert_eq!(c.complete_ms, Some(1000));
        assert_eq!(c.time_in_25_30_band_ms, Some(800));
        assert_eq!(c.process_assets_pending_at_end, Some(0)); // 12 requested, 12 completed
        assert_eq!(c.frames, Some(160)); // 260 - 100
        assert!(!f.is_active());
    }

    #[test]
    fn timeout_without_ready_is_infinite_loading_signal() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("auto", "main"), t0);
        let events = f.end("timeout", "", 0, 4, 100, t0 + Duration::from_secs(90));
        let c = &events[0];
        assert_eq!(c.complete_ms, Some(-1)); // never reached `done`
        assert_eq!(c.dismissed_by.as_deref(), Some("wall_clock_timeout"));
    }

    /// A load whose only completion evidence is the `done` phase must still classify as
    /// `completion`. This was a real on-device misclassification: the dismissal used to depend on
    /// a progress-tick milestone that the session teardown races, so completed loads were reported
    /// as `wall_clock_timeout` and would have poisoned the infinite-loading metric.
    #[test]
    fn completion_is_decided_by_the_done_phase_alone() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        f.begin(ctx("auto", "main"), t0);
        f.on_complete(t0 + Duration::from_secs(67)); // session reached `done`
        let events = f.end("hidden", "", 10, 10, 100, t0 + Duration::from_secs(69));
        let c = &events[0];
        assert_eq!(c.complete_ms, Some(67_000));
        assert_eq!(c.dismissed_by.as_deref(), Some("completion")); // NOT wall_clock_timeout
    }

    #[test]
    fn asset_failures_emit_second_event_with_same_id() {
        let mut f = LoadingFunnel::default();
        let t0 = Instant::now();
        let started = f.begin(ctx("auto", "main"), t0);
        f.note_asset_failure(2);
        f.note_asset_failure(1);
        let events = f.end("hidden", "", 1, 1, 10, t0 + Duration::from_millis(500));
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
        f.begin(ctx("auto", "main"), t0);
        let rf = f.realm_change_failed("bad.example", "invalid /about response");
        assert_eq!(rf.event_type, "realm_change_failed");
        assert_eq!(rf.loading_id, 1);
        let events = f.end("hidden", "", 0, 0, 10, t0 + Duration::from_millis(200));
        assert_eq!(events[0].dismissed_by.as_deref(), Some("error"));
    }

    #[test]
    fn end_without_begin_is_noop() {
        let mut f = LoadingFunnel::default();
        assert!(f.end("hidden", "", 0, 0, 0, Instant::now()).is_empty());
    }
}
