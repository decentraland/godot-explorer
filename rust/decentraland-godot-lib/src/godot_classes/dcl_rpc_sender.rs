use godot::engine::RefCounted;
use godot::prelude::*;

use crate::dcl::scene_apis::RpcResultSender;
use crate::dcl::TakeAndCompareSnapshotResponse;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclRpcSender {
    sender: Option<RpcResultSender<Result<TakeAndCompareSnapshotResponse, String>>>,

    #[base]
    _base: Base<RefCounted>,
}

impl godot::builtin::meta::GodotConvert for TakeAndCompareSnapshotResponse {
    type Via = Dictionary;
}

impl FromGodot for TakeAndCompareSnapshotResponse {
    fn try_from_godot(via: Dictionary) -> Option<Self> {
        let is_match = via.get("is_match")?.to::<bool>();
        let similarity = via.get("similarity")?.to::<f32>();
        let was_exist = via.get("was_exist")?.to::<bool>();
        let replaced = via.get("replaced")?.to::<bool>();
        Some(Self {
            is_match,
            similarity,
            was_exist,
            replaced,
        })
    }
}

impl DclRpcSender {
    pub fn set_sender(
        &mut self,
        sender: RpcResultSender<Result<TakeAndCompareSnapshotResponse, String>>,
    ) {
        self.sender = Some(sender);
    }
}

#[godot_api]
impl DclRpcSender {
    #[func]
    fn send(&mut self, response: Variant) {
        if let Some(sender) = self.sender.as_ref() {
            let response = response.try_to::<Dictionary>().unwrap();
            let response = TakeAndCompareSnapshotResponse::from_godot(response);
            let sender: tokio::sync::oneshot::Sender<
                Result<TakeAndCompareSnapshotResponse, String>,
            > = sender.take();
            let ret = sender.send(Ok(response));

            match ret {
                Ok(_) => {
                    tracing::info!("Response sent");
                }
                Err(e) => {
                    tracing::info!("Error sending response {:?}", e);
                }
            }
        }
    }
}
