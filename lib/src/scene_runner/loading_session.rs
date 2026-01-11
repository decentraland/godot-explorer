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
    /// Loading GLTF assets (40-80%)
    Assets,
    /// Waiting for scenes to reach tick >= 4 (80-100%)
    Ready,
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
}

impl LoadingSession {
    /// Session timeout in seconds
    pub const SESSION_TIMEOUT_SECS: u64 = 60;
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
        }
    }

    /// Calculate current progress (0-100), never decreasing
    pub fn calculate_progress(&mut self) -> f32 {
        let raw = match self.phase {
            LoadingPhase::Idle => 0.0,
            LoadingPhase::Metadata => {
                // 0-20%: Fetching scene metadata
                let expected = self.expected_scene_entities.len().max(1) as f32;
                let fetched = self.fetched_scene_entities.len() as f32;
                (fetched / expected) * 20.0
            }
            LoadingPhase::Spawning => {
                // 20-40%: Spawning scene nodes
                let expected = self.expected_scene_entities.len().max(1) as f32;
                let spawned = self.spawned_scenes.len() as f32;
                20.0 + (spawned / expected) * 20.0
            }
            LoadingPhase::Assets => {
                // 40-80%: Loading GLTF assets
                let total_expected: u32 = self.expected_assets.values().sum();
                let total_loaded: u32 = self.loaded_assets.values().sum();
                let ratio = if total_expected > 0 {
                    (total_loaded as f32) / (total_expected as f32)
                } else {
                    1.0 // No assets expected = 100% done
                };
                40.0 + ratio * 40.0
            }
            LoadingPhase::Ready => {
                // 80-100%: Waiting for scenes to reach tick >= 4
                let spawned = self.spawned_scenes.len().max(1) as f32;
                let ready = self.ready_scenes.len() as f32;
                80.0 + (ready / spawned) * 20.0
            }
            LoadingPhase::Done => 100.0,
        };

        // Never go backwards (high water mark)
        self.max_progress = self.max_progress.max(raw);
        self.max_progress
    }

    /// Check if session has timed out
    pub fn is_session_timed_out(&self) -> bool {
        self.start_time.elapsed().as_secs() > Self::SESSION_TIMEOUT_SECS
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

    /// Report that an asset started loading (increments expected count)
    pub fn report_asset_loading_started(&mut self, scene_id: SceneId) {
        *self.expected_assets.entry(scene_id).or_insert(0) += 1;
        let now = Instant::now();
        self.scene_last_progress.insert(scene_id, now);
        self.last_asset_registered = Some(now);
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

    /// Minimum time to wait in Assets phase for asset discovery (milliseconds)
    const ASSET_DISCOVERY_GRACE_MS: u64 = 500;
    /// Time since last asset registration before we consider discovery complete (milliseconds)
    const ASSET_STABLE_MS: u64 = 200;

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
                // Wait for asset discovery grace period
                let grace_period_passed = self
                    .assets_phase_start
                    .map(|start| {
                        now.duration_since(start).as_millis()
                            >= Self::ASSET_DISCOVERY_GRACE_MS as u128
                    })
                    .unwrap_or(false);

                // Check if asset discovery has stabilized (no new assets registered recently)
                let discovery_stable = self
                    .last_asset_registered
                    .map(|last| {
                        now.duration_since(last).as_millis() >= Self::ASSET_STABLE_MS as u128
                    })
                    .unwrap_or(true); // If no assets ever registered, consider stable

                // Check if all registered assets are loaded
                let all_loaded = self.expected_assets.iter().all(|(scene_id, expected)| {
                    self.loaded_assets.get(scene_id).unwrap_or(&0) >= expected
                });

                // Transition conditions:
                // 1. Grace period passed AND discovery stable AND (have assets that are all loaded, OR no assets)
                if grace_period_passed && discovery_stable && all_loaded {
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
                // Transition when all spawned scenes are ready
                if !self.spawned_scenes.is_empty()
                    && self.ready_scenes.len() >= self.spawned_scenes.len()
                {
                    self.phase = LoadingPhase::Done;
                }
                // Also complete if no scenes were spawned (empty area)
                if self.spawned_scenes.is_empty() && self.expected_scene_entities.is_empty() {
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

        // Force grace period to pass by backdating assets_phase_start
        session.assets_phase_start = Some(Instant::now() - std::time::Duration::from_secs(1));
        session.last_asset_registered = Some(Instant::now() - std::time::Duration::from_secs(1));

        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Ready);

        // Scene ready
        session.report_scene_ready(SceneId(1));
        session.check_phase_transition();
        assert_eq!(session.phase, LoadingPhase::Done);
    }
}
