use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::{
        common::SceneLogLevel,
        components::{
            internal_player_data::InternalPlayerData,
            proto_components::{
                common::BorderRect,
                sdk::components::{
                    common::{InputAction, PointerEventType, RaycastHit},
                    PbAvatarEmoteCommand, PbUiCanvasInformation, TransitionMode,
                },
            },
            SceneEntityId,
        },
        crdt::SceneCrdtStateProtoComponents,
        DclScene, DclSceneRealmData, RendererResponse, SceneId, SceneResponse, SpawnDclSceneData,
    },
    godot_classes::{
        dcl_avatar::DclAvatar, dcl_camera_3d::DclCamera3D, dcl_global::DclGlobal,
        dcl_ui_control::DclUiControl, dcl_virtual_camera::DclVirtualCamera,
        rpc_sender::take_and_compare_snapshot_response::DclRpcSenderTakeAndCompareSnapshotResponse,
        JsonGodotClass,
    },
    realm::dcl_scene_entity_definition::DclSceneEntityDefinition,
    tools::network_inspector::NETWORK_INSPECTOR_ENABLE,
};
use godot::{
    classes::{
        control::{LayoutPreset, MouseFilter},
        PhysicsRayQueryParameters3D,
    },
    prelude::*,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::atomic::AtomicU32,
    time::{Duration, Instant},
};

use super::{
    components::pointer_events::{get_entity_pointer_event, pointer_events_system},
    input::InputState,
    loading_session::LoadingSession,
    pool_manager::PoolManager,
    scene::{
        Dirty, GlobalSceneType, GodotDclRaycastResult, RaycastResult, Scene, SceneState, SceneType,
        SceneUpdateState,
    },
    update_scene::_process_scene,
};

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneManager {
    base: Base<Node>,

    #[var]
    base_ui: Gd<DclUiControl>,

    ui_canvas_information: PbUiCanvasInformation,
    interactable_area: Rect2i,

    scenes: HashMap<SceneId, Scene>,

    #[var]
    player_avatar_node: Gd<Node3D>,

    #[var]
    player_body_node: Gd<Node3D>,

    #[var]
    console: Callable,

    #[var]
    cursor_position: Vector2,

    #[var]
    raycast_use_cursor_position: bool,

    // Cached center position of viewport for raycasting
    viewport_center: Vector2,
    // Cached raycast query for performance
    cached_raycast_query: Gd<PhysicsRayQueryParameters3D>,

    player_position: Vector2i,
    current_parcel_scene_id: SceneId,
    last_current_parcel_scene_id: SceneId,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,

    total_time_seconds_time: f32,
    pause: bool,
    begin_time: Instant,
    sorted_scene_ids: Vec<SceneId>,
    dying_scene_ids: Vec<SceneId>,
    global_scene_ids: Vec<SceneId>,

    input_state: InputState,
    last_raycast_result: Option<GodotDclRaycastResult>,

    #[var]
    pointer_tooltips: VarArray,

    // Track avatar under crosshair
    last_avatar_under_crosshair: Option<Gd<DclAvatar>>,

    // Track when pointer was pressed on avatar for click-and-release mechanism
    avatar_pointer_press_time: Option<Instant>,

    // Global pool manager for all scene resources (physics areas, etc.)
    // Uses RefCell because we need interior mutability while iterating scenes
    pool_manager: RefCell<PoolManager>,

    // Loading session tracking
    current_loading_session: Option<LoadingSession>,
    next_session_id: u64,

    // SDK-controlled skybox time
    // When a scene sets the SkyboxTime component on the root entity,
    // the explorer should use that time instead of the global time
    #[var(get)]
    sdk_skybox_time_active: bool,
    #[var(get)]
    sdk_skybox_fixed_time: u32, // seconds since 00:00 (0-86400)
    #[var(get)]
    sdk_skybox_transition_forward: bool, // true = forward, false = backward
}

// This value is the current global tick number, is used for marking the cronolgy of lamport timestamp
pub static GLOBAL_TICK_NUMBER: AtomicU32 = AtomicU32::new(0);
pub static GLOBAL_TIMESTAMP: AtomicU32 = AtomicU32::new(0);

const MAX_TIME_PER_SCENE_TICK_US: i64 = 8333; // 50% of update time at 60fps
const MIN_TIME_TO_PROCESS_SCENE_US: i64 = 2083; // 25% of max_time_per_scene_tick_us (4 scenes per frame)

#[godot_api]
impl SceneManager {
    #[signal]
    fn scene_spawned(scene_id: i32, entity_id: GString);

    #[signal]
    fn scene_killed(scene_id: i32, entity_id: GString);

    // Loading session signals
    #[signal]
    fn loading_started(session_id: i64, expected_count: i32);

    #[signal]
    fn loading_phase_changed(phase: GString);

    #[signal]
    fn loading_progress(percent: f32, ready: i32, total: i32);

    #[signal]
    fn loading_complete(session_id: i64);

    #[signal]
    fn loading_timeout(session_id: i64);

    #[signal]
    fn loading_cancelled(session_id: i64);

    // Testing a comment for the API
    #[func]
    fn start_scene(
        &mut self,
        local_main_js_file_path: GString,
        local_main_crdt_file_path: GString,
        dcl_scene_entity_definition: Gd<DclSceneEntityDefinition>,
        inspect: bool,
    ) -> i32 {
        let scene_entity_definition = dcl_scene_entity_definition.bind().get_ref();

        let content_mapping = scene_entity_definition.content_mapping.clone();
        let scene_type = if scene_entity_definition.is_global {
            SceneType::Global(GlobalSceneType::GlobalRealm)
        } else {
            SceneType::Parcel
        };

        let dcl_global = DclGlobal::singleton();

        let new_scene_id = Scene::new_id();
        let signal_data = (new_scene_id, scene_entity_definition.id.clone());
        let testing_mode_active = dcl_global.bind().testing_scene_mode;
        let fixed_skybox_time = dcl_global.bind().fixed_skybox_time;
        let ethereum_provider = dcl_global.bind().ethereum_provider.clone();
        let ephemeral_wallet = DclGlobal::singleton()
            .bind()
            .player_identity
            .bind()
            .try_get_ephemeral_auth_chain();

        let realm = dcl_global.bind().realm.clone();
        let realm = realm.bind();
        let realm_name = realm.get_realm_name().to_string();
        let base_url = realm.get_realm_url().to_string();
        let network_id = realm.get_network_id();

        let is_preview = dcl_global.bind().get_preview_mode();

        let comms_adapter = dcl_global
            .bind()
            .comms
            .bind()
            .get_current_adapter_conn_str()
            .to_string();

        let network_inspector = dcl_global.bind().get_network_inspector();
        let network_inspector_sender =
            if NETWORK_INSPECTOR_ENABLE.load(std::sync::atomic::Ordering::Relaxed) {
                Some(network_inspector.bind().get_sender())
            } else {
                None
            };

        // The SDK expects only the origin (protocol://hostname) without path or trailing /
        let base_url = url::Url::parse(&base_url)
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or(base_url);

        let dcl_scene = DclScene::spawn_new_js_dcl_scene(SpawnDclSceneData {
            scene_id: new_scene_id,
            scene_entity_definition: scene_entity_definition.clone(),
            local_main_js_file_path: local_main_js_file_path.to_string(),
            local_main_crdt_file_path: local_main_crdt_file_path.to_string(),
            content_mapping: content_mapping.clone(),
            thread_sender_to_main: self.thread_sender_to_main.clone(),
            testing_mode: testing_mode_active,
            fixed_skybox_time,
            ethereum_provider,
            ephemeral_wallet,
            realm_info: DclSceneRealmData {
                base_url,
                realm_name,
                network_id,
                comms_adapter,
                is_preview,
            },
            inspect,
            network_inspector_sender,
        });

        let new_scene = Scene::new(
            new_scene_id,
            scene_entity_definition,
            dcl_scene,
            content_mapping.clone(),
            scene_type.clone(),
            self.base_ui.clone(),
        );

        self.base_mut().add_child(
            &new_scene
                .godot_dcl_scene
                .root_node_3d
                .clone()
                .upcast::<Node>(),
        );

        if let SceneType::Global(_) = scene_type {
            self.base_ui.add_child(
                &new_scene
                    .godot_dcl_scene
                    .root_node_ui
                    .clone()
                    .upcast::<Node>(),
            );
        }

        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);
        self.sorted_scene_ids.push(new_scene_id);

