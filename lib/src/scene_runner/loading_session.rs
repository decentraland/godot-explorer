use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::dcl::SceneId;

/// Loading phases with fixed progress ranges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingPhase {
    /// No loading active
    Idle,
    /// Fetching scene metadata (0-20%)
    Metadata,
    /// Spawning scene nodes (20-40%)
    Spawning,
    /// Loading GLTF assets (40-75%)
    Assets,
    /// Waiting for scenes to reach tick >= 4 (75-90%)
    Ready,
    /// Generating floating island terrain (90-100%)
    FloatingIslands,
    /// Loading complete
    Done,
}

impl LoadingPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoadingPhase::Idle => "idle",
            LoadingPhase::Metadata => "metadata",
            LoadingPhase::Spawning => "spawning",
            LoadingPhase::Assets => "assets",
            LoadingPhase::Ready => "ready",
            LoadingPhase::FloatingIslands => "floating_islands",
            LoadingPhase::Done => "done",
        }
    }
}

/// Tracks the loading state of a single loading session.
/// A new session is created each time the user teleports or scenes change.
pub struct LoadingSession {
    /// Unique session ID
    pub id: u64,
    /// Current loading phase
    pub phase: LoadingPhase,
    /// When the session started
    pub start_time: Instant,

    // Phase 1: Metadata (scene entity IDs are strings like "bafkreie...")
    /// Scene entity IDs we expect to fetch
    pub expected_scene_entities: HashSet<String>,
    /// Scene entity IDs that have been fetched
    pub fetched_scene_entities: HashSet<String>,

    // Phase 2-4: Scenes (numeric SceneId after spawn)
    /// Scenes that have been spawned (SceneId mapped from entity ID)
    pub spawned_scenes: HashSet<SceneId>,
    /// Expected asset count per scene
    pub expected_assets: HashMap<SceneId, u32>,
    /// Loaded asset count per scene
    pub loaded_assets: HashMap<SceneId, u32>,
    /// Scenes that are fully ready (tick >= 4 and GLTF done)
    pub ready_scenes: HashSet<SceneId>,

    // Timeout tracking
    /// Last progress timestamp per scene for timeout detection
    pub scene_last_progress: HashMap<SceneId, Instant>,

    // Progress tracking
    /// High water mark - progress never goes backwards
    max_progress: f32,

    // Asset discovery tracking
    /// When we entered the Assets phase (for grace period)
    assets_phase_start: Option<Instant>,
    /// Tracks the last time an asset was registered (started loading)
    last_asset_registered: Option<Instant>,

    // Floating islands tracking
    /// Whether we're waiting for floating islands generation
    pub waiting_for_floating_islands: bool,
    /// When floating islands generation started (for asset discovery delay)
    floating_islands_phase_start: Option<Instant>,
    /// Total expected floating island parcels
    floating_islands_expected: u32,
    /// Number of floating island parcels created so far
    floating_islands_created: u32,
}

impl LoadingSession {
    /// Per-scene timeout in seconds (no progress)
    pub const SCENE_TIMEOUT_SECS: u64 = 10;

    /// Create a new loading session
    pub fn new(id: u64, scene_entity_ids: Vec<String>) -> Self {
        Self {
            id,
            phase: if scene_entity_ids.is_empty() {
                LoadingPhase::Done
            } else {
                LoadingPhase::Metadata
            },
            start_time: Instant::now(),
            expected_scene_entities: scene_entity_ids.into_iter().collect(),
            fetched_scene_entities: HashSet::new(),
            spawned_scenes: HashSet::new(),
            expected_assets: HashMap::new(),
            loaded_assets: HashMap::new(),
            ready_scenes: HashSet::new(),
            scene_last_progress: HashMap::new(),
            max_progress: 0.0,
            assets_phase_start: None,
            last_asset_registered: None,
            waiting_for_floating_islands: false,
            floating_islands_phase_start: None,
            floating_islands_expected: 0,
            floating_islands_created: 0,
        }
    }

