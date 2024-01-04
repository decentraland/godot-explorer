use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use godot::{
    obj::UserClass,
    prelude::{Dictionary, Gd},
};

use crate::{
    dcl::{
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
        js::{testing::SceneTestResult, SceneLogMessage},
        scene_apis::RpcCall,
        // js::js_runtime::SceneLogMessage,
        DclScene,
        RendererResponse,
        SceneDefinition,
        SceneId,
    },
    godot_classes::{
        dcl_audio_source::DclAudioSource, dcl_audio_stream::DclAudioStream,
        dcl_ui_control::DclUiControl, dcl_video_player::DclVideoPlayer,
    },
};

use super::{components::tween::Tween, godot_dcl_scene::GodotDclScene};

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
    pub weak_ref: godot::prelude::Variant,
    pub waiting_textures: bool,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug)]
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
    NftShape,
    Animator,
    AvatarShape,
    Raycasts,
    AvatarAttach,
    SceneUi,
    VideoPlayer,
    AudioStream,
    AvatarModifierArea,
    CameraModeArea,
    AudioSource,
    ProcessRpcs,
    ComputeCrdtState,
    SendToThread,
    Processed,
}

impl SceneUpdateState {
    pub fn next(self) -> Self {
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
            Self::GltfContainer => Self::NftShape,
            Self::NftShape => Self::Animator,
            Self::Animator => Self::AvatarShape,
            Self::AvatarShape => Self::Raycasts,
            Self::Raycasts => Self::VideoPlayer,
            Self::VideoPlayer => Self::AudioStream,
            Self::AudioStream => Self::AvatarModifierArea,
            Self::AvatarModifierArea => Self::CameraModeArea,
            Self::CameraModeArea => Self::AudioSource,
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
    pub definition: SceneDefinition,
    pub tick_number: u32,

    pub state: SceneState,

    pub content_mapping: Dictionary,

    pub gltf_loading: HashSet<SceneEntityId>,
    pub pointer_events_result: Vec<(SceneEntityId, PbPointerEventsResult)>,
    pub continuos_raycast: HashSet<SceneEntityId>,

    pub current_dirty: Dirty,
    pub enqueued_dirty: Vec<Dirty>,
    pub distance: f32,

    pub start_time: Instant,
    pub last_tick_us: i64,
    pub next_tick_us: i64,

    pub materials: HashMap<DclMaterial, MaterialItem>,
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
    // Duplicated value to async-access the animator
    pub dup_animator: HashMap<SceneEntityId, PbAnimator>,
}

#[derive(Debug)]
pub struct GodotDclRaycastResult {
    pub scene_id: SceneId,
    pub entity_id: SceneEntityId,
    pub hit: RaycastHit,
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
        scene_definition: SceneDefinition,
        dcl_scene: DclScene,
        content_mapping: Dictionary,
        scene_type: SceneType,
        parent_ui_node: Gd<DclUiControl>,
    ) -> Self {
        let godot_dcl_scene = GodotDclScene::new(&scene_definition, &scene_id, parent_ui_node);

        Self {
            scene_id,
            tick_number: 0,
            godot_dcl_scene,
            definition: scene_definition,
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
            dup_animator: HashMap::new(),
        }
    }

    pub fn min_distance(&self, parcel_position: &godot::prelude::Vector2i) -> (f32, bool) {
        let diff = self.definition.base - *parcel_position;
        let mut distance_squared = diff.x * diff.x + diff.y * diff.y;
        for parcel in self.definition.parcels.iter() {
            let diff = *parcel - *parcel_position;
            distance_squared = distance_squared.min(diff.x * diff.x + diff.y * diff.y);
        }
        ((distance_squared as f32).sqrt(), distance_squared == 0)
    }

    pub fn unsafe_default() -> Self {
        let scene_definition = SceneDefinition::default();
        let scene_id = Scene::new_id();
        let dcl_scene = DclScene::spawn_new_test_scene(scene_id);
        let content_mapping = Dictionary::default();
        let godot_dcl_scene =
            GodotDclScene::new(&scene_definition, &scene_id, DclUiControl::alloc_gd());

        Self {
            scene_id,
            godot_dcl_scene,
            tick_number: 0,
            definition: scene_definition,
            dcl_scene,
            state: SceneState::Alive,
            enqueued_dirty: Vec::new(),
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
            distance: 0.0,
            next_tick_us: 0,
            last_tick_us: 0,
            gltf_loading: HashSet::new(),
            pointer_events_result: Vec::new(),
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
            dup_animator: HashMap::new(),
        }
    }
}
