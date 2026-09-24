//! Frame-locked scene ticks.
//!
//! Every rendered frame the scene manager KICKS the foreground scenes (current
//! parcel + globals) early in `_process`, right after the camera scripts have
//! finalised this frame's pose: it applies any scene output not yet applied and
//! replies to the scene thread with this frame's player / camera / pointer /
//! trigger state, which unblocks the scene's `onUpdate`. The main thread then
//! does its normal per-frame work while JS runs. As the very last `_process`
//! callback of the frame (`SceneFrameSyncCollector`, `process_priority =
//! i32::MAX`) it COLLECTS: it waits for the outputs of the scenes it kicked, but
//! only for the frame's leftover slack (see `SceneManager::collect_deadline`),
//! applies them to the Godot nodes and returns, so a scene that keeps up draws
//! its reaction to this frame's input in this frame.
//!
//! A slow scene never lowers the fps (same as the Bevy and Unity explorers): the
//! frame is drawn without its update (counted as missed, surfaced to the
//! creator), and the late output is applied and answered at the next kick, so
//! `onUpdate` keeps running at whatever rate the scene sustains, with `dt` =
//! the rendered time since its previous reply. A scene whose `onUpdate` has not
//! returned for `SCENE_NOT_RESPONDING_TIMEOUT_SECS` is killed and reported as a
//! crash (scene-crash modal for the current parcel). Background scenes keep the
//! distance-throttled, time-budgeted scheduler and are never waited on.

use std::sync::atomic::AtomicU64;
use std::time::Instant;

use godot::prelude::*;

use super::scene_manager::SceneManager;

/// `_process` priority of the scene manager (the kick). Runs after the camera
/// scripts (`camera_collision_clamp.gd` = -20, `dcl_global_camera_controller.gd`
/// = -10) so the reply carries this frame's final camera pose, and before every
/// priority-0 node so JS overlaps the rest of the main thread's frame work.
pub const SCENE_MANAGER_KICK_PROCESS_PRIORITY: i32 = -5;
/// `_process` priority of the collector: after every other node.
pub const COLLECT_PROCESS_PRIORITY: i32 = i32::MAX;
/// Main-thread apply budget per frame shared by the foreground scenes (kick +
/// collect applies). Larger than the background budget because it only covers
/// the scenes the player is looking at; a load burst still slices across frames.
pub const FOREGROUND_APPLY_BUDGET_US: i64 = 8_000;
/// Consecutive frames a foreground apply may stay incomplete before it is forced
/// to completion (parity with the background scheduler).
pub const FOREGROUND_STUCK_FRAMES_THRESHOLD: u32 = 10;
/// Main-thread time kept free after the collect wait on top of the measured
/// physics and render cost, so the wait never pushes the frame past its slot.
pub const COLLECT_SAFETY_MARGIN_US: u64 = 1_000;
/// Rendered seconds a scene may go without answering a reply before it is
/// killed and reported as crashed (Bevy and Unity explorers use 10 s too).
pub const SCENE_NOT_RESPONDING_TIMEOUT_SECS: f32 = 10.0;
/// Largest frame delta counted towards the timeout, so a main-thread hitch or
/// the app returning from the background does not kill healthy scenes.
pub const SCENE_NOT_RESPONDING_MAX_STEP_SECS: f32 = 0.25;
/// Rendered frames between two evaluations of the creator-facing warning.
pub const PERF_WARNING_CHECK_FRAMES: u32 = 120;
/// Minimum spacing between two warnings for the same scene.
pub const PERF_WARNING_MIN_INTERVAL_SECS: f32 = 5.0;
/// Missed-frame ratio (percent of the last window) that triggers the warning.
pub const PERF_WARNING_MISSED_PCT: f32 = 20.0;

/// Total microseconds the collect phase blocked waiting for scene output. Kept
/// outside `update_scene::record_state_timing` so the GP benchmark's per-state
/// breakdown never attributes the wait to an apply state.
pub static FRAME_SYNC_WAIT_US: AtomicU64 = AtomicU64::new(0);

/// Per-scene frame-sync scheduling state.
#[derive(Default)]
pub struct FrameSyncState {
    /// Set when a reply was sent and its output has not arrived yet ("in flight").
    pub kick_time: Option<Instant>,
    /// Replies sent so far, foreground or background. The first one answers the
    /// SDK's `crdtGetState` during `onStart` and is never awaited.
    pub replies_sent: u64,
    /// A reply (other than the first) was sent and the scene has not answered it.
    pub awaiting_output: bool,
    /// Rendered seconds `awaiting_output` has been true (clamped per frame).
    pub unresponsive_secs: f32,
    /// The V8 inspector is attached: a breakpoint may hold `onUpdate` for as
    /// long as the developer wants, so the not-responding timeout is off.
    pub watchdog_exempt: bool,
    /// Rendered-frame seconds accumulated since the last reply; shipped as the
    /// scene's next `onUpdate(dt)`.
    pub pending_dt_seconds: f32,
    /// Main-thread time spent so far applying the current output (may span frames).
    pub apply_us_accum: u32,
    /// Consecutive frames the scene missed the collect deadline.
    pub consecutive_misses: u32,
    /// Outputs that arrived after their frame's deadline.
    pub late_outputs: u32,
    /// Replies skipped because the channel was still full or the CRDT was locked.
    pub kick_blocked: u32,
}

