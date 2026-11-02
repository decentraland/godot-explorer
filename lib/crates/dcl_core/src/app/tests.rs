use std::sync::{Arc, Mutex};

use super::*;

#[derive(Default)]
struct Recorder {
    states: Mutex<Vec<State>>,
    engine_required: Mutex<u32>,
    preview_modes: Mutex<Vec<bool>>,
    crashes: Mutex<u32>,
}

impl AppEvents for Recorder {
    fn state_changed(&self, state: &State) {
        self.states.lock().unwrap().push(state.clone());
    }
    fn engine_required(&self) {
        *self.engine_required.lock().unwrap() += 1;
    }
    fn avatar_preview_mode(&self, on: bool) {
        self.preview_modes.lock().unwrap().push(on);
    }
    fn scene_crashed(&self) {
        *self.crashes.lock().unwrap() += 1;
    }
}

#[derive(Default)]
struct FakeEngine {
    commands: Mutex<Vec<EngineCommand>>,
}

impl EngineSink for FakeEngine {
    fn send(&self, command: EngineCommand) {
        self.commands.lock().unwrap().push(command);
    }
}

impl FakeEngine {
    fn take(&self) -> Vec<EngineCommand> {
        std::mem::take(&mut *self.commands.lock().unwrap())
    }
}

const RECT: Rect = Rect {
    x: 10,
    y: 20,
    width: 300,
    height: 400,
};

fn plaza() -> Destination {
    Destination {
        realm: None,
        parcel: Some(Parcel::new(0, 0)),
    }
}

fn app() -> (App, Arc<Recorder>) {
    let events = Arc::new(Recorder::default());
    (App::new(events.clone()), events)
}

fn signed_in() -> (App, Arc<Recorder>) {
    let (mut app, events) = app();
    app.finish_boot(true).unwrap();
    (app, events)
}

fn with_engine(app: &mut App) -> Arc<FakeEngine> {
    let engine = Arc::new(FakeEngine::default());
    app.engine_attach(engine.clone());
    engine.take();
    engine
}

#[test]
fn boot_ends_signed_in_or_out_and_only_once() {
    let (mut app, _) = app();
    assert_eq!(app.state(), &State::Booting);
    app.finish_boot(false).unwrap();
    assert_eq!(app.state(), &State::SignedOut);
    assert!(app.finish_boot(true).is_err());

    let (app, _) = signed_in();
    assert_eq!(app.state(), &State::SignedIn);
}

#[test]
fn calls_before_boot_are_rejected() {
    let (mut app, _) = app();
    assert!(app.begin_sign_in(SignInMethod::Guest).is_err());
    assert!(app.enter_world(plaza()).is_err());
    assert!(app.sign_out().is_err());
    assert!(app.show_avatar_preview(RECT).is_err());
}

#[test]
fn sign_in_walks_its_steps_and_ends_signed_in() {
    let (mut app, _) = app();
    app.finish_boot(false).unwrap();
    app.begin_sign_in(SignInMethod::Email).unwrap();
    assert_eq!(
        app.state(),
        &State::SigningIn {
            method: SignInMethod::Email,
            step: SignInStep::Starting
        }
    );
    app.sign_in_step(SignInStep::AwaitingCode).unwrap();
    app.sign_in_succeeded().unwrap();
    assert_eq!(app.state(), &State::SignedIn);
}

#[test]
fn browser_and_wallet_start_at_their_waiting_step() {
    let (mut app, _) = app();
    app.finish_boot(false).unwrap();
    app.begin_sign_in(SignInMethod::Browser).unwrap();
    assert!(matches!(
        app.state(),
        State::SigningIn {
            step: SignInStep::AwaitingBrowser,
            ..
        }
    ));
    app.begin_sign_in(SignInMethod::Wallet).unwrap();
    assert!(matches!(
        app.state(),
        State::SigningIn {
            step: SignInStep::AwaitingWallet,
            ..
        }
    ));
}

#[test]
fn a_failed_sign_in_returns_to_signed_out() {
    let (mut app, _) = app();
    app.finish_boot(false).unwrap();
    app.begin_sign_in(SignInMethod::Guest).unwrap();
    app.sign_in_failed().unwrap();
    assert_eq!(app.state(), &State::SignedOut);
    assert!(app.sign_in_succeeded().is_err());
    assert!(app.sign_in_step(SignInStep::Starting).is_err());
}

