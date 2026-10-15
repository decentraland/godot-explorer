//! The app state machine (D-2): the one place that knows whether the user is signing in, in a menu,
//! previewing an avatar, loading or inside a world. The shell renders `State`; the engine executes
//! the `EngineCommand`s this module emits. No I/O here, so every transition is unit-testable.

mod control;
mod policy;
mod shared;
mod types;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use rand::{rngs::StdRng, SeedableRng};

use crate::avatar::{self, Catalog};

pub use control::{
    AvatarState, CameraFocus, ChatLine, ChatProfile, ColorTarget, Control, HudInsets, HudSnapshot,
    LoadingDetail, UiState, Vec3,
};
pub use policy::EngineStartPolicy;
pub use shared::{CommandQueue, HostEvents};
pub use types::{
    AppError, AppEvents, Destination, DisconnectReason, EngineCommand, EngineEvent, EngineNotice,
    EngineSink, EntryRefusal, Parcel, Rect, SignInMethod, SignInStep, State,
};

pub struct App {
    state: State,
    events: Arc<dyn AppEvents>,
    engine: Option<Arc<dyn EngineSink>>,
    /// Commands issued while no engine is attached; flushed in order on attach.
    queued: Vec<EngineCommand>,
    engine_requested: bool,
    policy: EngineStartPolicy,
    rendering_sent: Option<bool>,
    avatar: Option<AvatarState>,
    last_hud: Option<HudSnapshot>,
    /// The preview is shown over a running world (the in-world backpack), not as a state of its own.
    overlay: bool,
    catalog: Catalog,
    rng: StdRng,
}

