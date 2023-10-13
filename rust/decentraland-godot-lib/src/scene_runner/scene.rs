use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use godot::prelude::{Dictionary, Gd};

use crate::{
    dcl::{
        components::{
            material::DclMaterial,
            proto_components::sdk::components::{common::RaycastHit, PbPointerEventsResult},
            SceneEntityId,
        },
        js::SceneLogMessage,
        // js::js_runtime::SceneLogMessage,
        DclScene,
        DirtyEntities,
        DirtyGosComponents,
        DirtyLwwComponents,
        RendererResponse,
        SceneDefinition,
        SceneId,
    },
    godot_classes::dcl_audio_source::DclAudioSource,
};

use super::godot_dcl_scene::GodotDclScene;

pub struct Dirty {
    pub waiting_process: bool,
    pub entities: DirtyEntities,
    pub lww_components: DirtyLwwComponents,
    pub gos_components: DirtyGosComponents,
    pub logs: Vec<SceneLogMessage>,
    pub renderer_response: Option<RendererResponse>,
    pub update_state: SceneUpdateState,
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
    TransformAndParent,
    VisibilityComponent,
    MeshRenderer,
    ScenePointerEvents,
    Material,
    TextShape,
    Billboard,
    MeshCollider,
    GltfContainer,
    Animator,
    AvatarShape,
    Raycasts,
    AvatarAttach,
    VideoPlayer,
    CameraModeArea,
    AudioSource,
    ComputeCrdtState,
    SendToThread,
    Processed,
}

impl SceneUpdateState {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::PrintLogs,
            Self::PrintLogs => Self::DeletedEntities,
            Self::DeletedEntities => Self::TransformAndParent,
            Self::TransformAndParent => Self::VisibilityComponent,
            Self::VisibilityComponent => Self::MeshRenderer,
            Self::MeshRenderer => Self::ScenePointerEvents,
            Self::ScenePointerEvents => Self::Material,
            Self::Material => Self::TextShape,
            Self::TextShape => Self::Billboard,
            Self::Billboard => Self::MeshCollider,
            Self::MeshCollider => Self::GltfContainer,
            Self::GltfContainer => Self::Animator,
            Self::Animator => Self::AvatarShape,
            Self::AvatarShape => Self::Raycasts,
            Self::Raycasts => Self::VideoPlayer,
            Self::VideoPlayer => Self::CameraModeArea,
            Self::CameraModeArea => Self::AudioSource,
            Self::AudioSource => Self::AvatarAttach,
            Self::AvatarAttach => Self::ComputeCrdtState,
            Self::ComputeCrdtState => Self::SendToThread,
            Self::SendToThread => Self::Processed,
            Self::Processed => Self::Processed,
        }
    }
}

pub enum SceneType {
    Parcel,
    Global,
    PortableExperience,
}

pub struct Scene {
    pub scene_id: SceneId,
    pub godot_dcl_scene: GodotDclScene,
    pub dcl_scene: DclScene,
    pub definition: SceneDefinition,

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

static SCENE_ID_MONOTONIC_COUNTER: once_cell::sync::Lazy<std::sync::atomic::AtomicU32> =
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
    ) -> Self {
        let godot_dcl_scene = GodotDclScene::new(&scene_definition, &scene_id);

        Self {
            scene_id,
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
            scene_type,
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
        let godot_dcl_scene = GodotDclScene::new(&scene_definition, &scene_id);

        Self {
            scene_id,
            godot_dcl_scene,
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
        }
    }
}
