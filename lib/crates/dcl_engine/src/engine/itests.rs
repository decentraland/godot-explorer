// Scene requests through the engine, run inside the Godot test runner.

use std::{cell::Cell, rc::Rc};

use dcl_core::scene_requests::{RequestId, SceneRequest};
use godot::prelude::*;
use tokio::sync::oneshot::{self, error::TryRecvError};

use super::{
    scene_requests::{GodotUiSink, SceneRequestSink, Verdict},
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
    engine().bind_mut().set_sink(Rc::new(GodotUiSink));
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
fn the_godot_ui_sink_reports_a_missing_modal_manager(_ctx: &TestContext) {
    // The test runner boots without the app services, so there is no modal manager.
    restore_default_sink();
    let (response, mut receiver) = responder();

    DclEngine::scene_request(url(), response);

    assert_eq!(
        receiver.try_recv(),
        Ok(Err("modal_manager not available".to_string()))
    );
}

#[godot::test::itest]
fn the_emulator_sink_accepts_links_and_nfts_without_a_ui(_ctx: &TestContext) {
    engine().bind_mut().set_emulator_mode(true);
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
    engine().bind_mut().set_emulator_mode(false);
}