    /// Calculate current progress (0-100), never decreasing
    /// Uses weight-based accumulation: each phase contributes its weighted portion to total
    pub fn calculate_progress(&mut self) -> f32 {
        if self.phase == LoadingPhase::Idle {
            return 0.0;
        }
        if self.phase == LoadingPhase::Done {
            self.max_progress = 100.0;
            return 100.0;
        }

        // Calculate each phase's contribution based on its weight
        let expected_scenes = self.expected_scene_entities.len().max(1) as f32;

        // Metadata: weight 5%
        let metadata_ratio = self.fetched_scene_entities.len() as f32 / expected_scenes;
        let metadata_contribution = metadata_ratio * Self::WEIGHT_METADATA;

        // Spawning: weight 5%
        let spawning_ratio = self.spawned_scenes.len() as f32 / expected_scenes;
        let spawning_contribution = spawning_ratio * Self::WEIGHT_SPAWNING;

        // Assets: weight 60%
        // Only count assets progress after floating islands started AND 5 seconds have passed
        // This gives time for asset discovery before showing progress
        const ASSET_DISCOVERY_WAIT_SECS: u64 = 5;
        let metadata_complete =
            self.fetched_scene_entities.len() >= self.expected_scene_entities.len();
        let asset_discovery_ready = self
            .floating_islands_phase_start
            .map(|start| start.elapsed().as_secs() >= ASSET_DISCOVERY_WAIT_SECS)
            .unwrap_or(false);

        let total_expected_assets: u32 = self.expected_assets.values().sum();
        let total_loaded_assets: u32 = self.loaded_assets.values().sum();

        let assets_ratio = if !metadata_complete || !asset_discovery_ready {
            // Still waiting for metadata or asset discovery period
            0.0
        } else if total_expected_assets > 0 {
            (total_loaded_assets as f32) / (total_expected_assets as f32)
        } else if self.phase as u8 > LoadingPhase::Assets as u8 {
            // Assets phase completed with no assets
            1.0
        } else {
            // No assets discovered yet, but discovery period passed
            0.0
        };
        let assets_contribution = assets_ratio * Self::WEIGHT_ASSETS;

        // Ready: weight 15%
        let spawned_count = self.spawned_scenes.len().max(1) as f32;
        let ready_ratio = self.ready_scenes.len() as f32 / spawned_count;
        let ready_contribution = ready_ratio * Self::WEIGHT_READY;

        // FloatingIslands: weight 15%
        // Now percentage-based instead of simple 0%/100%
        let islands_ratio = if self.floating_islands_expected > 0 {
            // Use count-based progress during generation
            (self.floating_islands_created as f32 / self.floating_islands_expected as f32).min(1.0)
        } else if !self.waiting_for_floating_islands
            && self.phase as u8 >= LoadingPhase::FloatingIslands as u8
        {
            // Floating islands phase completed or skipped (no islands expected)
            1.0
        } else {
            // Not started yet
            0.0
        };
        let islands_contribution = islands_ratio * Self::WEIGHT_FLOATING_ISLANDS;

        // Sum all contributions
        let raw = metadata_contribution
            + spawning_contribution
            + assets_contribution
            + ready_contribution
            + islands_contribution;

        // Apply time-based dampening in early phases (first 20 seconds)
        // This prevents the bar from jumping to high % before we know actual scope
        let elapsed_secs = self.start_time.elapsed().as_secs();
        let dampened = if elapsed_secs < Self::EARLY_PHASE_DURATION_SECS
            && self.phase != LoadingPhase::FloatingIslands
            && self.phase != LoadingPhase::Done
        {
            raw.min(Self::EARLY_PHASE_MAX_PROGRESS) // Cap at 30% during first 20 seconds
        } else {
            raw
        };

        // Never go backwards (high water mark)
        self.max_progress = self.max_progress.max(dampened);
        self.max_progress
    }

    /// Get scenes that have timed out (no progress in SCENE_TIMEOUT_SECS)
    pub fn get_timed_out_scenes(&self, now: Instant) -> Vec<SceneId> {
        self.scene_last_progress
            .iter()
            .filter(|(scene_id, last_progress)| {
                !self.ready_scenes.contains(scene_id)
                    && now.duration_since(**last_progress).as_secs() > Self::SCENE_TIMEOUT_SECS
            })
            .map(|(scene_id, _)| *scene_id)
            .collect()
    }

