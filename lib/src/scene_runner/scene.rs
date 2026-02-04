use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use godot::{
    obj::{NewAlloc, Singleton},
    prelude::Gd,
    prelude::ToGodot,
};

use crate::{
    content::content_mapping::{ContentMappingAndUrl, ContentMappingAndUrlRef},
    dcl::{
        common::{SceneLogMessage, SceneTestResult},
        components::{
            internal_player_data::InternalPlayerData,
            material::DclMaterial,
            proto_components::sdk::components::{
                common::RaycastHit, PbAnimator, PbAvatarBase, PbAvatarEmoteCommand,
                PbAvatarEquippedData, PbPlayerIdentityData, PbPointerEventsResult,
            },
            transform_and_parent::DclTransformAndParent,
            SceneEntityId,
        },
        crdt::{DirtyEntities, DirtyGosComponents, DirtyLwwComponents},
        scene_apis::RpcCall,
        DclScene, RendererResponse, SceneId,
    },
    godot_classes::{
        dcl_audio_source::DclAudioSource, dcl_audio_stream::DclAudioStream,
        dcl_locomotion_settings::DclLocomotionSettings, dcl_ui_control::DclUiControl,
        dcl_video_player::DclVideoPlayer, dcl_virtual_camera::DclVirtualCamera,
    },
    realm::scene_definition::SceneEntityDefinition,
};

use super::{
    components::{
        gltf_node_modifiers::GltfNodeModifierState, trigger_area::TriggerAreaState, tween::Tween,
    },
    godot_dcl_scene::GodotDclScene,
};

/// State for texture UV animation (used by TextureMove and TextureMoveContinuous tweens)
#[derive(Debug, Clone)]
pub struct TextureAnimation {
    /// Accumulated UV offset (for TMT_OFFSET mode)
    pub uv_offset: godot::builtin::Vector2,
    /// Accumulated UV scale multiplier (for TMT_TILING mode, starts at 1.0)
    pub uv_scale: godot::builtin::Vector2,
}

impl Default for TextureAnimation {
    fn default() -> Self {
        Self {
            uv_offset: godot::builtin::Vector2::ZERO,
            uv_scale: godot::builtin::Vector2::new(1.0, 1.0),
        }
    }
}

pub struct Dirty {
    pub waiting_process: bool,
    pub entities: DirtyEntities,
    pub lww_components: DirtyLwwComponents,
    pub gos_components: DirtyGosComponents,
    pub logs: Vec<SceneLogMessage>,
    pub renderer_response: Option<RendererResponse>,
    pub update_state: SceneUpdateState,
    pub rpc_calls: Vec<RpcCall>,
}

#[derive(PartialEq)]
pub enum SceneState {
    Alive,
    ToKill,
    KillSignal(i64),
    Dead,
}

pub struct MaterialItem {
    pub dcl_mat: DclMaterial,
    pub weak_ref: godot::prelude::Variant,
    pub waiting_textures: bool,
    pub alive: bool,
}

#[derive(Debug)]
pub struct PartialIteratorState {
    pub current_index: usize,
    pub items: Vec<SceneEntityId>,
}

#[derive(Debug)]
pub enum SceneUpdateState {
    None,
    PrintLogs,
    DeletedEntities,
    Tween,
    TransformAndParent,
    VisibilityComponent,
    MeshRenderer,
    ScenePointerEvents,
    Material,
    TextShape,
    Billboard,
    MeshCollider,
    GltfContainer,
    SyncGltfContainer,
    GltfNodeModifiers,
    NftShape,
    Animator,
    AvatarShape,
    AvatarShapeEmoteCommand,
    Raycasts,
    AvatarAttach,
    SceneUi,
    VideoPlayer,
    AudioStream,
    AvatarModifierArea,
    AvatarLocomotionSettings,
    CameraModeArea,
    InputModifier,
    SkyboxTime,
    TriggerArea,
    VirtualCameras,
    AudioSource,
    ProcessRpcs,
    ComputeCrdtState,
    SendToThread,
    Processed,
}

