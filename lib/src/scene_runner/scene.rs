use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use godot::{obj::NewAlloc, prelude::Gd};

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
        dcl_ui_control::DclUiControl, dcl_video_player::DclVideoPlayer,
        dcl_virtual_camera::DclVirtualCamera,
    },
    realm::scene_definition::SceneEntityDefinition,
};

use super::{
    components::{trigger_area::TriggerAreaState, tween::Tween},
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
    NftShape,
    Animator,
    AvatarShape,
    AvatarShapeEmoteCommand,
    Raycasts,
    AvatarAttach,
    SceneUi,
    #[cfg(feature = "use_ffmpeg")]
    VideoPlayer,
    #[cfg(feature = "use_ffmpeg")]
    AudioStream,
    AvatarModifierArea,
    CameraModeArea,
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
        match &self {
            &Self::None => Self::PrintLogs,
            &Self::PrintLogs => Self::DeletedEntities,
            &Self::DeletedEntities => Self::Tween,
            &Self::Tween => Self::TransformAndParent,
            &Self::TransformAndParent => Self::VisibilityComponent,
            &Self::VisibilityComponent => Self::MeshRenderer,
            &Self::MeshRenderer => Self::ScenePointerEvents,
            &Self::ScenePointerEvents => Self::Material,
            &Self::Material => Self::TextShape,
            &Self::TextShape => Self::Billboard,
            &Self::Billboard => Self::MeshCollider,
            &Self::MeshCollider => Self::GltfContainer,
            &Self::GltfContainer => Self::SyncGltfContainer,
            &Self::SyncGltfContainer => Self::NftShape,
            &Self::NftShape => Self::Animator,
            &Self::Animator => Self::AvatarShape,
            &Self::AvatarShape => Self::AvatarShapeEmoteCommand,
            &Self::AvatarShapeEmoteCommand => Self::Raycasts,
            #[cfg(feature = "use_ffmpeg")]
            &Self::Raycasts => Self::VideoPlayer,
            #[cfg(feature = "use_ffmpeg")]
            &Self::VideoPlayer => Self::AudioStream,
            #[cfg(feature = "use_ffmpeg")]
            &Self::AudioStream => Self::AvatarModifierArea,
            #[cfg(not(feature = "use_ffmpeg"))]
            &Self::Raycasts => Self::AvatarModifierArea,
            &Self::AvatarModifierArea => Self::CameraModeArea,
            &Self::CameraModeArea => Self::TriggerArea,
            &Self::TriggerArea => Self::VirtualCameras,
            &Self::VirtualCameras => Self::AudioSource,
            &Self::AudioSource => Self::AvatarAttach,
            &Self::AvatarAttach => Self::SceneUi,
            &Self::SceneUi => Self::ProcessRpcs,
            &Self::ProcessRpcs => Self::ComputeCrdtState,
            &Self::ComputeCrdtState => Self::SendToThread,
            &Self::SendToThread => Self::Processed,
            &Self::Processed => Self::Processed,
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

    pub virtual_camera: Gd<DclVirtualCamera>,

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
            pointer_events_result: Vec::new(),
            trigger_area_results: Vec::new(),
            continuos_raycast: HashSet::new(),
            start_time: Instant::now(),
            materials: HashMap::new(),
            dirty_materials: false,
            audio_sources: HashMap::new(),
            audio_streams: HashMap::new(),
            video_players: HashMap::new(),
            scene_type,
            avatar_scene_updates: Default::default(),
            scene_tests: HashMap::new(),
            scene_test_plan_received: false,
            tweens: HashMap::new(),
            texture_animations: HashMap::new(),
            dup_animator: HashMap::new(),
            trigger_areas: TriggerAreaState::default(),
            paused: false,
            virtual_camera: Default::default(),
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
            avatar_scene_updates: Default::default(),
            scene_tests: HashMap::new(),
            scene_test_plan_received: false,
            tweens: HashMap::new(),
            texture_animations: HashMap::new(),
            dup_animator: HashMap::new(),
            trigger_areas: TriggerAreaState::default(),
            paused: false,
            virtual_camera: Default::default(),
            deno_memory_stats: None,
        }
    }
}