    /// Report that a scene entity's metadata was fetched
    pub fn report_scene_fetched(&mut self, scene_entity_id: &str) {
        self.fetched_scene_entities
            .insert(scene_entity_id.to_string());
    }

    /// Report that a scene was spawned (maps entity ID to SceneId)
    /// Note: expected_assets is NOT set here - it's discovered dynamically as GLTF components are created
    pub fn report_scene_spawned(&mut self, scene_id: SceneId, _expected_assets: u32) {
        self.spawned_scenes.insert(scene_id);
        // Don't set expected_assets here - let asset discovery populate it
        // This prevents premature Assets → Ready transition
        self.scene_last_progress.insert(scene_id, Instant::now());
    }

    /// Report that an asset was loaded for a scene
    pub fn report_asset_loaded(&mut self, scene_id: SceneId) {
        *self.loaded_assets.entry(scene_id).or_insert(0) += 1;
        self.scene_last_progress.insert(scene_id, Instant::now());
    }

    /// Report that an asset started loading (increments expected count).
    /// Does NOT reset the per-scene timeout — only actual completions
    /// (`report_asset_loaded`) count as meaningful progress. This prevents
    /// a stream of "started" events from keeping a stalled scene alive.
    pub fn report_asset_loading_started(&mut self, scene_id: SceneId) {
        *self.expected_assets.entry(scene_id).or_insert(0) += 1;
        self.last_asset_registered = Some(Instant::now());
    }

    /// Report that a scene is fully ready (tick >= 4 and GLTF done)
    pub fn report_scene_ready(&mut self, scene_id: SceneId) {
        self.ready_scenes.insert(scene_id);
        self.scene_last_progress.insert(scene_id, Instant::now());
    }

    /// Report that a scene encountered an error (treat as ready)
    pub fn report_scene_error(&mut self, scene_id: SceneId) {
        // Treat errored scenes as ready so we don't block on them
        self.ready_scenes.insert(scene_id);
    }

    /// Mark timed-out scenes as ready
    pub fn mark_timed_out_scenes_ready(&mut self, scene_ids: Vec<SceneId>) {
        for scene_id in scene_ids {
            self.ready_scenes.insert(scene_id);
        }
    }

    /// Start floating islands generation phase with expected count
    pub fn start_floating_islands(&mut self, count: u32) {
        self.waiting_for_floating_islands = true;
        self.floating_islands_expected = count;
        self.floating_islands_created = 0;
    }

    /// Report floating islands generation progress
    pub fn report_floating_islands_progress(&mut self, created: u32, total: u32) {
        self.floating_islands_created = created;
        self.floating_islands_expected = total;
    }

    /// Finish floating islands generation (100% progress)
    /// Also starts the 5-second asset discovery delay timer
    pub fn finish_floating_islands(&mut self) {
        self.waiting_for_floating_islands = false;
        self.floating_islands_phase_start = Some(Instant::now());
        // Ensure counts reflect completion
        self.floating_islands_created = self.floating_islands_expected;
    }

    /// Minimum time to wait in Assets phase for asset discovery (milliseconds)
    const ASSET_DISCOVERY_GRACE_MS: u64 = 500;
    /// Time since last asset registration before we consider discovery complete (milliseconds)
    const ASSET_STABLE_MS: u64 = 200;

    /// Duration in seconds during which we dampen progress to avoid instant jumps
    const EARLY_PHASE_DURATION_SECS: u64 = 20;
    /// Maximum progress allowed during early phase (before we know actual scope)
    const EARLY_PHASE_MAX_PROGRESS: f32 = 30.0;

    // Phase weights (must sum to 100)
    const WEIGHT_METADATA: f32 = 5.0;
    const WEIGHT_SPAWNING: f32 = 5.0;
    const WEIGHT_ASSETS: f32 = 60.0;
    const WEIGHT_READY: f32 = 15.0;
    const WEIGHT_FLOATING_ISLANDS: f32 = 15.0;