#[test]
fn signing_in_while_signed_in_is_rejected() {
    let (mut app, _) = signed_in();
    assert!(app.begin_sign_in(SignInMethod::Guest).is_err());
}

#[test]
fn enter_world_without_an_engine_requests_it_once_and_queues() {
    let (mut app, events) = signed_in();
    app.enter_world(plaza()).unwrap();
    app.enter_world(plaza()).unwrap();
    assert_eq!(*events.engine_required.lock().unwrap(), 1);
    assert!(matches!(app.state(), State::Loading { progress, .. } if *progress == 0.0));

    let engine = Arc::new(FakeEngine::default());
    app.engine_attach(engine.clone());
    assert_eq!(
        engine.take(),
        vec![
            EngineCommand::EnterWorld(plaza()),
            EngineCommand::EnterWorld(plaza()),
            EngineCommand::SetRendering(true),
        ]
    );
}

#[test]
fn loading_events_drive_in_world() {
    let (mut app, _) = signed_in();
    let engine = with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    assert_eq!(
        engine.take(),
        vec![
            EngineCommand::EnterWorld(plaza()),
            EngineCommand::SetRendering(true)
        ]
    );

    app.engine_event(EngineEvent::LoadingProgress(0.5));
    assert!(matches!(app.state(), State::Loading { progress, .. } if *progress == 0.5));
    app.engine_event(EngineEvent::LoadingProgress(7.0));
    assert!(matches!(app.state(), State::Loading { progress, .. } if *progress == 1.0));

    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(3, -4),
    });
    assert_eq!(
        app.state(),
        &State::InWorld {
            realm: "main".into(),
            parcel: Parcel::new(3, -4)
        }
    );
}

#[test]
fn engine_events_that_do_not_fit_the_state_are_ignored() {
    let (mut app, events) = signed_in();
    app.engine_event(EngineEvent::LoadingProgress(0.3));
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(0, 0),
    });
    app.engine_event(EngineEvent::SceneCrashed);
    assert_eq!(app.state(), &State::SignedIn);
    assert_eq!(*events.crashes.lock().unwrap(), 0);
}

#[test]
fn a_scene_crash_stays_in_world_and_raises_an_event() {
    let (mut app, events) = signed_in();
    with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(0, 0),
    });
    app.engine_event(EngineEvent::SceneCrashed);
    assert!(matches!(app.state(), State::InWorld { .. }));
    assert_eq!(*events.crashes.lock().unwrap(), 1);
}

#[test]
fn exit_world_keeps_the_session_and_can_enter_again() {
    let (mut app, _) = signed_in();
    let engine = with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(0, 0),
    });
    engine.take();

    app.exit_world().unwrap();
    assert_eq!(app.state(), &State::SignedIn);
    assert_eq!(
        engine.take(),
        vec![EngineCommand::ExitWorld, EngineCommand::SetRendering(false)]
    );
    assert!(app.exit_world().is_err());

    app.enter_world(plaza()).unwrap();
    assert!(matches!(app.state(), State::Loading { .. }));
}

#[test]
fn entering_another_world_replaces_the_destination() {
    let (mut app, _) = signed_in();
    with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    let other = Destination {
        realm: Some("https://world.example".into()),
        parcel: None,
    };
    app.enter_world(other.clone()).unwrap();
    assert!(matches!(app.state(), State::Loading { destination, .. } if *destination == other));
}

#[test]
fn avatar_preview_flips_the_surface_order_and_rendering() {
    let (mut app, events) = signed_in();
    let engine = with_engine(&mut app);

    app.show_avatar_preview(RECT).unwrap();
    assert_eq!(app.state(), &State::AvatarPreview { rect: RECT });
    assert_eq!(
        engine.take(),
        vec![
            EngineCommand::ShowAvatarPreview(RECT),
            EngineCommand::SetRendering(true)
        ]
    );
    assert_eq!(*events.preview_modes.lock().unwrap(), vec![true]);

    let moved = Rect { x: 0, ..RECT };
    app.show_avatar_preview(moved).unwrap();
    assert_eq!(*events.preview_modes.lock().unwrap(), vec![true]);

    app.hide_avatar_preview().unwrap();
    assert_eq!(app.state(), &State::SignedIn);
    assert_eq!(*events.preview_modes.lock().unwrap(), vec![true, false]);
    assert_eq!(
        engine.take().last(),
        Some(&EngineCommand::SetRendering(false))
    );
    assert!(app.hide_avatar_preview().is_err());
}

