use prost::Message as _;

use crate::dcl::components::proto_components::sdk::development::{
    ws_scene_message::Message as WsMessage, UpdateModelType, WsSceneMessage,
};
use crate::utils::ws_client::{Frame, WsClient, WsState};

#[derive(Debug, PartialEq)]
pub enum PreviewEvent {
    /// The whole scene changed: legacy `SCENE_UPDATE` text frames and protobuf `update_scene`.
    SceneUpdate { scene_id: String },
    /// One .glb/.gltf changed. `hash` derives from the path, not the contents, so it is stable
    /// across edits and empty when `removed`.
    ModelUpdate {
        scene_id: String,
        src: String,
        hash: String,
        removed: bool,
    },
}

/// The preview server's file-change socket.
#[derive(Default)]
pub struct PreviewSocket {
    ws: WsClient,
    pending_url: String,
    // The server sends the protobuf message AND the legacy JSON one for every change; acting on
    // both would follow every precise model swap with a full scene reload, so once a server has
    // proven it speaks protobuf its legacy frames are dropped. (The server's shared 800ms
    // debounce keeps only the last of several models saved together: fixing that belongs in
    // sdk-commands' file-watch-notifier, not here.)
    speaks_protobuf: bool,
}

impl PreviewSocket {
    pub fn set_url(&mut self, url: &str) {
        self.pending_url = url
            .to_lowercase()
            .replace("http://", "ws://")
            .replace("https://", "wss://");
    }

    pub fn poll(&mut self) -> Vec<PreviewEvent> {
        let mut events = Vec::new();
        match self.ws.state() {
            WsState::Open => {
                // A new url re-points an open socket: close now, reconnect once it is closed.
                if !self.pending_url.is_empty() {
                    self.ws.close();
                    return events;
                }
                while let Some(frame) = self.ws.next_frame() {
                    match frame {
                        Frame::Text(text) => events.extend(self.on_text(&text)),
                        Frame::Binary(bytes) => events.extend(self.on_binary(&bytes)),
                    }
                }
            }
            WsState::Closed => {
                if !self.pending_url.is_empty() {
                    // Re-probed per connection: a reconnect may land on an older server.
                    self.speaks_protobuf = false;
                    let url = std::mem::take(&mut self.pending_url);
                    self.ws.connect(&url, "");
                    tracing::info!("preview-ws > connecting to {url}");
                }
            }
            WsState::Connecting => {}
        }
        events
    }

    /// The server also sends a bare "update" text frame next to the JSON one; it is not JSON
    /// and is ignored.
    fn on_text(&self, text: &str) -> Option<PreviewEvent> {
        if self.speaks_protobuf {
            return None;
        }
        let json: serde_json::Value = serde_json::from_str(text).ok()?;
        match json.get("type")?.as_str()? {
            "SCENE_UPDATE" => Some(PreviewEvent::SceneUpdate {
                scene_id: json["payload"]["sceneId"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
            }),
            other => {
                tracing::error!("preview-ws > unknown message type {other}");
                None
            }
        }
    }

    fn on_binary(&mut self, bytes: &[u8]) -> Option<PreviewEvent> {
        let Ok(message) = WsSceneMessage::decode(bytes) else {
            tracing::error!(
                "preview-ws > undecodable binary frame ({} bytes)",
                bytes.len()
            );
            return None;
        };
        let event = match message.message? {
            WsMessage::UpdateScene(update) => PreviewEvent::SceneUpdate {
                scene_id: update.scene_id,
            },
            WsMessage::UpdateModel(update) => PreviewEvent::ModelUpdate {
                removed: update.r#type == UpdateModelType::UmtRemove as i32,
                scene_id: update.scene_id,
                src: update.src,
                hash: update.hash,
            },
        };
        if !self.speaks_protobuf {
            tracing::info!("preview-ws > server speaks protobuf; ignoring its legacy JSON frames");
        }
        self.speaks_protobuf = true;
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_scene_update_is_read_until_protobuf_is_seen() {
        let mut socket = PreviewSocket::default();
        let frame = r#"{"type":"SCENE_UPDATE","payload":{"sceneId":"bafy"}}"#;
        assert_eq!(
            socket.on_text(frame),
            Some(PreviewEvent::SceneUpdate {
                scene_id: "bafy".into()
            })
        );
        assert_eq!(socket.on_text("update"), None);

        let bytes = WsSceneMessage {
            message: Some(WsMessage::UpdateScene(
                crate::dcl::components::proto_components::sdk::development::UpdateScene {
                    scene_id: "bafy".into(),
                },
            )),
        }
        .encode_to_vec();
        assert!(socket.on_binary(&bytes).is_some());
        assert_eq!(socket.on_text(frame), None);
        assert_eq!(socket.on_binary(&[0xff, 0xff]), None);
    }

    fn encode(message: WsMessage) -> Vec<u8> {
        WsSceneMessage {
            message: Some(message),
        }
        .encode_to_vec()
    }

    #[test]
    fn a_model_change_and_a_model_removal_are_told_apart() {
        use crate::dcl::components::proto_components::sdk::development::UpdateModel;
        let model = |hash: &str, kind: UpdateModelType| {
            encode(WsMessage::UpdateModel(UpdateModel {
                scene_id: "bafkreiscene".into(),
                src: "models/chair.glb".into(),
                hash: hash.into(),
                r#type: kind as i32,
            }))
        };
        let mut socket = PreviewSocket::default();
        assert_eq!(
            socket.on_binary(&model("b64-bmV3", UpdateModelType::UmtChange)),
            Some(PreviewEvent::ModelUpdate {
                scene_id: "bafkreiscene".into(),
                src: "models/chair.glb".into(),
                hash: "b64-bmV3".into(),
                removed: false,
            })
        );
        assert!(matches!(
            socket.on_binary(&model("", UpdateModelType::UmtRemove)),
            Some(PreviewEvent::ModelUpdate { removed: true, .. })
        ));
    }

    #[test]
    fn stray_frames_are_dropped_without_panicking() {
        let mut socket = PreviewSocket::default();
        // The legacy text protocol shares the socket, so a JSON frame's bytes show up here.
        assert_eq!(socket.on_binary(br#"{"type":"SCENE_UPDATE"}"#), None);
        // An empty oneof decodes but carries no variant.
        assert_eq!(
            socket.on_binary(&WsSceneMessage { message: None }.encode_to_vec()),
            None
        );
        assert!(!socket.speaks_protobuf);
    }

    #[test]
    fn set_url_swaps_the_scheme_for_a_websocket_one() {
        let mut socket = PreviewSocket::default();
        socket.set_url("HTTPS://Host:8000");
        assert_eq!(socket.pending_url, "wss://host:8000");
    }
}
