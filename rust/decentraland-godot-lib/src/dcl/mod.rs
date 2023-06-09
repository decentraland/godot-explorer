pub mod components;
pub mod crdt;
pub mod js;
pub mod serialization;

use self::{
    components::{SceneComponentId, SceneEntityId},
    crdt::SceneCrdtState,
    js::scene_thread,
};

use deno_core::v8::IsolateHandle;
use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct SceneId(pub u32);

// scene metadata
#[derive(Clone, Default, Debug)]
pub struct SceneDefinition {
    pub path: String,
    pub base: godot::prelude::Vector2i,
    pub visible: bool,

    pub parcels: Vec<godot::prelude::Vector2i>,
    pub is_global: bool,
}

pub type DirtyComponents = HashMap<SceneComponentId, HashSet<SceneEntityId>>;

// message from scene-thread describing new and deleted entities
#[derive(Debug, Default)]
pub struct DirtyEntities {
    pub born: HashSet<SceneEntityId>,
    pub died: HashSet<SceneEntityId>,
}

// data from renderer to scene
#[derive(Debug)]
pub enum RendererResponse {
    Ok((DirtyEntities, DirtyComponents)),
    Kill,
}

// data from scene to renderer
#[derive(Debug)]
pub enum SceneResponse {
    Error(SceneId, String),
    Ok(SceneId, (DirtyEntities, DirtyComponents)),
}

static SCENE_ID_MONOTONIC_COUNTER: Lazy<AtomicU32> = Lazy::new(Default::default);
pub(crate) static VM_HANDLES: Lazy<Mutex<HashMap<SceneId, IsolateHandle>>> =
    Lazy::new(Default::default);

pub struct DclScene {
    pub scene_id: SceneId,
    pub scene_crdt: Arc<Mutex<SceneCrdtState>>,
    pub main_sender_to_thread: tokio::sync::mpsc::Sender<RendererResponse>,
    pub thread_join_handle: JoinHandle<()>,
}

impl DclScene {
    pub fn spawn_new(
        scene_definition: SceneDefinition,
        thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    ) -> Self {
        let id = SceneId(SCENE_ID_MONOTONIC_COUNTER.fetch_add(1, Ordering::Relaxed));
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
}
