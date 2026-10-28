// Scene requests through the engine, run inside the Godot test runner.

use std::{cell::Cell, rc::Rc};

use dcl_core::scene_requests::{RequestId, SceneRequest};
use godot::prelude::*;
use tokio::sync::oneshot::{self, error::TryRecvError};

use super::{
    scene_requests::{EmulatorSink, SceneRequestSink, Verdict},
    DclEngine,
};
use crate::{dcl::scene_apis::RpcResultSender, framework::TestContext};

type Responder = RpcResultSender<Result<(), String>>;

fn responder() -> (Responder, oneshot::Receiver<Result<(), String>>) {
    let (sender, receiver) = oneshot::channel();
    (RpcResultSender::new(sender), receiver)
}

fn engine() -> Gd<DclEngine> {
    DclEngine::get().expect("the engine is created with Global")
}

fn url() -> SceneRequest {
    SceneRequest::OpenExternalUrl {
        url: "https://decentraland.org".to_string(),
    }
}

struct AnswersNow(Result<(), String>);

impl SceneRequestSink for AnswersNow {
    fn handle(&self, _id: RequestId, _request: &SceneRequest) -> Verdict {
        Verdict::Done(self.0.clone())
    }
}

/// Leaves the request pending and remembers its id.
struct AnswersLater(Rc<Cell<RequestId>>);

impl SceneRequestSink for AnswersLater {
    fn handle(&self, id: RequestId, _request: &SceneRequest) -> Verdict {
        self.0.set(id);
        Verdict::Pending
    }
}

fn restore_default_sink() {
    engine().bind_mut().set_sink(Rc::new(EmulatorSink));
}

#[godot::test::itest]
fn a_sink_that_answers_now_resolves_the_scene_rpc(_ctx: &TestContext) {
    engine().bind_mut().set_sink(Rc::new(AnswersNow(Ok(()))));
    let (response, mut receiver) = responder();

    DclEngine::scene_request(url(), response);

    assert_eq!(receiver.try_recv(), Ok(Ok(())));
    restore_default_sink();
}

#[godot::test::itest]
fn an_error_from_the_sink_reaches_the_scene(_ctx: &TestContext) {
    engine()
        .bind_mut()
        .set_sink(Rc::new(AnswersNow(Err("nope".to_string()))));
    let (response, mut receiver) = responder();

    DclEngine::scene_request(url(), response);

    assert_eq!(receiver.try_recv(), Ok(Err("nope".to_string())));
    restore_default_sink();
}

#[godot::test::itest]
fn a_pending_request_waits_for_the_answer_and_is_answered_once(_ctx: &TestContext) {
    let seen = Rc::new(Cell::new(0));
    engine()
        .bind_mut()
        .set_sink(Rc::new(AnswersLater(seen.clone())));
    let (response, mut receiver) = responder();

    DclEngine::scene_request(url(), response);
    assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));

    let id = seen.get();
    assert_ne!(id, 0);
    engine().bind_mut().answer_scene_request(id, true);
    assert_eq!(receiver.try_recv(), Ok(Ok(())));

    // A second answer for the same request does nothing.
    engine().bind_mut().answer_scene_request(id, false);
    restore_default_sink();
}

#[godot::test::itest]
fn declining_a_pending_request_is_an_error_for_the_scene(_ctx: &TestContext) {
    let seen = Rc::new(Cell::new(0));
    engine()
        .bind_mut()
        .set_sink(Rc::new(AnswersLater(seen.clone())));
    let (response, mut receiver) = responder();

    DclEngine::scene_request(url(), response);
    engine().bind_mut().answer_scene_request(seen.get(), false);

    assert!(matches!(receiver.try_recv(), Ok(Err(_))));
    restore_default_sink();
}

#[godot::test::itest]
fn the_emulator_sink_accepts_links_and_nfts_without_a_ui(_ctx: &TestContext) {
    restore_default_sink();
    let (response, mut receiver) = responder();
    DclEngine::scene_request(url(), response);
    assert_eq!(receiver.try_recv(), Ok(Ok(())));

    let (response, mut receiver) = responder();
    DclEngine::scene_request(
        SceneRequest::OpenNft {
            urn: "urn:decentraland:ethereum:erc721:0xabc:1".to_string(),
        },
        response,
    );
    assert_eq!(receiver.try_recv(), Ok(Ok(())));
}

#[godot::test::itest]
fn every_control_maps_to_a_kind_with_typed_args(_ctx: &TestContext) {
    use super::core_link::control_payload;
    use dcl_core::app::{CameraFocus, ColorTarget, Control, HudInsets, Parcel};

    let (kind, args) = control_payload(&Control::Teleport {
        parcel: Parcel::new(3, -4),
    })
    .unwrap();
    assert_eq!(kind, "teleport");
    assert_eq!(args.get("x").map(|v| v.to::<i32>()), Some(3));
    assert_eq!(args.get("y").map(|v| v.to::<i32>()), Some(-4));

    let (kind, args) = control_payload(&Control::SetHudInsets {
        insets: HudInsets {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        },
    })
    .unwrap();
    assert_eq!(kind, "set_hud_insets");
    assert_eq!(args.get("left").map(|v| v.to::<f32>()), Some(4.0));

    let (kind, args) = control_payload(&Control::FocusCamera {
        focus: CameraFocus::Hands,
    })
    .unwrap();
    assert_eq!(
        (kind, args.get("focus").map(|v| v.to::<String>())),
        ("focus_camera", Some("hands".to_string()))
    );

    let (kind, args) = control_payload(&Control::SetWorldSecret {
        realm: "https://worlds/x.dcl.eth".to_string(),
        secret: "hunter2".to_string(),
    })
    .unwrap();
    assert_eq!(kind, "set_world_secret");
    assert_eq!(
        args.get("secret").map(|v| v.to::<String>()),
        Some("hunter2".to_string())
    );

    // Avatar edits are computed by the core and reach the engine as `apply_avatar`.
    for edit in [
        Control::SetColor {
            target: ColorTarget::Eyes,
            color: "#112233".to_string(),
        },
        Control::SetEmoteSlot {
            slot: 7,
            urn: "wave".to_string(),
        },
        Control::Equip { urn: String::new() },
        Control::Unequip { urn: String::new() },
        Control::SetBodyShape { urn: String::new() },
        Control::RandomizeAvatar,
    ] {
        assert!(control_payload(&edit).is_none(), "{edit:?}");
    }

    let kinds: Vec<&str> = [
        Control::SendChat {
            text: String::new(),
        },
        Control::Emote { urn: String::new() },
        Control::SetChatWidth { width: 1.0 },
        Control::ChatOpen { open: true },
        Control::HideUi { hidden: true },
        Control::PlayPreviewEmote { urn: String::new() },
        Control::SetPreviewUrl { url: String::new() },
        Control::SetName {
            name: String::new(),
        },
    ]
    .iter()
    .map(|c| control_payload(c).unwrap().0)
    .collect();
    assert_eq!(
        kinds,
        [
            "send_chat",
            "emote",
            "set_chat_width",
            "chat_open",
            "hide_ui",
            "play_preview_emote",
            "set_preview_url",
            "set_name"
        ]
    );
}
