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
        parcel: Some((0, 0)),
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
        parcel: (3, -4),
    });
    assert_eq!(
        app.state(),
        &State::InWorld {
            realm: "main".into(),
            parcel: (3, -4)
        }
    );
}

#[test]
fn engine_events_that_do_not_fit_the_state_are_ignored() {
    let (mut app, events) = signed_in();
    app.engine_event(EngineEvent::LoadingProgress(0.3));
    app.engine_event(EngineEvent::LoadingFinished {
        realm: "main".into(),
        parcel: (0, 0),
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
        parcel: (0, 0),
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
        parcel: (0, 0),
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
