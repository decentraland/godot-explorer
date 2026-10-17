//! Scene-requested actions on the engine side: who is asked, and how the scene's RPC is answered.

use std::rc::Rc;

use dcl_core::scene_requests::{PendingRequests, RequestId, SceneRequest};
use godot::prelude::*;

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

/// The app side of a scene request: the Godot UI today, the shell when RN renders the modals.
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

/// The Godot modals, as before: the RPC answers `Ok` as soon as the modal is shown.
pub struct GodotUiSink;

impl GodotUiSink {
    fn modal_manager() -> Result<Gd<Node>, String> {
        DclGlobal::try_singleton()
            .and_then(|global| global.get("modal_manager").try_to::<Gd<Node>>().ok())
            .ok_or_else(|| "modal_manager not available".to_string())
    }
}

impl SceneRequestSink for GodotUiSink {
    fn handle(&self, _id: RequestId, request: &SceneRequest) -> Verdict {
        let result = match request {
            SceneRequest::OpenNft { urn } => open_nft_dialog(urn),
            _ => Self::modal_manager().map(|mut modal_manager| match request {
                SceneRequest::ChangeRealm { realm, message } => {
                    modal_manager.call(
                        "async_show_change_realm_modal",
                        &[
                            realm.to_variant(),
                            message.clone().unwrap_or_default().to_variant(),
                        ],
                    );
                }
                SceneRequest::OpenExternalUrl { url } => {
                    modal_manager.call("async_show_external_link_modal", &[url.to_variant()]);
                }
                SceneRequest::Teleport {
                    parcel: Some((x, y)),
                    realm,
                } => {
                    modal_manager.call(
                        "async_show_teleport_modal",
                        &[
                            Vector2i::new(*x, *y).to_variant(),
                            realm.clone().unwrap_or_default().to_variant(),
                        ],
                    );
                }
                SceneRequest::Teleport {
                    parcel: None,
                    realm: Some(realm),
                } => {
                    modal_manager.call("async_show_realm_teleport_modal", &[realm.to_variant()]);
                }
                _ => {}
            }),
        };
        Verdict::Done(result)
    }
}

fn open_nft_dialog(urn: &str) -> Result<(), String> {
    let mut dialog_stack = DclEngine::get()
        .and_then(|engine| engine.bind().dialog_stack())
        .ok_or_else(|| "DialogStack not available".to_string())?;
    let mut dialog = godot::tools::load::<PackedScene>(
        "res://src/ui/components/organisms/dialogs/nft_dialog.tscn",
    )
    .instantiate()
    .ok_or_else(|| "NftDialog instantiate error".to_string())?;
    dialog_stack.add_child(&dialog.clone());
    dialog.call("async_load_nft", &[urn.to_variant()]);
    Ok(())
}

/// The headless emulator has no user to ask: teleports and realm changes are taken, links and
/// NFTs are printed. The sink runs inside the scene manager's frame, which holds a mutable
/// borrow of it; navigation calls back into the scene manager, so the actions are deferred.
pub struct EmulatorSink;

impl SceneRequestSink for EmulatorSink {
    fn handle(&self, _id: RequestId, request: &SceneRequest) -> Verdict {
        let mut global = DclGlobal::singleton();
        match request {
            SceneRequest::OpenExternalUrl { url } => {
                tracing::info!("[emulator] scene asked to open {url}");
            }
            SceneRequest::OpenNft { urn } => {
                tracing::info!("[emulator] scene asked to show NFT {urn}");
            }
            SceneRequest::Teleport {
                parcel: Some((x, y)),
                realm,
            } => {
                global.call_deferred(
                    "async_teleport_to",
                    &[
                        Vector2i::new(*x, *y).to_variant(),
                        realm.clone().unwrap_or_default().to_variant(),
                    ],
                );
            }
            SceneRequest::Teleport {
                parcel: None,
                realm: Some(realm),
            }
            | SceneRequest::ChangeRealm { realm, .. } => {
                global.call_deferred("async_join_world", &[realm.to_variant()]);
            }
            SceneRequest::Teleport { .. } => {}
        }
        Verdict::Done(Ok(()))
    }
}
