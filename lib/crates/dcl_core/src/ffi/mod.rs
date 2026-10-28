//! The typed surface the RN shell and the native hosts call (D-5), generated into TypeScript,
//! Kotlin and Swift from this module. Exported from `libdclgodot`, the one library the engine
//! also lives in, so there is one core per process (D-6). Keys never cross it.

use std::{sync::Arc, time::Duration};

use crate::{
    app::{AppEvents, State},
    Core,
};

mod avatar;
mod connection;
mod control;
mod ftue;
mod gates;
pub use gates::eula_accepted;
mod notifications;
mod places;
mod profile;
mod push;
mod review;
mod scene;
mod seasonal_skin;
mod session;
mod session_tracker;
mod settings;

#[derive(Debug, uniffi::Error)]
pub enum CoreError {
    NotInstalled,
    InvalidCall {
        detail: String,
    },
    /// A sign-in step failed; `code` is the stable value the shell maps to copy.
    SignIn {
        code: String,
        detail: String,
    },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstalled => write!(f, "the core is not installed"),
            Self::InvalidCall { detail } => write!(f, "{detail}"),
            Self::SignIn { code, detail } => write!(f, "{code}: {detail}"),
        }
    }
}

impl From<crate::hub::SignInError> for CoreError {
    fn from(error: crate::hub::SignInError) -> Self {
        Self::SignIn {
            code: error.code,
            detail: error.detail,
        }
    }
}

/// A page the shell shows in an in-app browser or webview.
#[derive(Clone, Debug, uniffi::Record)]
pub struct OpenUrl {
    pub url: String,
    pub use_webview: bool,
}

impl std::error::Error for CoreError {}

impl From<crate::app::AppError> for CoreError {
    fn from(error: crate::app::AppError) -> Self {
        Self::InvalidCall {
            detail: error.to_string(),
        }
    }
}

/// Implemented by the JS shell only: a foreign trait has one vtable per process, so the host's
/// events are a different trait (`dcl_host::HostListener`). Called from a core thread; the
/// generated bindings hop to the JS thread.
#[uniffi::export(with_foreign)]
pub trait AppListener: Send + Sync {
    fn state_changed(&self, state: State);
    fn scene_crashed(&self);
    /// A sign-in step failed; `code` is stable and the shell maps it to copy.
    fn auth_error(&self, code: String);
    fn open_url(&self, request: OpenUrl);
    fn settings_changed(&self, entry: crate::settings::SettingEntry);
    /// The player's whereabouts, a few times a second while in a world.
    fn hud(&self, snapshot: crate::app::HudSnapshot);
    fn chat(&self, line: crate::app::ChatLine);
    fn unread_chat(&self, count: u32);
    fn ui_state(&self, state: crate::app::UiState);
    fn open_backpack(&self, emotes: bool);
    fn loading_detail(&self, detail: crate::app::LoadingDetail);
    fn avatar_changed(&self, avatar: crate::app::AvatarState);
    /// A scene asked for the user's say (open a link, teleport, change realm, show an NFT).
    fn scene_request(&self, id: u32, request: crate::scene_requests::SceneRequest);
    /// Something the engine tells the player: a refused entry, a disconnect, a ban.
    fn engine_notice(&self, notice: crate::app::EngineNotice);
    /// A panic or error to send to the crash reporter, already past the keep/drop policy.
    fn report(&self, payload: crate::hub::ReportPayload);
    /// A tag for every later crash report; `None` removes it. Batched by the core.
    fn report_tag(&self, key: String, value: Option<String>);
    /// The remote feature flags were fetched, or failed and keep their defaults.
    fn flags_loaded(&self);
}

/// Hands the app's events to the shell from a thread of its own, in order. The app fires events
/// while it holds its lock, and the shell's JS thread can be waiting for that lock in a call of its
/// own; delivering on the firing thread would then wait for each other.
struct ListenerAdapter {
    queue: std::sync::mpsc::Sender<Delivery>,
    /// Only the latest `hud` snapshot is worth delivering: a slow shell would otherwise work
    /// through a backlog of stale ones (4 per second) before it saw a `state_changed`.
    hud: Arc<std::sync::Mutex<Option<crate::app::HudSnapshot>>>,
}

type Delivery = Box<dyn FnOnce(&dyn AppListener) + Send>;

fn flush_hud(slot: &std::sync::Mutex<Option<crate::app::HudSnapshot>>, listener: &dyn AppListener) {
    let snapshot = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(snapshot) = snapshot {
        listener.hud(snapshot);
    }
}