    /// Check and advance loading phase if conditions are met
    /// Returns true if phase changed
    pub fn check_phase_transition(&mut self) -> bool {
        let old_phase = self.phase;
        let now = Instant::now();

        match self.phase {
            LoadingPhase::Metadata => {
                // Transition when all expected scenes are fetched
                if self.fetched_scene_entities.len() >= self.expected_scene_entities.len() {
                    self.phase = LoadingPhase::Spawning;
                }
            }
            LoadingPhase::Spawning => {
                // Transition when all expected scenes are spawned
                if self.spawned_scenes.len() >= self.expected_scene_entities.len() {
                    self.phase = LoadingPhase::Assets;
                    self.assets_phase_start = Some(now);
                }
            }
            LoadingPhase::Assets => {
                // Don't transition while floating islands are being generated (blocks the game loop)
                if self.waiting_for_floating_islands {
                    return false;
                }

                // Use floating_islands_phase_start if available (after islands finish),
                // otherwise fall back to assets_phase_start
                let grace_start = self
                    .floating_islands_phase_start
                    .or(self.assets_phase_start);
                let grace_period_ms = grace_start
                    .map(|start| now.duration_since(start).as_millis())
                    .unwrap_or(0);
                let grace_period_passed = grace_period_ms >= Self::ASSET_DISCOVERY_GRACE_MS as u128;

                // Check if asset discovery has stabilized (no new assets registered recently)
                let discovery_stable = self
                    .last_asset_registered
                    .map(|last| {
                        now.duration_since(last).as_millis() >= Self::ASSET_STABLE_MS as u128
                    })
                    .unwrap_or(true); // If no assets ever registered, consider stable

                // Check if all registered assets are loaded
                let total_expected: u32 = self.expected_assets.values().sum();
                let total_loaded: u32 = self.loaded_assets.values().sum();
                let all_loaded = self.expected_assets.iter().all(|(scene_id, expected)| {
                    self.loaded_assets.get(scene_id).unwrap_or(&0) >= expected
                });

                // Transition conditions:
                // 1. Grace period passed AND discovery stable AND (have assets that are all loaded, OR no assets)
                if grace_period_passed && discovery_stable && all_loaded {
                    tracing::debug!(
                        "[LOADING] Assets->Ready: grace={}ms, stable={}, loaded={}/{}, expected_assets={}",
                        grace_period_ms,
                        discovery_stable,
                        total_loaded,
                        total_expected,
                        self.expected_assets.len()
                    );
                    // Log a warning if no assets were discovered - this could indicate a bug
                    // where GLTF components weren't registered properly, or an empty scene
                    if self.expected_assets.is_empty() && !self.spawned_scenes.is_empty() {
                        tracing::warn!(
                            "LoadingSession {}: Assets phase completed with 0 assets discovered for {} spawned scenes. \
                             This may indicate scenes without GLTF content or a registration issue.",
                            self.id,
                            self.spawned_scenes.len()
                        );
                    }
                    self.phase = LoadingPhase::Ready;
                }
            }
            LoadingPhase::Ready => {
                // Check if all spawned scenes are ready
                let all_scenes_ready = (!self.spawned_scenes.is_empty()
                    && self.ready_scenes.len() >= self.spawned_scenes.len())
                    || (self.spawned_scenes.is_empty() && self.expected_scene_entities.is_empty());

                if all_scenes_ready {
                    tracing::debug!(
                        "[LOADING] Ready->next: ready_scenes={}/{}, waiting_for_islands={}",
                        self.ready_scenes.len(),
                        self.spawned_scenes.len(),
                        self.waiting_for_floating_islands
                    );
                    // If waiting for floating islands, transition to that phase
                    // Otherwise, go directly to Done
                    if self.waiting_for_floating_islands {
                        self.phase = LoadingPhase::FloatingIslands;
                    } else {
                        self.phase = LoadingPhase::Done;
                    }
                }
            }
            LoadingPhase::FloatingIslands => {
                // Transition when floating islands generation is finished
                if !self.waiting_for_floating_islands {
                    self.phase = LoadingPhase::Done;
                }
            }
            LoadingPhase::Idle | LoadingPhase::Done => {}
        }

        old_phase != self.phase
    }

