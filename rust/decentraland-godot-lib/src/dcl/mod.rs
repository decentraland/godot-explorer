pub mod components;
pub mod crdt;
pub mod js;
pub mod serialization;

use self::{
    components::{SceneComponentId, SceneEntityId},
    crdt::SceneCrdtState,
    js::{js_runtime::SceneLogMessage, scene_thread},
};

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct SceneId(pub u32);

// scene metadata
#[derive(Clone, Default, Debug)]
pub struct SceneDefinition {
    pub path: String,
    pub main_crdt_path: String,
    pub base: godot::prelude::Vector2i,
    pub visible: bool,
    pub title: String,

    pub parcels: Vec<godot::prelude::Vector2i>,
    pub is_global: bool,
}

pub type DirtyLwwComponents = HashMap<SceneComponentId, HashSet<SceneEntityId>>;
pub type DirtyGosComponents = HashMap<SceneComponentId, HashMap<SceneEntityId, usize>>;

// message from scene-thread describing new and deleted entities
#[derive(Debug, Default)]
pub struct DirtyEntities {
    pub born: HashSet<SceneEntityId>,
    pub died: HashSet<SceneEntityId>,
}

// data from renderer to scene
#[derive(Debug)]
pub enum RendererResponse {
    Ok((DirtyEntities, DirtyLwwComponents, DirtyGosComponents)),
    Kill,
}

// data from scene to renderer
#[derive(Debug)]
pub enum SceneResponse {
    Error(SceneId, String),
    Ok(
        SceneId,
        (DirtyEntities, DirtyLwwComponents, DirtyGosComponents),
        Vec<SceneLogMessage>,
        f32,
    ),
}

pub type SharedSceneCrdtState = Arc<Mutex<SceneCrdtState>>;

pub struct DclScene {
    pub scene_id: SceneId,
    pub scene_crdt: SharedSceneCrdtState,
    pub main_sender_to_thread: tokio::sync::mpsc::Sender<RendererResponse>,
    pub thread_join_handle: JoinHandle<()>,
}

impl DclScene {
    pub fn spawn_new_js_dcl_scene(
        id: SceneId,
        scene_definition: SceneDefinition,
        thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    ) -> Self {
        let (main_sender_to_thread, thread_receive_from_renderer) =
            tokio::sync::mpsc::channel::<RendererResponse>(1);

        let scene_crdt = Arc::new(Mutex::new(SceneCrdtState::from_proto()));
        let thread_scene_crdt = scene_crdt.clone();

        let thread_join_handle = std::thread::Builder::new()
            .name(format!("scene thread {}", id.0))
            .spawn(move || {
                scene_thread(
                    id,
                    scene_definition,
                    thread_sender_to_main,
                    thread_receive_from_renderer,
                    thread_scene_crdt,
                )
            })
            .unwrap();

        DclScene {
            scene_id: id,
            scene_crdt,
            main_sender_to_thread,
            thread_join_handle,
        }
    }

    pub fn spawn_new_test_scene(id: SceneId) -> Self {
        let (main_sender_to_thread, _thread_receive_from_renderer) =
            tokio::sync::mpsc::channel::<RendererResponse>(1);

        let scene_crdt = Arc::new(Mutex::new(SceneCrdtState::from_proto()));

        let thread_join_handle = std::thread::Builder::new()
            .name(format!("scene thread {}", id.0))
            .spawn(move || {})
            .unwrap();

        DclScene {
            scene_id: id,
            scene_crdt,
            main_sender_to_thread,
            thread_join_handle,
        }
    }
}
