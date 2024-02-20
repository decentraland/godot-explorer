pub mod common;
pub mod components;
pub mod crdt;
#[cfg(feature = "use_deno")]
pub mod js;
pub mod scene_apis;
pub mod serialization;

use ethers::types::H160;
use godot::builtin::{Vector2, Vector3};

use crate::{
    auth::{ephemeral_auth_chain::EphemeralAuthChain, ethereum_provider::EthereumProvider},
    content::content_mapping::ContentMappingAndUrlRef,
    realm::scene_definition::SceneEntityDefinition,
};

use self::{
    common::{
        SceneLogMessage, TakeAndCompareSnapshotResponse, TestingScreenshotComparisonMethodRequest,
    },
    crdt::{DirtyCrdtState, SceneCrdtState},
    scene_apis::{RpcCall, RpcResultSender},
};

#[cfg(feature = "use_deno")]
use self::js::scene_thread;

use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct SceneId(pub i32);

impl SceneId {
    pub const INVALID: SceneId = SceneId(-1);
}

// data from renderer to scene
#[derive(Debug)]
pub enum RendererResponse {
    Ok {
        dirty_crdt_state: Box<DirtyCrdtState>,
        incoming_comms_message: Vec<(H160, Vec<u8>)>,
    },
    Kill,
}

// data from scene to renderer
#[derive(Debug)]
pub enum SceneResponse {
    Error(SceneId, String),
    Ok {
        scene_id: SceneId,
        dirty_crdt_state: DirtyCrdtState,
        logs: Vec<SceneLogMessage>,
        delta: f32,
        rpc_calls: Vec<RpcCall>,
    },
    RemoveGodotScene(SceneId, Vec<SceneLogMessage>),
    TakeSnapshot {
        scene_id: SceneId,
        src_stored_snapshot: String,
        camera_position: Vector3,
        camera_target: Vector3,
        screeshot_size: Vector2,
        method: TestingScreenshotComparisonMethodRequest,
        response: RpcResultSender<Result<TakeAndCompareSnapshotResponse, String>>,
    },
}

pub type SharedSceneCrdtState = Arc<Mutex<SceneCrdtState>>;

pub struct DclScene {
    pub scene_id: SceneId,
    pub scene_crdt: SharedSceneCrdtState,
    pub main_sender_to_thread: tokio::sync::mpsc::Sender<RendererResponse>,
    pub thread_join_handle: JoinHandle<()>,
}

impl DclScene {
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_new_js_dcl_scene(
        id: SceneId,
        scene_entity_definition: Arc<SceneEntityDefinition>,
        local_main_js_file_path: String,
        local_main_crdt_file_path: String,
        content_mapping: ContentMappingAndUrlRef,
        thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
        testing_mode: bool,
        ethereum_provider: Arc<EthereumProvider>,
        ephemeral_wallet: Option<EphemeralAuthChain>,
    ) -> Self {
        let (main_sender_to_thread, thread_receive_from_renderer) =
            tokio::sync::mpsc::channel::<RendererResponse>(1);

        let scene_crdt = Arc::new(Mutex::new(SceneCrdtState::from_proto()));
        let thread_scene_crdt = scene_crdt.clone();

        let thread_join_handle = std::thread::Builder::new()
            .name(format!("scene thread {}", id.0))
            .spawn(move || {
                #[cfg(feature = "use_deno")]
                scene_thread(
                    id,
                    scene_entity_definition,
                    local_main_js_file_path,
                    local_main_crdt_file_path,
                    content_mapping,
                    thread_sender_to_main,
                    thread_receive_from_renderer,
                    thread_scene_crdt,
                    testing_mode,
                    ethereum_provider,
                    ephemeral_wallet,
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