    /// Check if loading is complete
    pub fn is_complete(&self) -> bool {
        self.phase == LoadingPhase::Done
    }

    /// Get ready scene count and total scene count for progress display
    pub fn get_scene_counts(&self) -> (usize, usize) {
        (self.ready_scenes.len(), self.spawned_scenes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_session_completes_immediately() {
        let session = LoadingSession::new(1, vec![]);
        assert_eq!(session.phase, LoadingPhase::Done);
    }

    #[test]
    fn test_progress_never_decreases() {
        let mut session = LoadingSession::new(1, vec!["scene1".to_string(), "scene2".to_string()]);

        // Simulate partial progress
        session.report_scene_fetched("scene1");
        let progress1 = session.calculate_progress();

        // Even if we somehow "unfetch" (which shouldn't happen), progress stays
        session.fetched_scene_entities.clear();
        let progress2 = session.calculate_progress();

        assert!(
            progress2 >= progress1,
            "Progress should never decrease: {} >= {}",
            progress2,
            progress1
        );
    }

    #[test]
    fn test_phase_transitions() {
        let mut session = LoadingSession::new(1, vec!["scene1".to_string()]);
        assert_eq!(session.phase, LoadingPhase::Metadata);

        // Fetch scene
        session.report_scene_fetched("scene1");
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Spawning);

        // Spawn scene
        session.report_scene_spawned(SceneId(1), 0);
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Assets);

        // Register assets (discovered after spawn)
        session.report_asset_loading_started(SceneId(1));
        session.report_asset_loading_started(SceneId(1));

        // Load assets
        session.report_asset_loaded(SceneId(1));
        session.report_asset_loaded(SceneId(1));

        // Force grace period to pass by backdating assets_phase_start (6 secs for discovery wait)
        session.assets_phase_start = Some(Instant::now() - std::time::Duration::from_secs(6));
        session.last_asset_registered = Some(Instant::now() - std::time::Duration::from_secs(6));

        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Ready);

        // Scene ready
        session.report_scene_ready(SceneId(1));
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Done);
    }

    #[test]
    fn test_floating_islands_phase() {
        let mut session = LoadingSession::new(1, vec!["scene1".to_string()]);

        // Backdate session start to bypass time-based dampening (>20 secs)
        session.start_time = Instant::now() - std::time::Duration::from_secs(25);

        // Fast-forward through Metadata -> Spawning -> Assets
        session.report_scene_fetched("scene1");
        session.check_phase_transition();
        session.report_scene_spawned(SceneId(1), 0);
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Assets);

        // Backdate assets_phase_start to allow Assets -> Ready transition
        session.assets_phase_start = Some(Instant::now() - std::time::Duration::from_secs(6));
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Ready);

        // Start floating islands before scene is ready (0% floating islands progress)
        session.start_floating_islands(10); // 10 expected parcels
        assert!(session.waiting_for_floating_islands);
        assert_eq!(session.floating_islands_expected, 10);
        assert_eq!(session.floating_islands_created, 0);

        // Scene ready - should transition to FloatingIslands, not Done
        session.report_scene_ready(SceneId(1));
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::FloatingIslands);

        // Progress should be ~25% (metadata + spawning + ready = 5 + 5 + 15)
        // Assets are NOT counted yet because floating_islands_phase_start is None
        let progress = session.calculate_progress();
        assert!(
            progress >= 25.0 && progress < 26.0,
            "Progress should be ~25%, got {}",
            progress
        );

        // Finish floating islands (100% floating islands progress)
        // This also starts the 5-second asset discovery delay
        session.finish_floating_islands();
        assert!(!session.waiting_for_floating_islands);
        assert!(session.floating_islands_phase_start.is_some());

        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Done);

        // Done phase always returns 100% progress
        assert_eq!(session.calculate_progress(), 100.0);
    }
}