impl App {
    pub fn new(events: Arc<dyn AppEvents>) -> Self {
        Self {
            state: State::Booting,
            events,
            engine: None,
            queued: Vec::new(),
            engine_requested: false,
            policy: EngineStartPolicy::default(),
            rendering_sent: None,
            avatar: None,
            last_hud: None,
            overlay: false,
            catalog: Catalog::default(),
            rng: StdRng::from_entropy(),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// The equipped avatar as the engine last reported it.
    pub fn avatar(&self) -> Option<&AvatarState> {
        self.avatar.as_ref()
    }

    /// The wearables the avatar edits resolve against; set once the shell loaded them.
    pub fn set_catalog(&mut self, catalog: Catalog) {
        self.catalog = catalog;
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    #[cfg(test)]
    fn seed_rng(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Runs a shell command: avatar edits are computed here and the result goes to the engine to
    /// render; the rest is forwarded, starting the engine if it is not up yet.
    pub fn control(&mut self, control: Control) {
        if let Some(edit) = self.edit_avatar(&control) {
            match edit {
                Ok(next) if self.avatar.as_ref() != Some(&next) => {
                    self.events.avatar_changed(&next);
                    self.avatar = Some(next.clone());
                    self.send(EngineCommand::ApplyAvatar(next));
                }
                Ok(_) => {}
                Err(error) => tracing::warn!("avatar edit ignored: {error}"),
            }
            return;
        }
        self.send(EngineCommand::Control(control));
    }

    /// `None` for a command that is not an avatar edit. An edit before the engine reported the
    /// avatar has nothing to start from and is ignored.
    fn edit_avatar(
        &mut self,
        control: &Control,
    ) -> Option<Result<AvatarState, avatar::AvatarError>> {
        let is_edit = matches!(
            control,
            Control::Equip { .. }
                | Control::Unequip { .. }
                | Control::SetBodyShape { .. }
                | Control::SetColor { .. }
                | Control::SetEmoteSlot { .. }
                | Control::RandomizeAvatar
        );
        if !is_edit {
            return None;
        }
        let Some(current) = self.avatar.clone() else {
            return Some(Err(avatar::AvatarError::NoAvatar));
        };
        Some(match control {
            Control::Equip { urn } => avatar::equip(&current, &self.catalog, urn),
            Control::Unequip { urn } => Ok(avatar::unequip(&current, &self.catalog, urn)),
            Control::SetBodyShape { urn } => avatar::set_body_shape(&current, &self.catalog, urn),
            Control::SetColor { target, color } => avatar::set_color(&current, *target, color),
            Control::SetEmoteSlot { slot, urn } => avatar::set_emote_slot(&current, *slot, urn),
            _ => avatar::randomize(&current, &self.catalog, &mut self.rng),
        })
    }

    pub fn engine_attached(&self) -> bool {
        self.engine.is_some()
    }

    /// `Booting` ends once the persisted session was looked up.
    pub fn finish_boot(&mut self, has_session: bool) -> Result<(), AppError> {
        self.expect(matches!(self.state, State::Booting), "finish_boot")?;
        self.set_state(if has_session {
            State::SignedIn
        } else {
            State::SignedOut
        });
        Ok(())
    }

    /// The host drew the first RN screen: the moment the D-42 policy may start the engine early.
    pub fn first_screen_drawn(&mut self) {
        if self.policy.starts_in_advance(&self.state) {
            self.request_engine();
        }
    }

    pub fn begin_sign_in(&mut self, method: SignInMethod) -> Result<(), AppError> {
        self.expect(
            matches!(self.state, State::SignedOut | State::SigningIn { .. }),
            "begin_sign_in",
        )?;
        let step = SignInStep::first_for(method);
        self.set_state(State::SigningIn { method, step });
        Ok(())
    }

    pub fn sign_in_step(&mut self, step: SignInStep) -> Result<(), AppError> {
        let State::SigningIn { method, .. } = self.state else {
            return Err(AppError::invalid("sign_in_step", &self.state));
        };
        self.set_state(State::SigningIn { method, step });
        Ok(())
    }

    pub fn sign_in_succeeded(&mut self) -> Result<(), AppError> {
        self.expect(
            matches!(self.state, State::SigningIn { .. }),
            "sign_in_succeeded",
        )?;
        self.set_state(State::SignedIn);
        Ok(())
    }

    /// A failed or cancelled attempt goes back to `SignedOut`; the cause travels as an event.
    pub fn sign_in_failed(&mut self) -> Result<(), AppError> {
        self.expect(
            matches!(self.state, State::SigningIn { .. }),
            "sign_in_failed",
        )?;
        self.set_state(State::SignedOut);
        Ok(())
    }

    /// Valid from any signed-in state; leaves the world and the preview first.
    pub fn sign_out(&mut self) -> Result<(), AppError> {
        self.expect(self.state.is_signed_in(), "sign_out")?;
        if self.state.in_world_flow() {
            self.send(EngineCommand::ExitWorld);
        }
        if self.state.is_preview() {
            self.send(EngineCommand::HideAvatarPreview);
        }
        self.set_state(State::SignedOut);
        Ok(())
    }

    /// Also valid while loading or in a world: it replaces the current destination.
    pub fn enter_world(&mut self, destination: Destination) -> Result<(), AppError> {
        self.expect(self.state.is_signed_in(), "enter_world")?;
        if self.state.is_preview() {
            self.send(EngineCommand::HideAvatarPreview);
        }
        self.send(EngineCommand::EnterWorld(destination.clone()));
        self.set_state(State::Loading {
            destination,
            progress: 0.0,
        });
        Ok(())
    }

    /// Keeps the session: back to the menus.
    pub fn exit_world(&mut self) -> Result<(), AppError> {
        self.expect(self.state.in_world_flow(), "exit_world")?;
        self.set_overlay(false);
        self.send(EngineCommand::ExitWorld);
        self.set_state(State::SignedIn);
        Ok(())
    }

    /// In a world the preview is an overlay and the state stays `InWorld`.
    pub fn show_avatar_preview(&mut self, rect: Rect) -> Result<(), AppError> {
        if matches!(self.state, State::InWorld { .. }) {
            self.send(EngineCommand::ShowAvatarPreview(rect));
            self.set_overlay(true);
            return Ok(());
        }
        self.expect(
            matches!(self.state, State::SignedIn | State::AvatarPreview { .. }),
            "show_avatar_preview",
        )?;
        self.send(EngineCommand::ShowAvatarPreview(rect));
        self.set_state(State::AvatarPreview { rect });
        Ok(())
    }

    pub fn hide_avatar_preview(&mut self) -> Result<(), AppError> {
        if self.overlay {
            self.send(EngineCommand::HideAvatarPreview);
            self.set_overlay(false);
            return Ok(());
        }
        self.expect(self.state.is_preview(), "hide_avatar_preview")?;
        self.send(EngineCommand::HideAvatarPreview);
        self.set_state(State::SignedIn);
        Ok(())
    }

    fn set_overlay(&mut self, on: bool) {
        if self.overlay != on {
            self.overlay = on;
            self.events.avatar_preview_mode(on);
        }
    }

    /// The engine started and registered itself: send what waited for it.
    pub fn engine_attach(&mut self, engine: Arc<dyn EngineSink>) {
        self.engine = Some(engine.clone());
        self.engine_requested = false;
        for command in std::mem::take(&mut self.queued) {
            engine.send(command);
        }
        self.rendering_sent = None;
        self.sync_rendering();
    }

    pub fn engine_detach(&mut self) {
        self.engine = None;
        self.engine_requested = false;
        self.rendering_sent = None;
    }

    /// Engine events drive `Loading` → `InWorld`; ones that do not fit the state are ignored,
    /// since the engine and the shell are not in lockstep.
    pub fn engine_event(&mut self, event: EngineEvent) {
        match (event, &self.state) {
            (EngineEvent::LoadingProgress(progress), State::Loading { destination, .. }) => {
                let next = State::Loading {
                    destination: destination.clone(),
                    progress: progress.clamp(0.0, 1.0),
                };
                self.set_state(next);
            }
            (EngineEvent::LoadingFinished { realm, parcel }, State::Loading { .. }) => {
                self.set_state(State::InWorld { realm, parcel });
            }
            (EngineEvent::SceneCrashed, State::InWorld { .. }) => self.events.scene_crashed(),
            (EngineEvent::Hud(snapshot), State::InWorld { .. }) => {
                self.events.hud(&snapshot);
                self.last_hud = Some(snapshot);
            }
            (EngineEvent::Chat(line), _) => self.events.chat(&line),
            (EngineEvent::UnreadChat(count), _) => self.events.unread_chat(count),
            (EngineEvent::UiState(ui), _) => self.events.ui_state(&ui),
            (EngineEvent::OpenBackpack { emotes }, _) => self.events.open_backpack(emotes),
            (EngineEvent::LoadingDetail(detail), State::Loading { .. }) => {
                self.events.loading_detail(&detail)
            }
            (EngineEvent::Avatar(avatar), _) => {
                self.events.avatar_changed(&avatar);
                self.avatar = Some(avatar);
            }
            (EngineEvent::Notice(notice), state) => {
                tracing::debug!("engine notice: {notice:?} in {state:?}");
                // A refused entry leaves the player where they were: the menus.
                if matches!(notice, EngineNotice::EntryRefused { .. })
                    && matches!(state, State::Loading { .. })
                {
                    self.set_state(State::SignedIn);
                }
                self.events.engine_notice(&notice);
            }
            (EngineEvent::SceneRequest { id, request }, _) => {
                self.events.scene_request(id, &request)
            }
            _ => {}
        }
    }

    fn expect(&self, ok: bool, call: &'static str) -> Result<(), AppError> {
        if ok {
            Ok(())
        } else {
            Err(AppError::invalid(call, &self.state))
        }
    }

    fn set_state(&mut self, next: State) {
        if next == self.state {
            return;
        }
        let was_preview = self.state.is_preview();
        self.state = next;
        if was_preview != self.state.is_preview() {
            self.events.avatar_preview_mode(self.state.is_preview());
        }
        self.events.state_changed(&self.state);
        self.sync_rendering();
    }

    /// Rendering is on only while a state shows the engine (D-42).
    fn sync_rendering(&mut self) {
        let wanted = self.state.shows_engine();
        if self.engine.is_some() && self.rendering_sent != Some(wanted) {
            self.rendering_sent = Some(wanted);
            self.send(EngineCommand::SetRendering(wanted));
        }
    }

    fn send(&mut self, command: EngineCommand) {
        match &self.engine {
            Some(engine) => engine.send(command),
            None => {
                self.queued.push(command);
                self.request_engine();
            }
        }
    }

    fn request_engine(&mut self) {
        if self.engine.is_none() && !self.engine_requested {
            self.engine_requested = true;
            self.events.engine_required();
        }
    }
}
