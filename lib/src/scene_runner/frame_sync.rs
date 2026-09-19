//! Frame-locked scene ticks.
//!
//! Every rendered frame the scene manager KICKS the foreground scenes (current
//! parcel + globals) early in `_process`, right after the camera scripts have
//! finalised this frame's pose: it applies any scene output not yet applied and
//! replies to the scene thread with this frame's player / camera / pointer /
//! trigger state, which unblocks the scene's `onUpdate`. The main thread then
//! does its normal per-frame work while JS runs. As the very last `_process`
//! callback of the frame (`SceneFrameSyncCollector`, `process_priority =
//! i32::MAX`) it COLLECTS: it waits, bounded by one frame measured from the
//! kick, for the outputs of the scenes it kicked, applies them to the Godot
//! nodes and returns, so the frame that is drawn contains the scene's reaction
//! to this frame's input.
//!
//! A scene slower than the budget misses the frame (counted, surfaced to the
//! creator) and the renderer draws without it; a wedged scene stops being
//! waited on after `FRAME_SYNC_MAX_CONSECUTIVE_MISSES`. Background scenes keep
//! the distance-throttled, time-budgeted scheduler and are never waited on.

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
/// After this many consecutive missed frames the scene is assumed wedged and the
/// collect phase stops spending the wait budget on it until its next output.
pub const FRAME_SYNC_MAX_CONSECUTIVE_MISSES: u32 = 30;
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
    /// Replies sent so far. The first one answers the SDK's `crdtGetState` during
    /// `onStart` and is never awaited.
    pub replies_sent: u64,
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
    /// True once `consecutive_misses` reached the limit; cleared by the next output.
    pub wait_suppressed: bool,
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