impl SceneUpdateState {
    pub fn next(&self) -> Self {
        match self {
            Self::None => Self::PrintLogs,
            Self::PrintLogs => Self::DeletedEntities,
            Self::DeletedEntities => Self::Tween,
            Self::Tween => Self::TransformAndParent,
            Self::TransformAndParent => Self::VisibilityComponent,
            Self::VisibilityComponent => Self::MeshRenderer,
            Self::MeshRenderer => Self::ScenePointerEvents,
            Self::ScenePointerEvents => Self::Material,
            Self::Material => Self::TextShape,
            Self::TextShape => Self::Billboard,
            Self::Billboard => Self::MeshCollider,
            Self::MeshCollider => Self::GltfContainer,
            Self::GltfContainer => Self::SyncGltfContainer,
            Self::SyncGltfContainer => Self::GltfNodeModifiers,
            Self::GltfNodeModifiers => Self::NftShape,
            Self::NftShape => Self::Animator,
            Self::Animator => Self::AvatarShape,
            Self::AvatarShape => Self::AvatarShapeEmoteCommand,
            Self::AvatarShapeEmoteCommand => Self::Raycasts,
            Self::Raycasts => Self::VideoPlayer,
            Self::VideoPlayer => Self::AudioStream,
            Self::AudioStream => Self::AvatarModifierArea,
            Self::AvatarModifierArea => Self::AvatarLocomotionSettings,
            Self::AvatarLocomotionSettings => Self::CameraModeArea,
            Self::CameraModeArea => Self::InputModifier,
            Self::InputModifier => Self::SkyboxTime,
            Self::SkyboxTime => Self::TriggerArea,
            Self::TriggerArea => Self::VirtualCameras,
            Self::VirtualCameras => Self::AudioSource,
            Self::AudioSource => Self::AvatarAttach,
            Self::AvatarAttach => Self::SceneUi,
            Self::SceneUi => Self::ProcessRpcs,
            Self::ProcessRpcs => Self::ComputeCrdtState,
            Self::ComputeCrdtState => Self::SendToThread,
            Self::SendToThread => Self::Processed,
            Self::Processed => Self::Processed,
        }
    }
}

#[derive(Clone)]
pub enum SceneType {
    Parcel,
    Global(GlobalSceneType),
}

#[derive(Clone)]
pub enum GlobalSceneType {
    GlobalRealm,
    SmartWearable,
    PortableExperience,
}

#[derive(Default)]
pub struct SceneAvatarUpdates {
    pub internal_player_data: HashMap<SceneEntityId, InternalPlayerData>,
    pub transform: HashMap<SceneEntityId, Option<DclTransformAndParent>>,
    pub player_identity_data: HashMap<SceneEntityId, PbPlayerIdentityData>,
    pub avatar_base: HashMap<SceneEntityId, PbAvatarBase>,
    pub avatar_equipped_data: HashMap<SceneEntityId, PbAvatarEquippedData>,
    pub pointer_events_result: HashMap<SceneEntityId, Vec<PbPointerEventsResult>>,
    pub avatar_emote_command: HashMap<SceneEntityId, Vec<PbAvatarEmoteCommand>>,
    pub deleted_entities: HashSet<SceneEntityId>,
}

pub struct Scene {
    pub scene_id: SceneId,
    pub godot_dcl_scene: GodotDclScene,
    pub dcl_scene: DclScene,
    pub scene_entity_definition: Arc<SceneEntityDefinition>,
    pub tick_number: u32,

    pub state: SceneState,

    pub content_mapping: ContentMappingAndUrlRef,

    pub gltf_loading: HashSet<SceneEntityId>,
    /// Count of GLTF entities that started loading (for loading session tracking)
    pub gltf_loading_started_count: u32,
    /// Count of GLTF entities that finished loading (for loading session tracking)
    pub gltf_loading_finished_count: u32,
    /// Whether this scene has been reported as ready to the loading session
    pub loading_reported_ready: bool,
    pub pointer_events_result: Vec<(SceneEntityId, PbPointerEventsResult)>,
    pub trigger_area_results: Vec<(
        SceneEntityId,
        crate::dcl::components::proto_components::sdk::components::PbTriggerAreaResult,
    )>,
    pub continuos_raycast: HashSet<SceneEntityId>,

    pub current_dirty: Dirty,
    pub enqueued_dirty: Vec<Dirty>,
    pub distance: f32,

