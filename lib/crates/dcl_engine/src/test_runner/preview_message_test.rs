use godot::prelude::*;
use prost::Message as _;

use crate::dcl::components::proto_components::sdk::development::{
    ws_scene_message::Message as WsMessage, UpdateModel, UpdateModelType, UpdateScene,
    WsSceneMessage,
};
use crate::framework::TestContext;
use crate::godot_classes::dcl_preview_message::DclPreviewMessage;

fn encode(message: WsMessage) -> PackedByteArray {
    let mut buf = Vec::new();
    WsSceneMessage {
        message: Some(message),
    }
    .encode(&mut buf)
    .expect("encode WsSceneMessage");
    PackedByteArray::from(buf.as_slice())
}

/// The preview server's whole-scene reload message — the protobuf replacement
/// for the legacy JSON `SCENE_UPDATE` frame.
#[godot::test::itest]
fn test_decode_update_scene(_ctx: &TestContext) {
    let dict = DclPreviewMessage::decode(encode(WsMessage::UpdateScene(UpdateScene {
        scene_id: "bafkreiscene".into(),
    })));

    assert_eq!(
        dict.get("type").unwrap().to::<GString>(),
        "update_scene".into()
    );
    assert_eq!(
        dict.get("scene_id").unwrap().to::<GString>(),
        "bafkreiscene".into()
    );
}

/// The per-file variant that drives model hot-swap.
#[godot::test::itest]
fn test_decode_update_model(_ctx: &TestContext) {
    let dict = DclPreviewMessage::decode(encode(WsMessage::UpdateModel(UpdateModel {
        scene_id: "bafkreiscene".into(),
        src: "models/chair.glb".into(),
        hash: "b64-bmV3".into(),
        r#type: UpdateModelType::UmtChange as i32,
    })));

    assert_eq!(
        dict.get("type").unwrap().to::<GString>(),
        "update_model".into()
    );
    assert_eq!(
        dict.get("src").unwrap().to::<GString>(),
        "models/chair.glb".into()
    );
    assert_eq!(dict.get("hash").unwrap().to::<GString>(), "b64-bmV3".into());
    assert!(!dict.get("removed").unwrap().to::<bool>());
}

/// A deleted file: the swap path turns this into a dropped mapping entry.
#[godot::test::itest]
fn test_decode_update_model_removed(_ctx: &TestContext) {
    let dict = DclPreviewMessage::decode(encode(WsMessage::UpdateModel(UpdateModel {
        scene_id: "bafkreiscene".into(),
        src: "models/chair.glb".into(),
        hash: "".into(),
        r#type: UpdateModelType::UmtRemove as i32,
    })));

    assert!(dict.get("removed").unwrap().to::<bool>());
}

/// Stray frames must not panic across the GDExtension boundary — the legacy
/// text protocol shares this socket, so garbage is expected in the wild.
#[godot::test::itest]
fn test_decode_rejects_garbage(_ctx: &TestContext) {
    // A JSON text frame's bytes, i.e. what an older CLI puts on the wire.
    let json = br#"{"type":"SCENE_UPDATE"}"#;
    assert!(DclPreviewMessage::decode(PackedByteArray::from(json.as_slice())).is_empty());

    // An empty oneof decodes cleanly as a message but carries no variant.
    let mut buf = Vec::new();
    WsSceneMessage { message: None }
        .encode(&mut buf)
        .expect("encode empty WsSceneMessage");
    assert!(DclPreviewMessage::decode(PackedByteArray::from(buf.as_slice())).is_empty());
}
