//! Scene-requested actions on the engine side: who is asked, and how the scene's RPC is answered.

use std::rc::Rc;

use dcl_core::app::EngineEvent;
use dcl_core::scene_requests::{PendingRequests, RequestId, SceneRequest};
use godot::prelude::*;

use super::local_task::defer;
use super::DclEngine;
use crate::{dcl::scene_apis::RpcResultSender, godot_classes::dcl_global::DclGlobal};

pub type SceneResponse = RpcResultSender<Result<(), String>>;

/// What a sink did with a request.
pub enum Verdict {
    /// The RPC is answered now.
    Done(Result<(), String>),
    /// The sink answers later through `DclEngine::answer_scene_request`.
    Pending,
}

/// The app side of a scene request: the shell renders the modals; without one the emulator answers.
pub trait SceneRequestSink {
    fn handle(&self, id: RequestId, request: &SceneRequest) -> Verdict;
}

pub type SharedSink = Rc<dyn SceneRequestSink>;
pub type PendingResponses = PendingRequests<SceneResponse>;

impl DclEngine {
    /// Routes a scene's request to the active sink and answers the RPC when the sink says so.
    pub fn scene_request(request: SceneRequest, response: SceneResponse) {
        let Some(mut engine) = DclEngine::get() else {
            response.send(Err("engine not available".to_string()));
            return;
        };
        // No borrow of the engine is held while the sink runs: it calls into GDScript.
        let (id, sink) = {
            let mut engine = engine.bind_mut();
            (engine.pending.submit(response), engine.sink.clone())
        };
        if let Verdict::Done(result) = sink.handle(id, &request) {
            let responder = engine.bind_mut().pending.take(id);
            if let Some(responder) = responder {
                responder.send(result);
            }
        }
    }
}

/// The shell renders the modal: the request goes to the core as an event and the RPC answers when
/// the shell sends `Control::AnswerSceneRequest`.
pub struct ShellSink;

impl SceneRequestSink for ShellSink {
    fn handle(&self, id: RequestId, request: &SceneRequest) -> Verdict {
        match DclEngine::get() {
            Some(engine) => {
                engine.bind().report(EngineEvent::SceneRequest {
                    id,
                    request: request.clone(),
                });
                Verdict::Pending
            }
            None => Verdict::Done(Err("engine not available".to_string())),
        }
    }
}

/// The headless emulator has no user to ask: teleports and realm changes are taken, links and
/// NFTs are printed. The sink runs inside the scene manager's frame, which holds a mutable
/// borrow of it; navigation calls back into the scene manager, so the actions are deferred.
pub struct EmulatorSink;

impl SceneRequestSink for EmulatorSink {
    fn handle(&self, _id: RequestId, request: &SceneRequest) -> Verdict {
        match request {
            SceneRequest::OpenExternalUrl { url } => {
                tracing::info!("[emulator] scene asked to open {url}");
            }
            SceneRequest::OpenNft { urn } => {
                tracing::info!("[emulator] scene asked to show NFT {urn}");
            }
            SceneRequest::Teleport {
                parcel: Some(parcel),
                realm,
            } => {
                let (parcel, realm) = (
                    Vector2i::new(parcel.x, parcel.y),
                    realm.clone().unwrap_or_default(),
                );
                defer(move || DclGlobal::async_teleport_to(parcel, realm.as_str().into()));
            }
            SceneRequest::Teleport {
                parcel: None,
                realm: Some(realm),
            }
            | SceneRequest::ChangeRealm { realm, .. } => {
                let realm = realm.clone();
                defer(move || DclGlobal::async_join_world(realm.as_str().into()));
            }
            SceneRequest::Teleport { .. } => {}
        }
        Verdict::Done(Ok(()))
    }
}