    pub start_time: Instant,
    pub last_tick_us: i64,
    pub next_tick_us: i64,

    pub materials: HashMap<SceneEntityId, MaterialItem>,
    pub dirty_materials: bool,

    pub scene_type: SceneType,
    pub audio_sources: HashMap<SceneEntityId, Gd<DclAudioSource>>,

    // Used by VideoPlayer and AudioStream
    pub audio_streams: HashMap<SceneEntityId, Gd<DclAudioStream>>,
    pub video_players: HashMap<SceneEntityId, Gd<DclVideoPlayer>>,

    // Tracks entities with livekit video players
    pub livekit_video_player_entities: HashSet<SceneEntityId>,

    pub avatar_scene_updates: SceneAvatarUpdates,
    pub scene_tests: HashMap<String, Option<SceneTestResult>>,
    pub scene_test_plan_received: bool,

    // Tween
    pub tweens: HashMap<SceneEntityId, Tween>,
    // Texture animations (UV offset/scale) driven by TextureMove/TextureMoveContinuous tweens
    pub texture_animations: HashMap<SceneEntityId, TextureAnimation>,
    // Duplicated value to async-access the animator
    pub dup_animator: HashMap<SceneEntityId, PbAnimator>,

    // Trigger Areas
    pub trigger_areas: TriggerAreaState,

    // GltfNodeModifiers - state tracking for restoration
    pub gltf_node_modifier_states: HashMap<SceneEntityId, GltfNodeModifierState>,
    // Entities pending GltfNodeModifiers re-application after GLTF loads
    pub gltf_node_modifiers_pending: HashSet<SceneEntityId>,
    /// Last known player scene - used to detect when player enters/leaves this scene
    /// for trigger area activation. Initialized to invalid (-1) so first check detects transition.
    pub last_player_scene_id: SceneId,

    pub virtual_camera: Gd<DclVirtualCamera>,

    pub locomotion_settings: Gd<DclLocomotionSettings>,

    pub paused: bool,

    // Deno/V8 memory statistics for this scene
    pub deno_memory_stats: Option<crate::dcl::DenoMemoryStats>,
}

#[derive(Debug, Clone)]
pub struct GodotDclRaycastResult {
    pub scene_id: SceneId,
    pub entity_id: SceneEntityId,
    pub hit: RaycastHit,
}

#[derive(Debug)]
pub enum RaycastResult {
    SceneEntity(GodotDclRaycastResult),
    Avatar(godot::prelude::Gd<crate::godot_classes::dcl_avatar::DclAvatar>),
}

impl GodotDclRaycastResult {
    pub fn eq_key(a: &Option<GodotDclRaycastResult>, b: &Option<GodotDclRaycastResult>) -> bool {
        if a.is_some() && b.is_some() {
            let a = a.as_ref().unwrap();
            let b = b.as_ref().unwrap();
            a.scene_id == b.scene_id && a.entity_id == b.entity_id
        } else {
            a.is_none() && b.is_none()
        }
    }

    // pub fn get_hit(&self) -> RaycastHit {
    //     RaycastHit {
    //         // pub position: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub global_origin: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub direction: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub normal_hit: ::core::option::Option<super::super::super::common::Vector3>,
    //         // pub length: f32,
    //         // pub mesh_name: ::core::option::Option<::prost::alloc::string::String>,
    //         // pub entity_id: ::core::option::Option<u32>,
    //     }
    // }
}

static SCENE_ID_MONOTONIC_COUNTER: once_cell::sync::Lazy<std::sync::atomic::AtomicI32> =
    once_cell::sync::Lazy::new(Default::default);