impl FrameSyncState {
    /// A reply reached the scene thread (see `_process_scene`'s `SendToThread`).
    pub fn on_reply_sent(&mut self) {
        self.replies_sent += 1;
        self.awaiting_output = self.replies_sent > 1;
        self.unresponsive_secs = 0.0;
    }

    /// The scene thread sent an output.
    pub fn on_output_received(&mut self) {
        self.awaiting_output = false;
        self.unresponsive_secs = 0.0;
        if self.consecutive_misses > 0 {
            self.late_outputs += 1;
        }
        self.consecutive_misses = 0;
    }

    /// Advances the not-responding timer by one rendered frame. True once the
    /// scene has gone `SCENE_NOT_RESPONDING_TIMEOUT_SECS` without answering.
    pub fn advance_unresponsive(&mut self, delta_seconds: f32) -> bool {
        if !self.awaiting_output || self.watchdog_exempt {
            return false;
        }
        self.unresponsive_secs += delta_seconds.clamp(0.0, SCENE_NOT_RESPONDING_MAX_STEP_SECS);
        self.unresponsive_secs >= SCENE_NOT_RESPONDING_TIMEOUT_SECS
    }
}

/// Runs the collect phase as the last `_process` callback of the frame. A child
/// of `SceneManager`, so it inherits its process mode (disabled until the
/// explorer enables the runner).
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct SceneFrameSyncCollector {
    base: Base<Node>,
}

#[godot_api]
impl INode for SceneFrameSyncCollector {
    fn ready(&mut self) {
        self.base_mut()
            .set_process_priority(COLLECT_PROCESS_PRIORITY);
    }

    fn process(&mut self, _delta: f64) {
        let Some(parent) = self.base().get_parent() else {
            return;
        };
        let Ok(mut manager) = parent.try_cast::<SceneManager>() else {
            return;
        };
        manager.bind_mut().frame_sync_collect_phase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_reply_is_never_awaited() {
        let mut state = FrameSyncState::default();
        state.on_reply_sent();
        assert!(!state.awaiting_output);
        for _ in 0..1_000 {
            assert!(!state.advance_unresponsive(0.1));
        }

        state.on_output_received();
        state.on_reply_sent();
        assert!(state.awaiting_output);
    }

    #[test]
    fn times_out_after_ten_rendered_seconds_without_output() {
        let mut state = FrameSyncState::default();
        state.on_reply_sent();
        state.on_output_received();
        state.on_reply_sent();

        // 9.9 s at 30 fps: still alive.
        for _ in 0..297 {
            assert!(!state.advance_unresponsive(1.0 / 30.0));
        }
        // An answer resets the timer.
        state.on_output_received();
        state.on_reply_sent();
        for _ in 0..299 {
            assert!(!state.advance_unresponsive(1.0 / 30.0));
        }
        assert!(state.advance_unresponsive(1.0 / 30.0));
    }

    #[test]
    fn a_long_hitch_counts_as_one_clamped_step() {
        let mut state = FrameSyncState::default();
        state.on_reply_sent();
        state.on_output_received();
        state.on_reply_sent();

        // App back from the background after a minute: one frame, 0.25 s.
        assert!(!state.advance_unresponsive(60.0));
        assert!((state.unresponsive_secs - SCENE_NOT_RESPONDING_MAX_STEP_SECS).abs() < 1e-6);
    }

    #[test]
    fn inspector_attached_never_times_out() {
        let mut state = FrameSyncState {
            watchdog_exempt: true,
            ..Default::default()
        };
        state.on_reply_sent();
        state.on_output_received();
        state.on_reply_sent();
        for _ in 0..1_000 {
            assert!(!state.advance_unresponsive(0.25));
        }
    }

    #[test]
    fn late_output_is_counted_once() {
        let mut state = FrameSyncState {
            consecutive_misses: 3,
            ..Default::default()
        };
        state.on_output_received();
        assert_eq!(state.late_outputs, 1);
        assert_eq!(state.consecutive_misses, 0);
        state.on_output_received();
        assert_eq!(state.late_outputs, 1);
    }
}