impl ListenerAdapter {
    fn new(listener: Arc<dyn AppListener>) -> Self {
        let (queue, deliveries) = std::sync::mpsc::channel::<Delivery>();
        let spawned = std::thread::Builder::new()
            .name("dcl-shell-events".into())
            .spawn(move || {
                for delivery in deliveries {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        delivery(&*listener)
                    }));
                }
            });
        if let Err(error) = spawned {
            tracing::error!("shell events: cannot start the delivery thread: {error}");
        }
        Self {
            queue,
            hud: Arc::default(),
        }
    }

    fn deliver(&self, delivery: impl FnOnce(&dyn AppListener) + Send + 'static) {
        let _ = self.queue.send(Box::new(delivery));
    }
}

impl AppEvents for ListenerAdapter {
    fn state_changed(&self, state: &State) {
        let state = state.clone();
        let hud = self.hud.clone();
        self.deliver(move |l| {
            flush_hud(&hud, l);
            l.state_changed(state)
        });
    }
    fn engine_required(&self) {}
    fn avatar_preview_mode(&self, _on: bool) {}
    fn auth_error(&self, code: &str) {
        let code = code.to_string();
        self.deliver(move |l| l.auth_error(code));
    }
    fn open_url(&self, request: &crate::OpenUrlRequest) {
        let request = OpenUrl {
            url: request.url.clone(),
            use_webview: request.use_webview,
        };
        self.deliver(move |l| l.open_url(request));
    }
    fn settings_changed(&self, entry: &crate::settings::SettingEntry) {
        let entry = entry.clone();
        self.deliver(move |l| l.settings_changed(entry));
    }
    fn hud(&self, snapshot: &crate::app::HudSnapshot) {
        let replaced = self
            .hud
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace(snapshot.clone());
        if replaced.is_none() {
            let hud = self.hud.clone();
            self.deliver(move |l| flush_hud(&hud, l));
        }
    }
    fn chat(&self, line: &crate::app::ChatLine) {
        let line = line.clone();
        self.deliver(move |l| l.chat(line));
    }
    fn unread_chat(&self, count: u32) {
        self.deliver(move |l| l.unread_chat(count));
    }
    fn ui_state(&self, state: &crate::app::UiState) {
        let state = *state;
        self.deliver(move |l| l.ui_state(state));
    }
    fn open_backpack(&self, emotes: bool) {
        self.deliver(move |l| l.open_backpack(emotes));
    }
    fn loading_detail(&self, detail: &crate::app::LoadingDetail) {
        let detail = *detail;
        self.deliver(move |l| l.loading_detail(detail));
    }
    fn avatar_changed(&self, avatar: &crate::app::AvatarState) {
        let avatar = avatar.clone();
        self.deliver(move |l| l.avatar_changed(avatar));
    }
    fn scene_request(
        &self,
        id: crate::scene_requests::RequestId,
        request: &crate::scene_requests::SceneRequest,
    ) {
        let request = request.clone();
        self.deliver(move |l| l.scene_request(id, request));
    }
    fn engine_notice(&self, notice: &crate::app::EngineNotice) {
        let notice = notice.clone();
        self.deliver(move |l| l.engine_notice(notice));
    }
    fn report(&self, payload: &crate::hub::ReportPayload) {
        let payload = payload.clone();
        self.deliver(move |l| l.report(payload));
    }
    fn report_tag(&self, key: &str, value: Option<&str>) {
        let (key, value) = (key.to_string(), value.map(str::to_string));
        self.deliver(move |l| l.report_tag(key, value));
    }
    fn scene_crashed(&self) {
        self.deliver(|l| l.scene_crashed());
    }
    fn flags_loaded(&self) {
        self.deliver(|l| l.flags_loaded());
    }
}

pub(crate) fn core() -> Result<&'static Core, CoreError> {
    Core::try_get().ok_or(CoreError::NotInstalled)
}

/// How many times the process asked to install a core; the engine asserts it is not a second one.
#[uniffi::export]
pub fn installs() -> u32 {
    Core::installs() as u32
}

#[uniffi::export]
pub fn state() -> Result<State, CoreError> {
    Ok(core()?.with_app(|app| app.state().clone()))
}

/// Registers the shell's receiver of the app's events. The id removes it again.
#[uniffi::export]
pub fn add_listener(listener: Arc<dyn AppListener>) -> Result<u64, CoreError> {
    Ok(core()?.add_app_listener(Arc::new(ListenerAdapter::new(listener))))
}

#[uniffi::export]
pub fn remove_listener(id: u64) -> Result<(), CoreError> {
    core()?.remove_app_listener(id);
    Ok(())
}

