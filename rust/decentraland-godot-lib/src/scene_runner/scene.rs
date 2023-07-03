use std::{collections::HashSet, time::Instant};

use godot::prelude::Gd;

use crate::dcl::{
    components::{
        proto_components::sdk::components::{common::RaycastHit, PbPointerEventsResult},
        SceneEntityId,
    },
    js::SceneLogMessage,
    DclScene, DirtyEntities, DirtyGosComponents, DirtyLwwComponents, SceneDefinition, SceneId,
};

use super::{content::ContentMapping, godot_dcl_scene::GodotDclScene};

pub struct Dirty {
    pub waiting_process: bool,
    pub entities: DirtyEntities,
    pub lww_components: DirtyLwwComponents,
    pub gos_components: DirtyGosComponents,
    pub logs: Vec<SceneLogMessage>,
}

pub enum SceneState {
    Alive,
    ToKill,
    KillSignal(i64),
    Dead,
}

pub struct Scene {
    pub scene_id: SceneId,
    pub godot_dcl_scene: GodotDclScene,
    pub dcl_scene: DclScene,
    pub definition: SceneDefinition,

    pub waiting_for_updates: bool,
    pub state: SceneState,

    pub content_mapping: godot::prelude::Gd<ContentMapping>,

    pub gltf_loading: HashSet<SceneEntityId>,
    pub pointer_events_result: Vec<(SceneEntityId, PbPointerEventsResult)>,
    pub continuos_raycast: HashSet<SceneEntityId>,

    pub current_dirty: Dirty,
    pub distance: f32,

    pub start_time: Instant,
    pub last_tick_us: i64,
    pub next_tick_us: i64,
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
        content_mapping: godot::prelude::Gd<ContentMapping>,
    ) -> Self {
        let godot_dcl_scene = GodotDclScene::new(&scene_definition, &scene_id);

        Self {
            scene_id,
            godot_dcl_scene,
            definition: scene_definition,
            dcl_scene,
            waiting_for_updates: false,
            state: SceneState::Alive,

            content_mapping,
            current_dirty: Dirty {
                waiting_process: true,
                entities: DirtyEntities::default(),
                lww_components: DirtyLwwComponents::default(),
                gos_components: DirtyGosComponents::default(),
                logs: Vec::new(),
            },
            distance: 0.0,
            next_tick_us: 0,
            last_tick_us: 0,
            gltf_loading: HashSet::new(),
            pointer_events_result: Vec::new(),
            continuos_raycast: HashSet::new(),
            start_time: Instant::now(),
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

    pub fn default() -> Self {
        let scene_definition = SceneDefinition::default();
        let scene_id = Scene::new_id();
        let dcl_scene = DclScene::spawn_new_test_scene(scene_id);
        let content_mapping = Gd::<ContentMapping>::new_default();
        let godot_dcl_scene = GodotDclScene::new(&scene_definition, &scene_id);

        Self {
            scene_id,
            godot_dcl_scene,
            definition: scene_definition,
            dcl_scene,
            waiting_for_updates: false,
            state: SceneState::Alive,

            content_mapping,
            current_dirty: Dirty {
                waiting_process: true,
                entities: DirtyEntities::default(),
                lww_components: DirtyLwwComponents::default(),
                gos_components: DirtyGosComponents::default(),
                logs: Vec::new(),
            },
            distance: 0.0,
            next_tick_us: 0,
            last_tick_us: 0,
            gltf_loading: HashSet::new(),
            pointer_events_result: Vec::new(),
            continuos_raycast: HashSet::new(),
            start_time: Instant::now(),
        }
    }
}