        // Update global scene cache
        if let SceneType::Global(_) = scene_type {
            self.global_scene_ids.push(new_scene_id);
        }

        self.compute_scene_distance();

        // Report to loading session that this scene was spawned (with 0 expected assets initially)
        // The expected assets will be updated as GLTF containers are created
        if let Some(session) = &mut self.current_loading_session {
            session.report_scene_spawned(new_scene_id, 0);
        }

        self.base_mut().call_deferred(
            "emit_signal",
            &[
                "scene_spawned".to_variant(),
                signal_data.0 .0.to_variant(),
                signal_data.1.to_variant(),
            ],
        );
        new_scene_id.0
    }

    #[func]
    fn kill_scene(&mut self, scene_id: i32) -> bool {
        let scene_id = SceneId(scene_id);
        if let Some(scene) = self.scenes.get_mut(&scene_id) {
            if let SceneState::Alive = scene.state {
                scene.state = SceneState::ToKill;
                self.dying_scene_ids.push(scene_id);
                return true;
            }
        }
        false
    }

    #[func]
    fn kill_all_scenes(&mut self) {
        for (scene_id, scene) in self.scenes.iter_mut() {
            if let SceneState::Alive = scene.state {
                scene.state = SceneState::ToKill;
                self.dying_scene_ids.push(*scene_id);
            }
        }
    }

    // ============== Loading Session API ==============

    /// Start a new loading session for the given scene entity IDs.
    /// Any existing session is automatically cancelled.
    #[func]
    pub fn start_loading_session(&mut self, scene_entity_ids: PackedStringArray) {
        // Cancel any existing session
        if let Some(old_session) = self.current_loading_session.take() {
            tracing::debug!("[LOADING] Cancelling previous session {}", old_session.id);
            self.base_mut()
                .emit_signal("loading_cancelled", &[(old_session.id as i64).to_variant()]);
        }

        self.next_session_id += 1;
        let ids: Vec<String> = scene_entity_ids
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let count = ids.len() as i32;
        let session_id = self.next_session_id as i64;

        tracing::debug!(
            "[LOADING] START session {} with {} scenes: {:?}",
            session_id,
            count,
            ids
        );

        // Handle empty case - complete immediately
        if ids.is_empty() {
            tracing::debug!("[LOADING] Empty session, completing immediately");
            self.base_mut()
                .emit_signal("loading_complete", &[session_id.to_variant()]);
            return;
        }

        let session = LoadingSession::new(self.next_session_id, ids);
        self.current_loading_session = Some(session);

        self.base_mut().emit_signal(
            "loading_started",
            &[session_id.to_variant(), count.to_variant()],
        );
        self.base_mut().emit_signal(
            "loading_phase_changed",
            &[GString::from("metadata").to_variant()],
        );
    }

    /// Report that a scene entity's metadata was fetched
    #[func]
    pub fn report_scene_fetched(&mut self, scene_entity_id: GString) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_scene_fetched(&scene_entity_id.to_string());
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        }
    }

    /// Report that a scene was spawned and is now loading assets
    #[func]
    pub fn report_scene_spawned(&mut self, scene_id: i32, expected_assets: i32) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_scene_spawned(SceneId(scene_id), expected_assets.max(0) as u32);
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        }
    }

    /// Report that an asset started loading for a scene
    #[func]
    pub fn report_asset_loading_started(&mut self, scene_id: i32) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_asset_loading_started(SceneId(scene_id));
            // Don't emit progress here - this increases the denominator
            // Progress will be emitted when assets complete
        }
    }

    /// Report that an asset finished loading for a scene
    #[func]
    pub fn report_asset_loaded(&mut self, scene_id: i32) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_asset_loaded(SceneId(scene_id));
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        }
    }

    /// Report that a scene is fully ready (tick >= 4 and all GLTF loaded)
    #[func]
    pub fn report_scene_ready(&mut self, scene_id: i32) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_scene_ready(SceneId(scene_id));
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        }
    }

    /// Report that a scene encountered an error (treat as ready to not block)
    #[func]
    pub fn report_scene_error(&mut self, scene_id: i32) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_scene_error(SceneId(scene_id));
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        }
    }

    /// Start floating islands generation phase with expected count
    #[func]
    pub fn start_floating_islands(&mut self, count: i32) {
        tracing::debug!("[LOADING] FLOATING ISLANDS START (count={})", count);
        if let Some(session) = &mut self.current_loading_session {
            tracing::debug!(
                "[LOADING] Session {} - floating islands starting, phase: {:?}, count: {}",
                session.id,
                session.phase,
                count
            );
            session.start_floating_islands(count as u32);
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        } else {
            tracing::debug!("[LOADING] No active session for floating islands start");
        }
    }

    /// Report floating islands generation progress
    #[func]
    pub fn report_floating_islands_progress(&mut self, created: i32, total: i32) {
        if let Some(session) = &mut self.current_loading_session {
            session.report_floating_islands_progress(created as u32, total as u32);
            self.emit_loading_progress();
        }
    }

    /// Finish floating islands generation (100% progress)
    #[func]
    pub fn finish_floating_islands(&mut self) {
        tracing::debug!("[LOADING] FLOATING ISLANDS FINISH");
        if let Some(session) = &mut self.current_loading_session {
            tracing::debug!(
                "[LOADING] Session {} - floating islands finished, phase before: {:?}",
                session.id,
                session.phase
            );
            session.finish_floating_islands();
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        } else {
            tracing::debug!("[LOADING] No active session for floating islands finish");
        }
    }

    /// Cancel the current loading session
    #[func]
    pub fn cancel_loading_session(&mut self) {
        if let Some(session) = self.current_loading_session.take() {
            self.base_mut()
                .emit_signal("loading_cancelled", &[(session.id as i64).to_variant()]);
        }
    }

    /// Check if there's an active loading session
    #[func]
    pub fn has_active_loading_session(&self) -> bool {
        self.current_loading_session.is_some()
    }

    /// Get the current loading progress (0-100)
    #[func]
    pub fn get_loading_progress(&mut self) -> f32 {
        if let Some(session) = &mut self.current_loading_session {
            session.calculate_progress()
        } else {
            100.0
        }
    }

    /// Get the current loading phase as a string
    #[func]
    pub fn get_loading_phase(&self) -> GString {
        if let Some(session) = &self.current_loading_session {
            GString::from(session.phase.as_str())
        } else {
            GString::from("idle")
        }
    }

    /// Internal: Check and emit phase transition
    fn check_loading_phase_transition(&mut self) {
        let (phase_changed, new_phase, is_complete, session_id) = {
            if let Some(session) = &mut self.current_loading_session {
                let changed = session.check_phase_transition();
                (
                    changed,
                    session.phase,
                    session.is_complete(),
                    session.id as i64,
                )
            } else {
                return;
            }
        };

        if phase_changed {
            tracing::debug!(
                "[LOADING] Phase changed to {:?} for session {}",
                new_phase,
                session_id
            );
            self.base_mut().emit_signal(
                "loading_phase_changed",
                &[GString::from(new_phase.as_str()).to_variant()],
            );
        }

        if is_complete {
            tracing::debug!("[LOADING] COMPLETE - session {} finished", session_id);
            self.current_loading_session = None;
            self.base_mut()
                .emit_signal("loading_complete", &[session_id.to_variant()]);
        }
    }

    /// Internal: Emit loading progress
    fn emit_loading_progress(&mut self) {
        if let Some(session) = &mut self.current_loading_session {
            let progress = session.calculate_progress();
            let (ready, total) = session.get_scene_counts();
            self.base_mut().emit_signal(
                "loading_progress",
                &[
                    progress.to_variant(),
                    (ready as i32).to_variant(),
                    (total as i32).to_variant(),
                ],
            );
        }
    }

    /// Internal: Check for individual scene timeouts (called from physics_process)
    fn check_loading_timeouts(&mut self) {
        let timed_out_scenes = {
            if let Some(session) = &mut self.current_loading_session {
                let now = Instant::now();
                let timed_out_scenes = session.get_timed_out_scenes(now);

                // Mark timed-out scenes as ready
                if !timed_out_scenes.is_empty() {
                    session.mark_timed_out_scenes_ready(timed_out_scenes.clone());
                }

                timed_out_scenes
            } else {
                return;
            }
        };

        // Log timed-out scenes
        if !timed_out_scenes.is_empty() {
            tracing::warn!(
                "Loading session: {} scenes timed out: {:?}",
                timed_out_scenes.len(),
                timed_out_scenes
            );
            self.check_loading_phase_transition();
            self.emit_loading_progress();
        }
    }

    /// Internal: Collect loading events from scenes and update loading session
    fn update_loading_session_from_scenes(&mut self) {
        if self.current_loading_session.is_none() {
            return;
        }

        // Get spawned scene IDs from the current session
        let spawned_in_session: HashSet<SceneId> = self
            .current_loading_session
            .as_ref()
            .map(|s| s.spawned_scenes.clone())
            .unwrap_or_default();

        // Collect all loading events from scenes
        let mut assets_started: Vec<(SceneId, u32)> = Vec::new();
        let mut assets_finished: Vec<(SceneId, u32)> = Vec::new();
        let mut scenes_ready: Vec<SceneId> = Vec::new();

        for (scene_id, scene) in self.scenes.iter_mut() {
            // Skip non-alive scenes
            if !matches!(scene.state, SceneState::Alive) {
                continue;
            }

            // Only track scenes that are part of this loading session
            if !spawned_in_session.contains(scene_id) {
                continue;
            }

            // Collect GLTF loading events
            if scene.gltf_loading_started_count > 0 {
                assets_started.push((*scene_id, scene.gltf_loading_started_count));
                scene.gltf_loading_started_count = 0;
            }
            if scene.gltf_loading_finished_count > 0 {
                assets_finished.push((*scene_id, scene.gltf_loading_finished_count));
                scene.gltf_loading_finished_count = 0;
            }

            // Check if scene is ready (tick >= 10 and no GLTF loading)
            // We wait for tick 10 instead of 4 to give GLTFs time to start loading
            if !scene.loading_reported_ready
                && scene.tick_number >= 10
                && scene.gltf_loading.is_empty()
            {
                tracing::debug!(
                    "[LOADING] Scene {:?} marked ready - tick={}, gltf_loading_count={}",
                    scene_id,
                    scene.tick_number,
                    scene.gltf_loading.len()
                );
                scene.loading_reported_ready = true;
                scenes_ready.push(*scene_id);
            }
        }

        // Now update the loading session with collected events
        let session = self.current_loading_session.as_mut().unwrap();

        for (scene_id, count) in assets_started {
            tracing::debug!(
                "[LOADING] Scene {:?} - {} assets STARTED loading",
                scene_id,
                count
            );
            for _ in 0..count {
                session.report_asset_loading_started(scene_id);
            }
        }

        for (scene_id, count) in assets_finished {
            tracing::debug!(
                "[LOADING] Scene {:?} - {} assets FINISHED loading (total: {}/{})",
                scene_id,
                count,
                session.loaded_assets.get(&scene_id).unwrap_or(&0) + count,
                session.expected_assets.get(&scene_id).unwrap_or(&0)
            );
            for _ in 0..count {
                session.report_asset_loaded(scene_id);
            }
        }

        for scene_id in scenes_ready {
            tracing::debug!("[LOADING] Scene {:?} reported READY", scene_id);
            session.report_scene_ready(scene_id);
        }

        // Check for phase transitions and emit progress
        self.check_loading_phase_transition();
        self.emit_loading_progress();
    }

    // ============== End Loading Session API ==============

    #[func]
    fn on_primary_player_trigger_emote(&mut self, emote_id: GString, looping: bool) {
        let emote_command = PbAvatarEmoteCommand {
            emote_urn: emote_id.to_string(),
            r#loop: looping,
            timestamp: 0,
        };

        // Primary player send to all the scenes
        for (_, scene) in self.scenes.iter_mut() {
            let emote_vector = scene
                .avatar_scene_updates
                .avatar_emote_command
                .entry(SceneEntityId::PLAYER)
                .or_insert(Vec::new());

            emote_vector.push(emote_command.clone());
        }
    }

    #[func]
    fn set_player_node(
        &mut self,
        player_avatar_node: Gd<Node3D>,
        player_body_node: Gd<Node3D>,
        console: Callable,
    ) {
        self.player_avatar_node = player_avatar_node.clone();
        self.player_body_node = player_body_node.clone();
        self.console = console;
    }

    #[func]
    fn get_scene_content_mapping(&self, scene_id: i32) -> Gd<DclContentMappingAndUrl> {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            DclContentMappingAndUrl::from_ref(scene.content_mapping.clone())
        } else {
            DclContentMappingAndUrl::empty()
        }
    }

    #[func]
    fn get_scene_title(&self, scene_id: i32) -> GString {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            return scene.scene_entity_definition.get_title().to_godot();
        }
        GString::default()
    }

    #[func]
    pub fn get_scene_entity_id(&self, scene_id: i32) -> GString {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            return scene.scene_entity_definition.id.clone().to_godot();
        }
        GString::default()
    }

    #[func]
    fn get_scene_is_paused(&self, scene_id: i32) -> bool {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            scene.paused
        } else {
            false
        }
    }

    #[func]
    fn set_scene_is_paused(&mut self, scene_id: i32, value: bool) {
        if let Some(scene) = self.scenes.get_mut(&SceneId(scene_id)) {
            scene.paused = value;
        }
    }

    #[func]
    pub fn get_scene_id_by_parcel_position(&self, parcel_position: Vector2i) -> i32 {
        for scene in self.scenes.values() {
            if let SceneType::Global(_) = scene.scene_type {
                continue;
            }

            if scene
                .scene_entity_definition
                .scene_meta_scene
                .scene
                .parcels
                .contains(&parcel_position)
            {
                return scene.scene_id.0;
            }
        }

        SceneId::INVALID.0
    }

    #[func]
    fn get_scene_base_parcel(&self, scene_id: i32) -> Vector2i {
        if let Some(scene) = self.scenes.get(&SceneId(scene_id)) {
            return scene.scene_entity_definition.scene_meta_scene.scene.base;
        }
        Vector2i::default()
    }

    fn compute_scene_distance(&mut self) {
        self.current_parcel_scene_id = SceneId::INVALID;

        let mut player_global_position = self.player_avatar_node.get_global_transform().origin;
        player_global_position.x *= 0.0625;
        player_global_position.y *= 0.0625;
        player_global_position.z *= -0.0625;
        let player_parcel_position = Vector2i::new(
            player_global_position.x.floor() as i32,
            player_global_position.z.floor() as i32,
        );

        for (id, scene) in self.scenes.iter_mut() {
            let (distance, inside_scene) = scene.min_distance(&player_parcel_position);
            scene.distance = distance;
            if inside_scene {
                if let SceneType::Parcel = scene.scene_type {
                    self.current_parcel_scene_id = *id;
                }
            }
        }
    }

    fn scene_runner_update(&mut self, delta: f64) {
        if self.pause {
            return;
        }

        let start_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
        let end_time_us = start_time_us + MAX_TIME_PER_SCENE_TICK_US;

        self.total_time_seconds_time += delta as f32;

        self.receive_from_thread();

        let player_global_transform = self.player_avatar_node.get_global_transform();
        let camera_node = self.base().get_viewport().and_then(|x| x.get_camera_3d());

        let (camera_global_transform, camera_mode) = match camera_node.as_ref() {
            Some(camera_node) => {
                let camera_global_transform = camera_node.get_global_transform();
                let camera_node = camera_node.clone().try_cast::<DclCamera3D>();
                let camera_mode = if let Ok(camera_node) = camera_node {
                    camera_node.bind().get_camera_mode()
                } else {
                    0
                };
                (camera_global_transform, camera_mode)
            }

            None => (player_global_transform, 0),
        };

        let frames_count = godot::classes::Engine::singleton().get_physics_frames();

        let player_parcel_position = Vector2i::new(
            (player_global_transform.origin.x / 16.0).floor() as i32,
            (-player_global_transform.origin.z / 16.0).floor() as i32,
        );

        if player_parcel_position != self.player_position {
            self.compute_scene_distance();
            self.player_position = player_parcel_position;
        }

        // TODO: review to define a better behavior
        self.sorted_scene_ids.sort_by_key(|&scene_id| {
            let scene = self.scenes.get_mut(&scene_id).unwrap();
            if !scene.current_dirty.waiting_process || scene.paused {
                scene.next_tick_us = start_time_us + 120000;
                // Set at the end of the queue: scenes without processing from scene-runtime, wait until something comes
            } else if scene_id == self.current_parcel_scene_id {
                scene.next_tick_us = 1; // hardcoded priority for current parcel
            } else {
                scene.next_tick_us =
                    scene.last_tick_us + (20000.0 * scene.distance).clamp(10000.0, 100000.0) as i64;

                // TODO: distance in meters or in parcels
            }
            scene.next_tick_us
        });

        let mut scene_to_remove: HashSet<SceneId> = HashSet::new();

        if self.current_parcel_scene_id != self.last_current_parcel_scene_id {
            self.on_current_parcel_scene_changed();
        }

        // TODO: this is debug information, very useful to see the scene priority
        // if self.total_time_seconds_time > 1.0 {
        //     self.total_time_seconds_time = 0.0;
        //     let next_update_vec: Vec<String> = self
        //         .sorted_scene_ids
        //         .iter()
        //         .map(|value| {
        //             let scene = self.scenes.get(value).unwrap();
        //             let last_tick_ms = ((scene.last_tick_us - start_time_us) as f32) / 1000.0;
        //             let next_tick_ms = ((scene.next_tick_us - start_time_us) as f32) / 1000.0;
        //             format!(
        //                 "{} = {:#?}ms => {:#?}ms || d= {:#?}",
        //                 value.0, last_tick_ms, next_tick_ms, scene.distance
        //             )
        //         })
        //         .collect();
        //     tracing::info!("next_update: {next_update_vec:#?}");
        // }

        let mut current_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
        for scene_id in self.sorted_scene_ids.iter() {
            let scene: &mut Scene = self.scenes.get_mut(scene_id).unwrap();

            current_time_us = (std::time::Instant::now() - self.begin_time).as_micros() as i64;
            if scene.next_tick_us > current_time_us {
                break;
            }
            if (end_time_us - current_time_us) < MIN_TIME_TO_PROCESS_SCENE_US {
                break;
            }

            if let SceneState::Alive = scene.state {
                if scene.dcl_scene.thread_join_handle.is_finished() {
                    tracing::error!("scene closed without kill signal");
                    scene_to_remove.insert(*scene_id);
                    continue;
                }

                if _process_scene(
                    scene,
                    end_time_us,
                    frames_count,
                    &camera_global_transform,
                    &player_global_transform,
                    camera_mode,
                    self.console.clone(),
                    &self.current_parcel_scene_id,
                    &self.begin_time,
                    &self.ui_canvas_information,
                    &self.pool_manager,
                ) {
                    scene.last_tick_us =
                        (std::time::Instant::now() - self.begin_time).as_micros() as i64;
                }
            }
        }

        // Process loading session updates from all scenes
        self.update_loading_session_from_scenes();

        // Read SkyboxTime component from the current parcel scene
        self.update_sdk_skybox_time();

        for scene_id in self.dying_scene_ids.iter() {
            let scene = self.scenes.get_mut(scene_id).unwrap();
            match scene.state {
                SceneState::ToKill => {
                    if let Err(_e) = scene
                        .dcl_scene
                        .main_sender_to_thread
                        .try_send(RendererResponse::Kill)
                    {
                        tracing::error!("error sending kill signal to thread");
                    } else {
                        scene.state = SceneState::KillSignal(current_time_us);
                    }
                }
                SceneState::KillSignal(kill_time_us) => {
                    if scene.dcl_scene.thread_join_handle.is_finished() {
                        scene.state = SceneState::Dead;
                    } else {
                        let elapsed_from_kill_us = current_time_us - kill_time_us;
                        if elapsed_from_kill_us > 10 * 1e6 as i64 {
                            // 10 seconds from the kill signal - force terminate V8
                            tracing::error!(
                                "timeout killing scene {:?}, forcing V8 termination",
                                scene_id
                            );

                            // Use the V8 isolate handle to force-terminate execution
                            #[cfg(feature = "use_deno")]
                            {
                                if let Ok(handles) = crate::dcl::js::VM_HANDLES.lock() {
                                    if let Some(handle) = handles.get(scene_id) {
                                        handle.terminate_execution();
                                        tracing::info!(
                                            "V8 execution terminated for scene {:?}",
                                            scene_id
                                        );
                                    }
                                }
                            }

                            // Mark as dead - thread should exit soon after V8 termination
                            scene.state = SceneState::Dead;
                        }
                    }
                }
                SceneState::Dead => {
                    scene_to_remove.insert(*scene_id);
                }
                _ => {
                    if scene.dcl_scene.thread_join_handle.is_finished() {
                        tracing::error!("scene closed without kill signal");
                        scene.state = SceneState::Dead;
                    }
                }
            }
        }

        // Periodic pool health check and stats logging (handled by PoolManager)
        self.pool_manager.borrow_mut().tick();

        for scene_id in scene_to_remove.iter() {
            let mut scene = self.scenes.remove(scene_id).unwrap();
            let signal_data = (*scene_id, scene.scene_entity_definition.id.clone());

            // Cleanup trigger areas and release RIDs back to pool
            scene
                .trigger_areas
                .cleanup(self.pool_manager.borrow_mut().physics_area());

            // Cleanup Rust references first (doesn't free nodes yet)
            scene.cleanup();

            // Free root nodes - queue_free handles both removal from tree and freeing
            // This is safer than manually calling remove_child + queue_free separately
            // because queue_free schedules everything atomically for end of frame
            scene.godot_dcl_scene.free_root_nodes();

            self.sorted_scene_ids.retain(|x| x != scene_id);
            self.dying_scene_ids.retain(|x| x != scene_id);
            self.global_scene_ids.retain(|x| x != scene_id);

            // Clean up VM_HANDLES entry
            #[cfg(feature = "use_deno")]
            {
                if let Ok(mut handles) = crate::dcl::js::VM_HANDLES.lock() {
                    handles.remove(scene_id);
                }
            }

            if scene.dcl_scene.thread_join_handle.is_finished() {
                if let Err(err) = scene.dcl_scene.thread_join_handle.join() {
                    let msg = if let Some(panic_info) = err.downcast_ref::<&str>() {
                        format!("Thread panicked with: {}", panic_info)
                    } else if let Some(panic_info) = err.downcast_ref::<String>() {
                        format!("Thread panicked with: {}", panic_info)
                    } else {
                        "Thread panicked with an unknown payload".to_string()
                    };
                    tracing::error!("scene {} thread result: {:?}", scene_id.0, msg);
                }
            }

            self.base_mut().emit_signal(
                "scene_killed",
                &[signal_data.0 .0.to_variant(), signal_data.1.to_variant()],
            );
        }
    }

    fn receive_from_thread(&mut self) {
        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => match response {
                    SceneResponse::Error(scene_id, msg) => {
                        let mut arguments = VarArray::new();
                        arguments.push(&(scene_id.0).to_variant());
                        arguments.push(&(SceneLogLevel::SystemError as i32).to_variant());
                        arguments.push(&self.total_time_seconds_time.to_variant());
                        arguments.push(&msg.to_godot().to_variant());
                        self.console.callv(&arguments);
                    }
                    SceneResponse::Ok {
                        scene_id,
                        dirty_crdt_state,
                        logs,
                        rpc_calls,
                        delta: _,
                        deno_memory_stats,
                    } => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            // Update Deno memory stats if present
                            if deno_memory_stats.is_some() {
                                scene.deno_memory_stats = deno_memory_stats;
                            }

                            let dirty = Dirty {
                                waiting_process: true,
                                entities: dirty_crdt_state.entities,
                                lww_components: dirty_crdt_state.lww,
                                gos_components: dirty_crdt_state.gos,
                                logs,
                                renderer_response: None,
                                update_state: SceneUpdateState::None,
                                rpc_calls,
                            };

                            if !scene.current_dirty.waiting_process {
                                scene.current_dirty = dirty;
                            } else {
                                scene.enqueued_dirty.push(dirty);
                            }
                        }
                    }
                    SceneResponse::RemoveGodotScene(scene_id, logs) => {
                        if let Some(scene) = self.scenes.get_mut(&scene_id) {
                            scene.state = SceneState::Dead;
                            if !self.dying_scene_ids.contains(&scene_id) {
                                self.dying_scene_ids.push(scene_id);
                            }
                        }
                        // enable logs
                        for log in &logs {
                            let mut arguments = VarArray::new();
                            arguments.push(&scene_id.0.to_variant());
                            arguments.push(&(log.level as i32).to_variant());
                            arguments.push(&(log.timestamp as f32).to_variant());
                            arguments.push(&log.message.to_godot().to_variant());
                            self.console.callv(&arguments);
                        }
                    }

                    SceneResponse::TakeSnapshot {
                        scene_id,
                        src_stored_snapshot,
                        camera_position,
                        camera_target,
                        screeshot_size,
                        method,
                        response,
                    } => {
                        let offset = if let Some(scene) = self.scenes.get(&scene_id) {
                            scene.scene_entity_definition.get_godot_3d_position()
                        } else {
                            Vector3::new(0.0, 0.0, 0.0)
                        };

                        let global_camera_position =
                            Vector3::new(camera_position.x, camera_position.y, camera_position.z)
                                + offset;

                        let global_camera_target =
                            Vector3::new(camera_target.x, camera_target.y, camera_target.z)
                                + offset;

                        let mut testing_tools = DclGlobal::singleton().bind().get_testing_tools();
                        if testing_tools.has_method("async_take_and_compare_snapshot") {
                            let mut dcl_rpc_sender: Gd<DclRpcSenderTakeAndCompareSnapshotResponse> =
                                DclRpcSenderTakeAndCompareSnapshotResponse::new_gd();
                            dcl_rpc_sender.bind_mut().set_sender(response);

                            testing_tools.call_deferred(
                                "async_take_and_compare_snapshot",
                                &[
                                    scene_id.0.to_variant(),
                                    src_stored_snapshot.to_variant(),
                                    global_camera_position.to_variant(),
                                    global_camera_target.to_variant(),
                                    screeshot_size.to_variant(),
                                    method
                                        .to_godot_from_json()
                                        .unwrap_or(VarDictionary::new().to_variant())
                                        .to_variant(),
                                    dcl_rpc_sender.to_variant(),
                                ],
                            );
                        } else {
                            response.send(Err("Testing tools not available".to_string()));
                        }
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("render thread receiver exploded");
                }
            }
        }
    }

    #[func]
    fn set_pause(&mut self, value: bool) {
        self.pause = value;
    }

    #[func]
    fn is_paused(&mut self) -> bool {
        self.pause
    }

    fn get_current_mouse_entity(&mut self) -> Option<RaycastResult> {
        const RAY_LENGTH: f32 = 100.0;
        const CL_POINTER: u32 = 1;
        const CL_AVATAR: u32 = 536870912; // Layer 30 for avatars

        let camera_node = self.base().get_viewport().and_then(|x| x.get_camera_3d())?;

        let screen_point = if self.raycast_use_cursor_position {
            self.cursor_position
        } else {
            self.viewport_center
        };

        // Use cached viewport center for raycasting
        let raycast_from = camera_node.project_ray_origin(screen_point);
        let raycast_to = raycast_from + camera_node.project_ray_normal(screen_point) * RAY_LENGTH;
        let mut space = camera_node.get_world_3d()?.get_direct_space_state()?;

        // Update the cached raycast query parameters
        self.cached_raycast_query.set_from(raycast_from);
        self.cached_raycast_query.set_to(raycast_to);
        self.cached_raycast_query
            .set_collision_mask(CL_POINTER | CL_AVATAR);
        // Need to collide with areas for avatars (they use Area3D)
        self.cached_raycast_query.set_collide_with_areas(true);

        let raycast_result = space.intersect_ray(&self.cached_raycast_query.clone());

        // Check if we hit anything at all
        if !raycast_result.contains_key("collider") {
            return None;
        }

        // Validate collider is still a valid object before calling methods on it
        // (object could be freed between raycast and method call during scene loading)
        let collider_obj: Gd<Object> = raycast_result.get("collider")?.try_to().ok()?;
        if !collider_obj.is_instance_valid() {
            return None;
        }

        // The raycast returns the closest hit, so we just need to identify what type it is
        // Priority is naturally handled by distance - closer objects are returned first

        // First check if this is a DCL entity (scene object)
        let has_dcl_entity_id = collider_obj.has_meta("dcl_entity_id");

        if has_dcl_entity_id {
            // It's a scene entity, return it
            let dcl_entity_id = collider_obj.get_meta("dcl_entity_id").to::<i32>();
            let dcl_scene_id = collider_obj.get_meta("dcl_scene_id").to::<i32>();

            let scene = self.scenes.get(&SceneId(dcl_scene_id))?;
            let scene_position = scene.godot_dcl_scene.root_node_3d.get_position();
            let raycast_data = RaycastHit::from_godot_raycast(
                scene_position,
                self.player_avatar_node.get_global_position(),
                &raycast_result,
                Some(dcl_entity_id as u32),
            )?;

            return Some(RaycastResult::SceneEntity(GodotDclRaycastResult {
                scene_id: SceneId(dcl_scene_id),
                entity_id: SceneEntityId::from_i32(dcl_entity_id),
                hit: raycast_data,
            }));
        }

        // If not a scene entity, check if it's an avatar
        let is_avatar =
            collider_obj.has_meta("is_avatar") && collider_obj.get_meta("is_avatar").booleanize();

        if is_avatar {
            // Check distance for avatar interactions (limit to 10 meters)
            const MAX_AVATAR_INTERACTION_DISTANCE: f32 = 10.0;

            // Get hit position from raycast result
            if let Some(position_variant) = raycast_result.get("position") {
                let hit_position = position_variant.to::<Vector3>();
                let distance = raycast_from.distance_to(hit_position);

                // Only allow avatar interaction within the distance limit
                if distance <= MAX_AVATAR_INTERACTION_DISTANCE {
                    // Walk up the node tree to find the DclAvatar node
                    // First try to cast collider_obj to Node for tree traversal
                    if let Ok(mut current_node) = collider_obj.clone().try_cast::<Node>() {
                        loop {
                            // Try to cast to DclAvatar
                            if let Ok(avatar) = current_node.clone().try_cast::<DclAvatar>() {
                                return Some(RaycastResult::Avatar(avatar));
                            }

                            // Try to get parent
                            match current_node.get_parent() {
                                Some(parent) => current_node = parent,
                                None => break,
                            }
                        }
                    }
                }
            }
        }

        // Nothing found
        None
    }

    #[signal]
    fn pointer_tooltip_changed();

    fn create_ui_canvas_information(&self) -> PbUiCanvasInformation {
        let canvas_size = self.base_ui.get_size();
        let window_size: Vector2i = godot::classes::DisplayServer::singleton().window_get_size();

        let device_pixel_ratio = window_size.y as f32 / canvas_size.y;

        PbUiCanvasInformation {
            device_pixel_ratio,
            width: canvas_size.x as i32,
            height: canvas_size.y as i32,
            interactable_area: Some(BorderRect {
                top: self.interactable_area.position.x as f32,
                left: self.interactable_area.position.y as f32,
                right: self.interactable_area.end().x as f32,
                bottom: self.interactable_area.end().y as f32,
            }),
        }
    }

    #[func]
    fn _on_ui_resize(&mut self) {
        self.ui_canvas_information = self.create_ui_canvas_information();

        // Update cached viewport center when viewport resizes
        let viewport = self.base().get_viewport();
        if let Some(viewport) = viewport {
            let viewport_size = viewport.get_visible_rect();
            self.viewport_center =
                Vector2::new(viewport_size.size.x * 0.5, viewport_size.size.y * 0.5);
        }
    }

    #[func]
    fn set_interactable_area(&mut self, interactable_area: Rect2i) {
        self.interactable_area = interactable_area;
        self.ui_canvas_information = self.create_ui_canvas_information();
    }

    #[func]
    pub fn get_current_parcel_scene_id(&self) -> i32 {
        self.current_parcel_scene_id.0
    }

    /// Updates the SDK-controlled skybox time from the current parcel scene.
    /// Reads the SkyboxTime component from the scene's root entity (if present)
    /// and exposes the values to GDScript for the skybox to use.
    fn update_sdk_skybox_time(&mut self) {
        // Reset to default (no SDK control) first
        let mut active = false;
        let mut fixed_time = 0u32;
        let mut transition_forward = true;

        // Get the current parcel scene
        if let Some(scene) = self.scenes.get(&self.current_parcel_scene_id) {
            // Try to lock the CRDT state
            if let Ok(crdt_state) = scene.dcl_scene.scene_crdt.try_lock() {
                // Read the SkyboxTime component from the root entity
                let skybox_time_component =
                    SceneCrdtStateProtoComponents::get_skybox_time(&crdt_state);

                if let Some(entry) = skybox_time_component.values.get(&SceneEntityId::ROOT) {
                    if let Some(skybox_time) = entry.value.as_ref() {
                        active = true;
                        fixed_time = skybox_time.fixed_time;
                        transition_forward =
                            skybox_time.transition_mode() != TransitionMode::TmBackward;
                    }
                }
            }
        }

        self.sdk_skybox_time_active = active;
        self.sdk_skybox_fixed_time = fixed_time;
        self.sdk_skybox_transition_forward = transition_forward;
    }

    fn on_current_parcel_scene_changed(&mut self) {
        // Reset input modifiers when changing scenes
        // The new scene's InputModifier (if any) will be applied on the next update tick
        if let Some(mut global) = DclGlobal::try_singleton() {
            global.bind_mut().reset_input_modifiers();
        }

        if let Some(scene) = self.scenes.get_mut(&self.last_current_parcel_scene_id) {
            for (_, audio_source_node) in scene.audio_sources.iter() {
                let mut audio_source_node = audio_source_node.clone();
                audio_source_node.bind_mut().set_dcl_enable(false);
                audio_source_node.call("apply_audio_props", &[false.to_variant()]);
            }
            for (_, audio_stream_node) in scene.audio_streams.iter_mut() {
                audio_stream_node.bind_mut().set_muted(true);
            }
            for (_, video_player_node) in scene.video_players.iter_mut() {
                video_player_node.bind_mut().set_muted(true);
            }

            scene
                .avatar_scene_updates
                .internal_player_data
                .insert(SceneEntityId::PLAYER, InternalPlayerData { inside: false });

            // leave it orphan! it will be re-added when you are in the scene, and deleted on scene deletion
            // Use call_deferred to avoid "Parent node is busy" errors during rapid scene transitions
            self.base_ui.call_deferred(
                "remove_child",
                &[scene.godot_dcl_scene.root_node_ui.clone().to_variant()],
            );
        }

        if let Some(scene) = self.scenes.get_mut(&self.current_parcel_scene_id) {
            for (_, audio_source_node) in scene.audio_sources.iter() {
                let mut audio_source_node = audio_source_node.clone();
                audio_source_node.bind_mut().set_dcl_enable(true);
                audio_source_node.call("apply_audio_props", &[false.to_variant()]);
            }
            for (_, audio_stream_node) in scene.audio_streams.iter_mut() {
                audio_stream_node.bind_mut().set_muted(false);
            }
            for (_, video_player_node) in scene.video_players.iter_mut() {
                video_player_node.bind_mut().set_muted(false);
            }

            scene
                .avatar_scene_updates
                .internal_player_data
                .insert(SceneEntityId::PLAYER, InternalPlayerData { inside: true });

            self.base_ui
                .add_child(&scene.godot_dcl_scene.root_node_ui.clone().upcast::<Node>());
        }

        self.last_current_parcel_scene_id = self.current_parcel_scene_id;
        let scene_id = Variant::from(self.current_parcel_scene_id.0);
        self.base_mut()
            .emit_signal("on_change_scene_id", &[scene_id]);
    }

    #[signal]
    fn on_change_scene_id(scene_id: i32);

    pub fn get_all_scenes_mut(&mut self) -> &mut HashMap<SceneId, Scene> {
        &mut self.scenes
    }

    pub fn get_all_scenes(&mut self) -> &HashMap<SceneId, Scene> {
        &self.scenes
    }

    pub fn get_scene_mut(&mut self, scene_id: &SceneId) -> Option<&mut Scene> {
        self.scenes.get_mut(scene_id)
    }

    pub fn get_scene(&self, scene_id: &SceneId) -> Option<&Scene> {
        self.scenes.get(scene_id)
    }

    pub fn get_global_scene_ids(&self) -> &Vec<SceneId> {
        &self.global_scene_ids
    }

    #[func]
    pub fn get_scene_entity_node_or_null_3d(
        &self,
        scene_id: i32,
        entity_id: u32,
    ) -> Option<Gd<Node3D>> {
        self.scenes.get(&SceneId(scene_id)).and_then(|x| {
            x.godot_dcl_scene
                .get_node_or_null_3d(&SceneEntityId::from_i32(entity_id as i32))
                .cloned()
        })
    }

    #[func]
    pub fn get_scene_virtual_camera(&self, scene_id: i32) -> Option<Gd<DclVirtualCamera>> {
        self.scenes
            .get(&SceneId(scene_id))
            .map(|x| x.virtual_camera.clone())
    }

    #[func]
    pub fn is_scene_tests_finished(&self, scene_id: i32) -> bool {
        let Some(scene) = self.scenes.get(&SceneId(scene_id)) else {
            return false;
        };

        scene.scene_test_plan_received
            && scene
                .scene_tests
                .iter()
                .all(|(_, test_result)| test_result.is_some())
    }

    #[func]
    pub fn get_scene_tests_result(&self, scene_id: i32) -> VarDictionary {
        let Some(scene) = self.scenes.get(&SceneId(scene_id)) else {
            return VarDictionary::default();
        };

        let test_total = scene.scene_tests.len() as u32;
        let mut test_fail = 0;
        let mut text_test_list = String::new();
        let mut text_detail_failed = String::new();
        for value in scene.scene_tests.iter() {
            if let Some(result) = value.1 {
                if result.ok {
                    text_test_list += &format!(
                        "\t🟢 {} (frames={},time={}): OK\n",
                        value.0, result.total_frames, result.total_time
                    );
                } else {
                    text_test_list += &format!(
                        "\t🔴 {} (frames={},time={}):",
                        value.0, result.total_frames, result.total_time
                    );
                    test_fail += 1;
                    if let Some(error) = &result.error {
                        text_test_list += "\tFAIL with Error\n";
                        text_detail_failed += &format!("🔴{}: ❌{}\n", value.0, error);
                    } else {
                        text_test_list += "\tFAIL with Unknown Error \n";
                    }
                }
            }
        }

        let mut text = format!(
            "Scene {:?} tests:\n",
            scene.scene_entity_definition.get_title()
        );
        text += &format!("{}\n", text_test_list);
        if test_fail == 0 {
            text += &format!(
                "✅ All tests ({}) passed in the scene {:?}\n",
                test_total,
                scene.scene_entity_definition.get_title()
            );
        } else {
            text += &format!(
                "❌ {} tests failed of {} in the scene {:?}\n",
                test_fail,
                test_total,
                scene.scene_entity_definition.get_title()
            );
        }

        let mut dict = VarDictionary::default();
        dict.set("text", text.to_variant());
        dict.set("text_detail_failed", text_detail_failed.to_variant());
        dict.set("total", test_total.to_variant());
        dict.set("fail", test_fail.to_variant());

        dict
    }

    /// Get total Deno/V8 memory usage across all scenes in MB
    #[func]
    pub fn get_total_deno_memory_mb(&self) -> f64 {
        self.scenes
            .values()
            .filter_map(|scene| scene.deno_memory_stats)
            .map(|stats| stats.used_heap_mb())
            .sum()
    }

    /// Get total Deno/V8 heap size across all scenes in MB
    #[func]
    pub fn get_total_deno_heap_size_mb(&self) -> f64 {
        self.scenes
            .values()
            .filter_map(|scene| scene.deno_memory_stats)
            .map(|stats| stats.total_heap_mb())
            .sum()
    }

    /// Get count of active Deno runtimes (scenes with memory stats)
    #[func]
    pub fn get_deno_scene_count(&self) -> i32 {
        self.scenes
            .values()
            .filter(|scene| scene.deno_memory_stats.is_some())
            .count() as i32
    }

    /// Get average Deno memory usage per scene in MB
    #[func]
    pub fn get_average_deno_memory_mb(&self) -> f64 {
        let count = self.get_deno_scene_count();
        if count > 0 {
            self.get_total_deno_memory_mb() / count as f64
        } else {
            0.0
        }
    }

    /// Get total Deno/V8 external memory across all scenes in MB
    /// External memory includes: typed arrays, ArrayBuffers, native bindings
    /// This is NOT included in used_heap_mb() and could be a significant leak source
    #[func]
    pub fn get_total_deno_external_memory_mb(&self) -> f64 {
        self.scenes
            .values()
            .filter_map(|scene| scene.deno_memory_stats)
            .map(|stats| stats.external_memory_mb())
            .sum()
    }

    /// Get count of alive scenes (with active threads)
    #[func]
    pub fn get_alive_scene_count(&self) -> i32 {
        self.scenes
            .values()
            .filter(|scene| scene.state == SceneState::Alive)
            .count() as i32
    }
}