#[test]
fn entering_a_world_from_the_preview_hides_it_first() {
    let (mut app, events) = signed_in();
    let engine = with_engine(&mut app);
    app.show_avatar_preview(RECT).unwrap();
    engine.take();

    app.enter_world(plaza()).unwrap();
    assert_eq!(
        engine.take(),
        vec![
            EngineCommand::HideAvatarPreview,
            EngineCommand::EnterWorld(plaza())
        ]
    );
    assert_eq!(*events.preview_modes.lock().unwrap(), vec![true, false]);
}

#[test]
fn sign_out_leaves_the_world_and_the_preview() {
    let (mut app, _) = signed_in();
    let engine = with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    engine.take();
    app.sign_out().unwrap();
    assert_eq!(app.state(), &State::SignedOut);
    assert_eq!(engine.take()[0], EngineCommand::ExitWorld);

    let (mut app, _) = signed_in();
    let engine = with_engine(&mut app);
    app.show_avatar_preview(RECT).unwrap();
    engine.take();
    app.sign_out().unwrap();
    assert_eq!(engine.take()[0], EngineCommand::HideAvatarPreview);
}

#[test]
fn sign_out_is_rejected_when_signed_out() {
    let (mut app, _) = app();
    app.finish_boot(false).unwrap();
    assert!(app.sign_out().is_err());
}

#[test]
fn rendering_is_off_until_a_state_shows_the_engine() {
    let (mut app, _) = signed_in();
    let engine = Arc::new(FakeEngine::default());
    app.engine_attach(engine.clone());
    assert_eq!(engine.take(), vec![EngineCommand::SetRendering(false)]);
}

#[test]
fn detaching_the_engine_requests_it_again_on_the_next_need() {
    let (mut app, events) = signed_in();
    with_engine(&mut app);
    app.engine_detach();
    assert!(!app.engine_attached());
    app.show_avatar_preview(RECT).unwrap();
    assert_eq!(*events.engine_required.lock().unwrap(), 1);
}

#[test]
fn advance_policy_requests_the_engine_after_the_first_screen() {
    for has_session in [true, false] {
        let (mut app, events) = app();
        app.first_screen_drawn();
        assert_eq!(*events.engine_required.lock().unwrap(), 0, "still booting");

        app.finish_boot(has_session).unwrap();
        app.first_screen_drawn();
        app.first_screen_drawn();
        assert_eq!(*events.engine_required.lock().unwrap(), 1);
    }
}

#[test]
fn on_demand_policy_waits_for_the_first_call_that_needs_the_engine() {
    let (mut app, events) = app();
    app.policy = EngineStartPolicy::OnDemand;
    app.finish_boot(true).unwrap();
    app.first_screen_drawn();
    assert_eq!(*events.engine_required.lock().unwrap(), 0);
    app.show_avatar_preview(RECT).unwrap();
    assert_eq!(*events.engine_required.lock().unwrap(), 1);
}

#[test]
fn attaching_clears_the_pending_request() {
    let (mut app, events) = signed_in();
    app.first_screen_drawn();
    with_engine(&mut app);
    app.show_avatar_preview(RECT).unwrap();
    assert_eq!(*events.engine_required.lock().unwrap(), 1);
}

#[test]
fn every_state_change_is_reported_once() {
    let (mut app, events) = app();
    app.finish_boot(false).unwrap();
    app.begin_sign_in(SignInMethod::Guest).unwrap();
    app.sign_in_succeeded().unwrap();
    assert_eq!(
        *events.states.lock().unwrap(),
        vec![
            State::SignedOut,
            State::SigningIn {
                method: SignInMethod::Guest,
                step: SignInStep::Starting
            },
            State::SignedIn
        ]
    );
}

#[derive(Default)]
struct Facts {
    seen: Mutex<Vec<String>>,
}