/// Resolves after `millis` on the core runtime: proves an async call returns a promise.
#[uniffi::export]
pub async fn debug_delay(millis: u32) -> Result<u32, CoreError> {
    let core = core()?;
    core.handle()
        .spawn(async move {
            tokio::time::sleep(Duration::from_millis(millis as u64)).await;
            millis
        })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })
}

/// Sends `count` `state_changed` events to every registered listener from a core thread,
/// `per_second` of them each second, and returns once the last was sent. Exercises the callback
/// path under load; the app's state does not change.
#[uniffi::export]
pub async fn debug_fire_events(count: u32, per_second: u32) -> Result<u32, CoreError> {
    let core = core()?;
    let interval = Duration::from_secs_f64(1.0 / per_second.max(1) as f64);
    core.handle()
        .spawn(async move {
            for i in 0..count {
                core.broadcast_state(&State::Loading {
                    destination: crate::app::Destination {
                        realm: None,
                        parcel: None,
                    },
                    progress: i as f32 / count.max(1) as f32,
                });
                tokio::time::sleep(interval).await;
            }
            count
        })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod adapter_tests {
    use std::sync::{mpsc, Mutex};
    use std::time::Duration;

    use super::*;

    /// Blocks every delivery until the test lets it through, like a shell whose JS thread is busy.
    struct Gate {
        open: Mutex<mpsc::Receiver<()>>,
        seen: mpsc::Sender<u32>,
        huds: mpsc::Sender<f32>,
    }

    impl AppListener for Gate {
        fn unread_chat(&self, count: u32) {
            let _ = self.open.lock().unwrap().recv();
            let _ = self.seen.send(count);
        }
        fn state_changed(&self, _: State) {}
        fn scene_crashed(&self) {}
        fn auth_error(&self, _: String) {}
        fn open_url(&self, _: OpenUrl) {}
        fn settings_changed(&self, _: crate::settings::SettingEntry) {}
        fn hud(&self, snapshot: crate::app::HudSnapshot) {
            let _ = self.huds.send(snapshot.fps);
        }
        fn chat(&self, _: crate::app::ChatLine) {}
        fn ui_state(&self, _: crate::app::UiState) {}
        fn open_backpack(&self, _: bool) {}
        fn loading_detail(&self, _: crate::app::LoadingDetail) {}
        fn avatar_changed(&self, _: crate::app::AvatarState) {}
        fn scene_request(&self, _: u32, _: crate::scene_requests::SceneRequest) {}
        fn engine_notice(&self, _: crate::app::EngineNotice) {}
        fn report(&self, _: crate::hub::ReportPayload) {}
        fn report_tag(&self, _: String, _: Option<String>) {}
        fn flags_loaded(&self) {}
    }

    #[test]
    fn a_busy_shell_does_not_hold_up_the_thread_that_fires_the_event() {
        let (release, open) = mpsc::channel();
        let (seen_tx, seen) = mpsc::channel();
        let adapter = ListenerAdapter::new(Arc::new(Gate {
            open: Mutex::new(open),
            seen: seen_tx,
            huds: mpsc::channel().0,
        }));

        adapter.unread_chat(1);
        adapter.unread_chat(2);
        assert!(
            seen.try_recv().is_err(),
            "the shell has not been let through yet"
        );

        release.send(()).unwrap();
        release.send(()).unwrap();
        let timeout = Duration::from_secs(5);
        assert_eq!(seen.recv_timeout(timeout), Ok(1));
        assert_eq!(seen.recv_timeout(timeout), Ok(2));
    }

    fn hud_at(fps: f32) -> crate::app::HudSnapshot {
        crate::app::HudSnapshot {
            parcel: crate::app::Parcel { x: 0, y: 0 },
            realm: String::new(),
            fps,
            position: crate::app::Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            scene_base: None,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            camera_mode: crate::app::CameraMode::ThirdPerson,
            camera_mode_locked: false,
            joystick_visible: true,
        }
    }

    #[test]
    fn a_busy_shell_gets_only_the_latest_hud_before_the_next_state() {
        let (release, open) = mpsc::channel();
        let (seen_tx, seen) = mpsc::channel();
        let (huds_tx, huds) = mpsc::channel();
        let adapter = ListenerAdapter::new(Arc::new(Gate {
            open: Mutex::new(open),
            seen: seen_tx,
            huds: huds_tx,
        }));

        adapter.unread_chat(0);
        for fps in [1.0, 2.0, 3.0] {
            adapter.hud(&hud_at(fps));
        }
        release.send(()).unwrap();
        let timeout = Duration::from_secs(5);
        assert_eq!(seen.recv_timeout(timeout), Ok(0));
        assert_eq!(huds.recv_timeout(timeout), Ok(3.0));
        assert!(huds.recv_timeout(Duration::from_millis(200)).is_err());
    }
}
