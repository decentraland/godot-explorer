use godot::prelude::*;
use prost::Message as _;

use crate::dcl::components::proto_components::sdk::development::{
    ws_scene_message::Message as WsMessage, UpdateModelType, WsSceneMessage,
};

/// Decoder for the binary `WsSceneMessage` frames the sdk-commands preview
/// server broadcasts on file changes.
///
/// The server also still sends the legacy JSON `SCENE_UPDATE` text frame, but
/// only until the explorers migrate (js-sdk-toolchain#1502); `PreviewWebSocket`
/// keeps that path as a fallback for older CLIs and routes binary frames here.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclPreviewMessage {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclPreviewMessage {
    /// Decodes a `WsSceneMessage` frame.
    ///
    /// Returns an empty dictionary when the payload isn't a valid message, so
    /// the caller can fall back or log without risking a panic on stray frames.
    #[func]
    pub fn decode(bytes: PackedByteArray) -> VarDictionary {
        let Ok(msg) = WsSceneMessage::decode(bytes.as_slice()) else {
            return VarDictionary::new();
        };

        let mut dict = VarDictionary::new();
        match msg.message {
            Some(WsMessage::UpdateScene(update)) => {
                dict.set("type", "update_scene");
                dict.set("scene_id", update.scene_id);
            }
            Some(WsMessage::UpdateModel(update)) => {
                dict.set("type", "update_model");
                dict.set("scene_id", update.scene_id);
                dict.set("src", update.src);
                dict.set("hash", update.hash);
                dict.set(
                    "removed",
                    update.r#type == UpdateModelType::UmtRemove as i32,
                );
            }
            // Empty oneof: either an older/newer server variant this build
            // doesn't know about, or a keepalive. Treated as undecodable.
            None => return VarDictionary::new(),
        }
        dict
    }
}