impl AppEvents for Facts {
    fn state_changed(&self, _: &State) {}
    fn engine_required(&self) {}
    fn avatar_preview_mode(&self, _: bool) {}
    fn scene_crashed(&self) {}
    fn hud(&self, snapshot: &HudSnapshot) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("hud {}", snapshot.realm));
    }
    fn chat(&self, line: &ChatLine) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("chat {}", line.message));
    }
    fn unread_chat(&self, count: u32) {
        self.seen.lock().unwrap().push(format!("unread {count}"));
    }
    fn ui_state(&self, state: &UiState) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("ui {}", state.hidden));
    }
    fn open_backpack(&self, emotes: bool) {
        self.seen.lock().unwrap().push(format!("backpack {emotes}"));
    }
    fn loading_detail(&self, detail: &LoadingDetail) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("loading {}", detail.percent));
    }
    fn avatar_changed(&self, avatar: &AvatarState) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("avatar {}", avatar.body_shape));
    }
    fn scene_request(&self, id: u32, request: &crate::scene_requests::SceneRequest) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("request {id} {request:?}"));
    }
}

fn in_a_world(facts: &Arc<Facts>) -> (App, Arc<FakeEngine>) {
    let mut app = App::new(facts.clone());
    app.finish_boot(true).unwrap();
    let engine = with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(0, 0),
    });
    engine.take();
    (app, engine)
}

#[test]
fn shell_commands_reach_the_engine_and_wait_for_it_when_it_is_not_up() {
    let (mut app, events) = signed_in();
    app.control(Control::SendChat { text: "hi".into() });
    assert_eq!(*events.engine_required.lock().unwrap(), 1);

    let engine = Arc::new(FakeEngine::default());
    app.engine_attach(engine.clone());
    assert_eq!(
        engine.take()[0],
        EngineCommand::Control(Control::SendChat { text: "hi".into() })
    );
    app.control(Control::Teleport {
        parcel: Parcel::new(3, 4),
    });
    assert_eq!(
        engine.take(),
        vec![EngineCommand::Control(Control::Teleport {
            parcel: Parcel::new(3, 4)
        })]
    );
}

fn snapshot(parcel: Parcel, realm: &str) -> HudSnapshot {
    HudSnapshot {
        parcel,
        realm: realm.into(),
        fps: 30.0,
        position: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        scene_base: Some(parcel),
        camera_yaw: 0.0,
        camera_pitch: 0.0,
        camera_mode: CameraMode::ThirdPerson,
        camera_mode_locked: false,
        joystick_visible: true,
    }
}

#[test]
fn engine_facts_reach_the_listeners() {
    let facts = Arc::new(Facts::default());
    let (mut app, _engine) = in_a_world(&facts);
    app.engine_event(EngineEvent::Hud(snapshot(Parcel::new(1, 2), "main")));
    app.engine_event(EngineEvent::Chat(ChatLine {
        address: "0x1".into(),
        message: "hello".into(),
        timestamp: 1.0,
        profile: None,
    }));
    app.engine_event(EngineEvent::UnreadChat(3));
    app.engine_event(EngineEvent::UiState(UiState {
        hidden: true,
        chat_open: false,
    }));
    app.engine_event(EngineEvent::OpenBackpack { emotes: true });
    app.engine_event(EngineEvent::Avatar(AvatarState {
        body_shape: "female".into(),
        wearables: vec![],
        emotes: vec![],
        skin: "#ffffff".into(),
        hair: "#000000".into(),
        eyes: "#000000".into(),
    }));
    assert_eq!(
        *facts.seen.lock().unwrap(),
        vec![
            "hud main",
            "chat hello",
            "unread 3",
            "ui true",
            "backpack true",
            "avatar female"
        ]
    );
    assert_eq!(app.avatar().unwrap().body_shape, "female");
}

#[test]
fn hud_and_loading_detail_are_ignored_outside_their_states() {
    let facts = Arc::new(Facts::default());
    let mut app = App::new(facts.clone());
    app.finish_boot(true).unwrap();
    app.engine_event(EngineEvent::Hud(snapshot(Parcel::new(0, 0), "x")));
    app.engine_event(EngineEvent::LoadingDetail(LoadingDetail {
        loaded: 1,
        total: 2,
        mbps: 1.0,
        percent: 50.0,
    }));
    assert!(facts.seen.lock().unwrap().is_empty());

    with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    app.engine_event(EngineEvent::LoadingDetail(LoadingDetail {
        loaded: 1,
        total: 2,
        mbps: 1.0,
        percent: 50.0,
    }));
    assert_eq!(*facts.seen.lock().unwrap(), vec!["loading 50"]);
}