impl Scene {
    pub fn new_id() -> SceneId {
        SceneId(SCENE_ID_MONOTONIC_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }

    pub fn new(
        scene_id: SceneId,
        scene_entity_definition: Arc<SceneEntityDefinition>,
        dcl_scene: DclScene,
        content_mapping: ContentMappingAndUrlRef,
        scene_type: SceneType,
        parent_ui_node: Gd<DclUiControl>,
    ) -> Self {
        let godot_dcl_scene =
            GodotDclScene::new(scene_entity_definition.clone(), &scene_id, parent_ui_node);

        Self {
            scene_id,
            tick_number: 0,
            godot_dcl_scene,
            scene_entity_definition,
            dcl_scene,
            state: SceneState::Alive,

            content_mapping,
            current_dirty: Dirty {
                waiting_process: true,
                entities: DirtyEntities::default(),
                lww_components: DirtyLwwComponents::default(),
                gos_components: DirtyGosComponents::default(),
                logs: Vec::new(),
                renderer_response: None,
                update_state: SceneUpdateState::None,
                rpc_calls: Vec::new(),
            },
            enqueued_dirty: Vec::new(),
            distance: 0.0,
            next_tick_us: 0,
            last_tick_us: 0,
            gltf_loading: HashSet::new(),
            gltf_loading_started_count: 0,
            gltf_loading_finished_count: 0,
            loading_reported_ready: false,
            pointer_events_result: Vec::new(),
            trigger_area_results: Vec::new(),
            continuos_raycast: HashSet::new(),
            start_time: Instant::now(),
            materials: HashMap::new(),
            dirty_materials: false,
            audio_sources: HashMap::new(),
            audio_streams: HashMap::new(),
            video_players: HashMap::new(),
            livekit_video_player_entities: HashSet::new(),
            scene_type,
            avatar_scene_updates: Default::default(),
            scene_tests: HashMap::new(),
            scene_test_plan_received: false,
            tweens: HashMap::new(),
            texture_animations: HashMap::new(),
            dup_animator: HashMap::new(),
            trigger_areas: TriggerAreaState::default(),
            gltf_node_modifier_states: HashMap::new(),
            gltf_node_modifiers_pending: HashSet::new(),
            last_player_scene_id: SceneId(-1), // Sentinel: never matches real scene IDs
            paused: false,
            virtual_camera: Default::default(),
            locomotion_settings: Default::default(),
            deno_memory_stats: None,
        }
    }

    pub fn min_distance(&self, parcel_position: &godot::prelude::Vector2i) -> (f32, bool) {
        let diff = self.scene_entity_definition.get_base_parcel() - *parcel_position;
        let mut distance_squared = diff.x * diff.x + diff.y * diff.y;
        for parcel in self.scene_entity_definition.get_parcels() {
            let diff = *parcel - *parcel_position;
            distance_squared = distance_squared.min(diff.x * diff.x + diff.y * diff.y);
        }
        ((distance_squared as f32).sqrt(), distance_squared == 0)
    }

    pub fn unsafe_default() -> Self {
        let scene_entity_definition = Arc::new(SceneEntityDefinition::default());
        let scene_id = Scene::new_id();
        let dcl_scene = DclScene::spawn_new_test_scene(scene_id);
        let godot_dcl_scene = GodotDclScene::new(
            scene_entity_definition.clone(),
            &scene_id,
            DclUiControl::new_alloc(),
        );

        Self {
            scene_id,
            godot_dcl_scene,
            tick_number: 0,
            scene_entity_definition,
            dcl_scene,
            state: SceneState::Alive,
            enqueued_dirty: Vec::new(),
            content_mapping: Arc::new(ContentMappingAndUrl::new()),
            current_dirty: Dirty {
                waiting_process: true,
                entities: DirtyEntities::default(),
                lww_components: DirtyLwwComponents::default(),
                gos_components: DirtyGosComponents::default(),
                logs: Vec::new(),
                renderer_response: None,
                update_state: SceneUpdateState::None,
                rpc_calls: Vec::new(),
            },
            distance: 0.0,
            next_tick_us: 0,
            last_tick_us: 0,
            gltf_loading: HashSet::new(),
            gltf_loading_started_count: 0,
            gltf_loading_finished_count: 0,
            loading_reported_ready: false,
            pointer_events_result: Vec::new(),
            trigger_area_results: Vec::new(),
            continuos_raycast: HashSet::new(),
            start_time: Instant::now(),
            materials: HashMap::new(),
            dirty_materials: false,
            scene_type: SceneType::Parcel,
            audio_sources: HashMap::new(),
            audio_streams: HashMap::new(),
            video_players: HashMap::new(),
            livekit_video_player_entities: HashSet::new(),
            avatar_scene_updates: Default::default(),
            scene_tests: HashMap::new(),
            scene_test_plan_received: false,
            tweens: HashMap::new(),
            texture_animations: HashMap::new(),
            dup_animator: HashMap::new(),
            trigger_areas: TriggerAreaState::default(),
            gltf_node_modifier_states: HashMap::new(),
            gltf_node_modifiers_pending: HashSet::new(),
            last_player_scene_id: SceneId(-1), // Sentinel: never matches real scene IDs
            paused: false,
            virtual_camera: Default::default(),
            locomotion_settings: Default::default(),
            deno_memory_stats: None,
        }
    }

    pub fn register_livekit_video_player(&mut self, entity_id: SceneEntityId) {
        self.livekit_video_player_entities.insert(entity_id);
        tracing::debug!(
            "Registered livekit video player entity {}",
            entity_id.as_i32()
        );
    }

    pub fn process_livekit_video_frame(&mut self, width: u32, height: u32, data: &[u8]) {
        use crate::godot_classes::dcl_video_player::VIDEO_STATE_PLAYING;
        use crate::scene_runner::components::video_player::update_video_texture_from_livekit;
        use godot::classes::Time;

        let current_time = Time::singleton().get_ticks_msec() as f64 / 1000.0;

        // Send video frames to all registered livekit video players
        for entity_id in self.livekit_video_player_entities.clone() {
            // Update the texture
            if let Some(node) = self.godot_dcl_scene.get_godot_entity_node_mut(&entity_id) {
                if let Some(vp_data) = &mut node.video_player_data {
                    update_video_texture_from_livekit(vp_data, width, height, data);
                }
            }

            // Update video player state to PLAYING when receiving frames
            if let Some(video_player) = self.video_players.get_mut(&entity_id) {
                video_player.set("video_state", &VIDEO_STATE_PLAYING.to_variant());
                video_player.set("video_length", &(-1.0_f64).to_variant());
                video_player.set("last_frame_time", &current_time.to_variant());
            }
        }
    }

    pub fn init_livekit_audio(
        &mut self,
        sample_rate: u32,
        num_channels: u32,
        samples_per_channel: u32,
    ) {
        tracing::debug!(
            "Livekit audio initialized: sample_rate={}, channels={}, samples_per_channel={}",
            sample_rate,
            num_channels,
            samples_per_channel
        );

        // Configure the AudioStreamGenerator with the correct sample rate for all livekit video players
        for entity_id in self.livekit_video_player_entities.clone() {
            if let Some(video_player) = self.video_players.get_mut(&entity_id) {
                video_player.call(
                    "init_livekit_audio",
                    &[
                        sample_rate.to_variant(),
                        num_channels.to_variant(),
                        samples_per_channel.to_variant(),
                    ],
                );
            }
        }
    }

    pub fn process_livekit_audio_frame(&mut self, frame: godot::prelude::PackedVector2Array) {
        // Send audio frames to all registered livekit video players
        for entity_id in self.livekit_video_player_entities.clone() {
            if let Some(video_player) = self.video_players.get_mut(&entity_id) {
                // Call the stream_buffer method on the video player (GDScript)
                video_player.call("stream_buffer", &[frame.to_variant()]);
            }
        }
    }

    /// Cleanup all scene resources before destruction.
    /// This ensures all Godot nodes are properly freed and references are cleared.
    pub fn cleanup(&mut self) {
        // Free audio sources
        for (_, mut audio_source) in self.audio_sources.drain() {
            audio_source.queue_free();
        }

        // Free audio streams
        for (_, mut audio_stream) in self.audio_streams.drain() {
            audio_stream.queue_free();
        }

        // Free video players
        for (_, mut video_player) in self.video_players.drain() {
            video_player.queue_free();
        }

        // Virtual camera is RefCounted, no need to queue_free - it's freed when references drop
        // Just clear it to drop the reference
        self.virtual_camera.bind_mut().clear();

        // Clear other collections
        self.gltf_loading.clear();
        self.materials.clear();
        self.tweens.clear();
        self.dup_animator.clear();
        self.livekit_video_player_entities.clear();

        // Cleanup godot_dcl_scene (frees all entity nodes and root nodes)
        self.godot_dcl_scene.cleanup();
    }
}