#[godot_api]
impl INode for SceneManager {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        let mut base_ui = DclUiControl::new_alloc();
        base_ui.set_anchors_preset(LayoutPreset::FULL_RECT);
        base_ui.set_mouse_filter(MouseFilter::IGNORE);

        let canvas_size = base_ui.get_size();

        SceneManager {
            base,
            base_ui,
            ui_canvas_information: PbUiCanvasInformation::default(),

            scenes: HashMap::new(),
            pause: false,
            sorted_scene_ids: vec![],
            dying_scene_ids: vec![],
            global_scene_ids: vec![],
            current_parcel_scene_id: SceneId(0),
            last_current_parcel_scene_id: SceneId::INVALID,

            main_receiver_from_thread,
            thread_sender_to_main,

            player_avatar_node: Node3D::new_alloc(),
            player_body_node: Node3D::new_alloc(),

            player_position: Vector2i::new(-1000, -1000),

            total_time_seconds_time: 0.0,
            begin_time: Instant::now(),
            console: Callable::invalid(),
            input_state: InputState::default(),
            last_raycast_result: None,
            pointer_tooltips: VarArray::new(),
            interactable_area: Rect2i::from_components(
                0,
                0,
                canvas_size.x as i32,
                canvas_size.y as i32,
            ),
            viewport_center: Vector2::new(canvas_size.x * 0.5, canvas_size.y * 0.5),
            cursor_position: Vector2::new(canvas_size.x * 0.5, canvas_size.y * 0.5),
            raycast_use_cursor_position: false,
            cached_raycast_query: PhysicsRayQueryParameters3D::new_gd(),
            last_avatar_under_crosshair: None,
            avatar_pointer_press_time: None,
            pool_manager: RefCell::new(PoolManager::new()),
            current_loading_session: None,
            next_session_id: 0,
            sdk_skybox_time_active: false,
            sdk_skybox_fixed_time: 0,
            sdk_skybox_transition_forward: true,
        }
    }

    fn ready(&mut self) {
        let callable_on_ui_resize = self.base().callable("_on_ui_resize");

        self.base_ui.connect("resized", &callable_on_ui_resize);
        self.base_ui.set_name("scenes_ui");
        self.ui_canvas_information = self.create_ui_canvas_information();

        // Initialize cached viewport center
        let viewport = self.base().get_viewport();
        if let Some(viewport) = viewport {
            let viewport_size = viewport.get_visible_rect();
            self.viewport_center =
                Vector2::new(viewport_size.size.x * 0.5, viewport_size.size.y * 0.5);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        self.scene_runner_update(delta);

        // Check loading session timeouts
        self.check_loading_timeouts();

        // Note: Trigger area collision detection is now handled via PhysicsServer3D monitor callbacks
        // (area_set_monitor_callback). ENTER/EXIT events are processed in update_trigger_area.

        let changed_inputs = self.input_state.get_new_inputs();
        let current_raycast = self.get_current_mouse_entity();

        // Extract scene entity result for pointer events system
        let current_pointer_raycast_result = match &current_raycast {
            Some(RaycastResult::SceneEntity(entity)) => Some(entity.clone()),
            _ => None,
        };

        // Handle avatar detection
        match &current_raycast {
            Some(RaycastResult::Avatar(avatar)) => {
                // Update selected avatar if changed
                let avatar_changed = match &self.last_avatar_under_crosshair {
                    None => true,
                    Some(last) => last.instance_id() != avatar.instance_id(),
                };

                if avatar_changed {
                    self.last_avatar_under_crosshair = Some(avatar.clone());

                    // Update Global.selected_avatar directly
                    if let Some(mut global) = DclGlobal::try_singleton() {
                        global.bind_mut().selected_avatar = Some(avatar.clone());
                    }

                    // Emit signal for tooltip change
                    self.base_mut().emit_signal("pointer_tooltip_changed", &[]);
                }

                // Handle pointer press/release on avatar for profile opening
                // Check for pointer press (start timing)
                if changed_inputs.contains(&(InputAction::IaPointer, true)) {
                    // Record the time when pointer was pressed
                    self.avatar_pointer_press_time = Some(Instant::now());
                }

                // Check for pointer release (open profile if within time limit)
                if changed_inputs.contains(&(InputAction::IaPointer, false)) {
                    // Only open profile if:
                    // 1. We have a recorded press time
                    // 2. The release is within 1 second of the press
                    // 3. UI has focus
                    if let Some(press_time) = self.avatar_pointer_press_time {
                        let press_duration = Instant::now().duration_since(press_time);

                        // Check if release is within 1 second
                        if press_duration < Duration::from_secs(1) {
                            // Check if UI has focus using the Global singleton
                            let ui_has_focus = if let Some(global) = DclGlobal::try_singleton() {
                                global.bind().ui_has_focus()
                            } else {
                                true // Default to true if global not available
                            };

                            if ui_has_focus {
                                // Emit open_profile_by_avatar signal on the Global singleton
                                if let Some(mut global) = DclGlobal::try_singleton() {
                                    global.emit_signal(
                                        "open_profile_by_avatar",
                                        &[avatar.to_variant()],
                                    );
                                }
                            }
                        }

                        // Clear the press time after handling release
                        self.avatar_pointer_press_time = None;
                    }
                }
            }
            None | Some(RaycastResult::SceneEntity(_)) => {
                // Clear avatar selection if we're not hovering over an avatar
                if self.last_avatar_under_crosshair.is_some() {
                    self.last_avatar_under_crosshair = None;

                    // Clear the press time since we're no longer hovering over an avatar
                    self.avatar_pointer_press_time = None;

                    // Clear Global.selected_avatar
                    if let Some(mut global) = DclGlobal::try_singleton() {
                        global.bind_mut().selected_avatar = None;
                    }

                    // Emit signal for tooltip change
                    self.base_mut().emit_signal("pointer_tooltip_changed", &[]);
                }
            }
        }

        pointer_events_system(
            &mut self.scenes,
            &changed_inputs,
            &self.last_raycast_result,
            &current_pointer_raycast_result,
        );

        let mut tooltips = VarArray::new();
        if let Some(raycast) = current_pointer_raycast_result.as_ref() {
            if let Some(pointer_events) =
                get_entity_pointer_event(&self.scenes, &raycast.scene_id, &raycast.entity_id)
            {
                for pointer_event in pointer_events.pointer_events.iter() {
                    if let Some(info) = pointer_event.event_info.as_ref() {
                        let show_feedback = info.show_feedback.as_ref().unwrap_or(&true);
                        let max_distance = *info.max_distance.as_ref().unwrap_or(&10.0);
                        if !show_feedback || raycast.hit.length > max_distance {
                            continue;
                        }

                        let input_action =
                            InputAction::from_i32(*info.button.as_ref().unwrap_or(&0))
                                .unwrap_or(InputAction::IaAny);

                        let is_pet_up = pointer_event.event_type == PointerEventType::PetUp as i32;
                        let is_pet_down =
                            pointer_event.event_type == PointerEventType::PetDown as i32;
                        if is_pet_up || is_pet_down {
                            let text = if let Some(text) = info.hover_text.as_ref() {
                                text.to_godot()
                            } else {
                                GString::from("Interact")
                            };

                            let input_action_gstr = GString::from(input_action.as_str_name());

                            let dict = tooltips.iter_shared().find_map(|tooltip| {
                                let dictionary = tooltip.to::<VarDictionary>();
                                dictionary.get("action").and_then(|action| {
                                    if action.to_string() == input_action_gstr.to_string() {
                                        Some(dictionary.clone())
                                    } else {
                                        None
                                    }
                                })
                            });

                            let exists = dict.is_some();
                            let mut dict = dict.unwrap_or_else(VarDictionary::new);

                            if is_pet_down {
                                dict.set("text_pet_down", text.clone());
                            } else if is_pet_up {
                                dict.set("text_pet_up", text.clone());
                            }

                            dict.set("action", input_action_gstr.clone());

                            if !exists {
                                tooltips.push(&dict.to_variant());
                            }
                        }
                    }
                }
            }
        }

        // Add avatar profile tooltip if there's an avatar under crosshair with a valid ID
        // Skip AvatarShapes (NPCs from scenes) which don't have valid profile IDs
        if let Some(RaycastResult::Avatar(avatar)) = &current_raycast {
            // Check if avatar has a valid avatar_id (non-empty and not just "npc-*")
            let avatar_id: GString = avatar.get("avatar_id").try_to().unwrap_or_default();
            let is_avatar_shape: bool = avatar.get("is_avatar_shape").try_to().unwrap_or(false);
            if !is_avatar_shape && !avatar_id.is_empty() {
                let mut profile_dict = VarDictionary::new();
                profile_dict.set("text_pet_down", "View profile");
                profile_dict.set("action", "ia_pointer");
                tooltips.push(&profile_dict.to_variant());
            }
        }

        if self.pointer_tooltips != tooltips {
            self.pointer_tooltips = tooltips;
            self.base_mut().emit_signal("pointer_tooltip_changed", &[]);
        }

        if let Some(current_camera_node) =
            self.base().get_viewport().and_then(|x| x.get_camera_3d())
        {
            // Only update player/camera transforms for current scene and global scenes
            let player_transform = self.player_avatar_node.get_global_transform();
            let camera_transform = current_camera_node.get_global_transform();

            // Update current parcel scene
            if let Some(scene) = self.scenes.get_mut(&self.current_parcel_scene_id) {
                if let Some(scene_player_entity_node) = scene
                    .godot_dcl_scene
                    .get_node_or_null_3d_mut(&SceneEntityId::PLAYER)
                {
                    scene_player_entity_node.set_global_transform(player_transform);
                }

                if let Some(scene_camera_entity_node) = scene
                    .godot_dcl_scene
                    .get_node_or_null_3d_mut(&SceneEntityId::CAMERA)
                {
                    scene_camera_entity_node.set_global_transform(camera_transform);
                }
            }

            // Update global scenes
            for scene_id in self.get_global_scene_ids().clone() {
                if let Some(scene) = self.scenes.get_mut(&scene_id) {
                    if let Some(scene_player_entity_node) = scene
                        .godot_dcl_scene
                        .get_node_or_null_3d_mut(&SceneEntityId::PLAYER)
                    {
                        scene_player_entity_node.set_global_transform(player_transform);
                    }

                    if let Some(scene_camera_entity_node) = scene
                        .godot_dcl_scene
                        .get_node_or_null_3d_mut(&SceneEntityId::CAMERA)
                    {
                        scene_camera_entity_node.set_global_transform(camera_transform);
                    }
                }
            }
        }

        self.last_raycast_result = current_pointer_raycast_result;
        GLOBAL_TICK_NUMBER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