#[test]
fn the_preview_over_a_running_world_is_an_overlay_that_keeps_the_state() {
    let facts = Arc::new(Facts::default());
    let (mut app, engine) = in_a_world(&facts);
    app.show_avatar_preview(RECT).unwrap();
    assert!(matches!(app.state(), State::InWorld { .. }));
    assert_eq!(engine.take(), vec![EngineCommand::ShowAvatarPreview(RECT)]);

    app.hide_avatar_preview().unwrap();
    assert!(matches!(app.state(), State::InWorld { .. }));
    assert_eq!(engine.take(), vec![EngineCommand::HideAvatarPreview]);
    assert!(app.hide_avatar_preview().is_err(), "nothing left to hide");
}

#[test]
fn leaving_the_world_closes_the_overlay() {
    let recorder = Arc::new(Recorder::default());
    let mut app = App::new(recorder.clone());
    app.finish_boot(true).unwrap();
    with_engine(&mut app);
    app.enter_world(plaza()).unwrap();
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(0, 0),
    });
    app.show_avatar_preview(RECT).unwrap();
    app.exit_world().unwrap();
    assert_eq!(*recorder.preview_modes.lock().unwrap(), vec![true, false]);
}

#[test]
fn a_scene_request_reaches_the_shell_and_its_answer_reaches_the_engine() {
    use crate::scene_requests::SceneRequest;
    let facts = Arc::new(Facts::default());
    let (mut app, engine) = in_a_world(&facts);
    app.engine_event(EngineEvent::SceneRequest {
        id: 7,
        request: SceneRequest::OpenExternalUrl {
            url: "https://decentraland.org".into(),
        },
    });
    assert_eq!(
        *facts.seen.lock().unwrap(),
        vec!["request 7 OpenExternalUrl { url: \"https://decentraland.org\" }"]
    );

    app.control(Control::AnswerSceneRequest {
        id: 7,
        accepted: true,
    });
    assert_eq!(
        engine.take(),
        vec![EngineCommand::Control(Control::AnswerSceneRequest {
            id: 7,
            accepted: true
        })]
    );
}

fn wearable(urn: &str, category: &str) -> crate::avatar::CatalogItem {
    crate::avatar::CatalogItem {
        urn: urn.into(),
        name: urn.into(),
        category: category.into(),
        thumbnail: "t".into(),
        rarity: "base".into(),
        body_shapes: vec![],
        collectible: false,
    }
}

fn reported(app: &mut App) {
    app.set_catalog(crate::avatar::Catalog::new(vec![
        wearable("shirt_a", "upper_body"),
        wearable("shirt_b", "upper_body"),
    ]));
    app.engine_event(EngineEvent::Avatar(AvatarState {
        body_shape: crate::avatar::MALE.into(),
        wearables: vec!["shirt_a".into()],
        emotes: vec![String::new(); 10],
        skin: "#ffffff".into(),
        hair: "#000000".into(),
        eyes: "#000000".into(),
    }));
}

#[test]
fn an_avatar_edit_is_computed_here_and_the_engine_only_renders_it() {
    let facts = Arc::new(Facts::default());
    let (mut app, engine) = in_a_world(&facts);
    reported(&mut app);
    engine.take();
    facts.seen.lock().unwrap().clear();

    app.control(Control::Equip {
        urn: "shirt_b".into(),
    });

    let sent = engine.take();
    let [EngineCommand::ApplyAvatar(avatar)] = sent.as_slice() else {
        panic!("expected one ApplyAvatar, got {sent:?}");
    };
    assert_eq!(avatar.wearables, vec!["shirt_b".to_string()]);
    assert_eq!(app.avatar(), Some(avatar));
    assert_eq!(
        *facts.seen.lock().unwrap(),
        vec![format!("avatar {}", crate::avatar::MALE)]
    );
}

#[test]
fn an_edit_that_changes_nothing_sends_nothing() {
    let facts = Arc::new(Facts::default());
    let (mut app, engine) = in_a_world(&facts);
    reported(&mut app);
    engine.take();
    app.control(Control::Equip {
        urn: "shirt_a".into(),
    });
    app.control(Control::Unequip {
        urn: "not worn".into(),
    });
    assert_eq!(engine.take(), vec![]);
}

#[test]
fn a_rejected_edit_or_one_before_the_avatar_is_known_is_ignored() {
    let facts = Arc::new(Facts::default());
    let (mut app, engine) = in_a_world(&facts);
    engine.take();
    app.control(Control::RandomizeAvatar);
    assert_eq!(engine.take(), vec![], "no avatar reported yet");

    reported(&mut app);
    engine.take();
    app.control(Control::Equip {
        urn: "not in the catalog".into(),
    });
    app.control(Control::SetColor {
        target: ColorTarget::Hair,
        color: "red".into(),
    });
    app.control(Control::SetEmoteSlot {
        slot: 99,
        urn: "wave".into(),
    });
    assert_eq!(engine.take(), vec![]);
}

#[test]
fn other_controls_still_go_to_the_engine() {
    let facts = Arc::new(Facts::default());
    let (mut app, engine) = in_a_world(&facts);
    engine.take();
    app.control(Control::SendChat { text: "hi".into() });
    assert_eq!(
        engine.take(),
        vec![EngineCommand::Control(Control::SendChat {
            text: "hi".into()
        })]
    );
}

#[test]
fn randomize_draws_from_the_catalog_and_is_repeatable_for_a_seed() {
    let facts = Arc::new(Facts::default());
    let (mut a, engine_a) = in_a_world(&facts);
    reported(&mut a);
    a.seed_rng(3);
    engine_a.take();
    a.control(Control::RandomizeAvatar);
    let first = engine_a.take();

    let (mut b, engine_b) = in_a_world(&facts);
    reported(&mut b);
    b.seed_rng(3);
    engine_b.take();
    b.control(Control::RandomizeAvatar);
    assert_eq!(first, engine_b.take());
    assert!(matches!(first.as_slice(), [EngineCommand::ApplyAvatar(_)]));
}

#[derive(Default)]
struct Notices {
    seen: Mutex<Vec<EngineNotice>>,
}

impl AppEvents for Notices {
    fn state_changed(&self, _: &State) {}
    fn engine_required(&self) {}
    fn avatar_preview_mode(&self, _: bool) {}
    fn scene_crashed(&self) {}
    fn engine_notice(&self, notice: &EngineNotice) {
        self.seen.lock().unwrap().push(notice.clone());
    }
}

#[test]
fn a_refused_entry_sends_the_player_back_to_the_menus_and_is_announced() {
    let notices = Arc::new(Notices::default());
    let mut app = App::new(notices.clone());
    app.finish_boot(true).unwrap();
    app.enter_world(plaza()).unwrap();
    assert!(matches!(app.state(), State::Loading { .. }));

    let refusal = EngineNotice::EntryRefused {
        reason: EntryRefusal::PrivateWorld,
        realm: "https://worlds/x.dcl.eth".into(),
    };
    app.engine_event(EngineEvent::Notice(refusal.clone()));

    assert_eq!(*app.state(), State::SignedIn);
    assert_eq!(*notices.seen.lock().unwrap(), vec![refusal]);
}

#[test]
fn other_notices_do_not_move_the_state() {
    let notices = Arc::new(Notices::default());
    let mut app = App::new(notices.clone());
    app.finish_boot(true).unwrap();
    app.enter_world(plaza()).unwrap();
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: Parcel::new(0, 0),
    });
    for notice in [
        EngineNotice::Banned,
        EngineNotice::LowMemory,
        EngineNotice::Disconnected {
            reason: DisconnectReason::RoomClosed,
        },
        EngineNotice::Disconnected {
            reason: DisconnectReason::PlatformBan,
        },
        EngineNotice::DisconnectRecovered,
        EngineNotice::EntryRefused {
            reason: EntryRefusal::NotFound,
            realm: "r".into(),
        },
    ] {
        app.engine_event(EngineEvent::Notice(notice));
    }
    assert!(matches!(app.state(), State::InWorld { .. }));
    assert_eq!(notices.seen.lock().unwrap().len(), 6);
}
