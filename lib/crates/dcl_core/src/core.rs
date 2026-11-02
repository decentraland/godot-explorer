use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};

use crate::app::{App, AppEvents};
use crate::hub::{Hub, LiveBackend};
use crate::session::identity::Session;
use tokio::runtime::{Builder, Handle, Runtime};

static CORE: OnceLock<Core> = OnceLock::new();
static INSTALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Platform {
    Android,
    Ios,
    Desktop,
}

impl Platform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "android" => Self::Android,
            "ios" => Self::Ios,
            _ => Self::Desktop,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreConfig {
    pub data_dir: PathBuf,
    pub platform: Platform,
}

/// Process-wide root of the core: configuration plus the tokio runtime every async task runs on.
pub struct Core {
    config: CoreConfig,
    runtime: Runtime,
    hub: Hub,
}

impl Core {
    /// Installs the core once, with the process logger and panic hook. A second call returns the
    /// first core; in debug builds it must pass the same config, since a different one means two
    /// parts of the process disagree about who owns the core.
    pub fn install(config: CoreConfig) -> &'static Core {
        INSTALLS.fetch_add(1, Ordering::Relaxed);
        let mut first = false;
        let core = CORE.get_or_init(|| {
            first = true;
            crate::logging::init(config.platform);
            let runtime = Builder::new_multi_thread()
                .enable_all()
                .thread_name("dcl-core-tokio")
                .build()
                .expect("failed to build the core tokio runtime");
            Core {
                hub: Hub::new(&config.data_dir, Arc::new(LiveBackend)),
                config: config.clone(),
                runtime,
            }
        });
        if first {
            crate::lifecycle::wire(core);
        }
        debug_assert_eq!(
            core.config, config,
            "Core::install called again with a different config"
        );
        core
    }

    /// How many times `install` was called; the host and the engine share one core, so each
    /// process-level caller counts once and the engine asserts it did not create a second.
    pub fn installs() -> usize {
        INSTALLS.load(Ordering::Relaxed)
    }

    /// Panics before `install`; use `try_get` where the core may not exist yet.
    pub fn get() -> &'static Core {
        CORE.get().expect("Core::install was not called")
    }

    pub fn try_get() -> Option<&'static Core> {
        CORE.get()
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn handle(&self) -> &Handle {
        self.runtime.handle()
    }

    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    pub fn config(&self) -> &CoreConfig {
        &self.config
    }

    /// The account hub: app state, session, saved account and the sign-in flows.
    pub fn hub(&self) -> &Hub {
        &self.hub
    }

    /// Runs `f` on the app state machine, one caller at a time. `f` must not call back into the
    /// core's app, and the registered listeners run inside it.
    pub fn with_app<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        self.hub.with_app(f)
    }

    /// Runs `f` on the account state of the running app: the wallet, its ephemeral auth chain and
    /// the sign-in bookkeeping.
    pub fn with_session<R>(&self, f: impl FnOnce(&mut Session) -> R) -> R {
        self.hub.with_session(f)
    }

    /// Registers a receiver of the app's events (the shell and the host each add one); the id
    /// removes it.
    pub fn add_app_listener(&self, listener: Arc<dyn AppEvents>) -> u64 {
        self.hub.events().add_listener(listener)
    }

    pub fn remove_app_listener(&self, id: u64) {
        self.hub.events().remove_listener(id);
    }

    /// Sends `state` to every listener without changing the app (used to exercise the callback
    /// path under load).
    pub fn broadcast_state(&self, state: &crate::app::State) {
        self.hub.events().state_changed(state);
    }

    /// Routes a deep link: finishes a browser sign-in it carries and returns the rest for the
    /// shell (navigation, settings, analytics).
    pub fn handle_deep_link(
        &'static self,
        url: &str,
        start: crate::deep_link_route::StartKind,
    ) -> crate::deep_link_route::Routed {
        let outcome = self.hub.route_deep_link(url, start, self.config.platform);
        if let Some(token) = outcome.complete_sign_in {
            let hub = &self.hub;
            self.handle().spawn(async move {
                if let Err(e) = hub
                    .complete_browser(token.identity_id, token.auth_request_id)
                    .await
                {
                    tracing::warn!("deep link sign-in: {e}");
                }
            });
        }
        self.attribute_open(&outcome.routed);
        outcome.routed
    }

    /// What brought the player in, for the pending `App Opened`: a push, a link, nothing.
    fn attribute_open(&self, routed: &crate::deep_link_route::Routed) {
        use crate::analytics::session;
        if let (Some(trigger), Some(tracker)) =
            (session::trigger_of(&routed.effects), session::global(self))
        {
            session::emit(tracker.deep_link(trigger, session::monotonic_ms()));
        }
    }

    /// Sends an `avatar_preview_mode` event to every listener (exercises the host callback path).
    pub fn broadcast_avatar_preview_mode(&self, on: bool) {
        self.hub.events().avatar_preview_mode(on);
    }

    /// Spawns on the core runtime. Without an installed core it fails in debug builds and drops
    /// the future with an error in release; `dcl_core`'s own tests run it on a short-lived thread
    /// with its own runtime instead.
    pub fn spawn<F>(future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if let Some(core) = Self::try_get() {
            core.handle().spawn(future);
            return;
        }
        #[cfg(test)]
        std::thread::spawn(move || match Runtime::new() {
            Ok(runtime) => {
                runtime.block_on(future);
            }
            Err(e) => tracing::error!("failed to create a fallback runtime: {e}"),
        });
        #[cfg(not(test))]
        {
            drop(future);
            debug_assert!(false, "Core::spawn called before Core::install");
            tracing::error!("Core::spawn called before Core::install; the task was dropped");
        }
    }

    /// Handle to the core runtime, `None` before `install`.
    pub fn try_handle() -> Option<Handle> {
        Self::try_get().map(|core| core.handle().clone())
    }
}

/// A file under the core data dir, `None` before `Core::install`.
pub fn data_file(name: &str) -> Option<PathBuf> {
    Core::try_get().map(|core| core.data_dir().join(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    // One test: the core is a process-wide singleton, so splitting this races under cargo's parallel runner.
    #[test]
    fn spawn_and_install() {
        let (tx, rx) = mpsc::channel();
        let tx2 = tx.clone();
        Core::spawn(async move { tx.send("fallback").unwrap() });
        assert_eq!(rx.recv().unwrap(), "fallback");

        let core = Core::install(CoreConfig {
            data_dir: PathBuf::from("/tmp/dcl"),
            platform: Platform::Desktop,
        });
        assert_eq!(core.data_dir(), Path::new("/tmp/dcl"));
        Core::spawn(async move { tx2.send("core").unwrap() });
        assert_eq!(rx.recv().unwrap(), "core");
        assert!(std::ptr::eq(core, Core::get()));

        let same = Core::install(core.config().clone());
        assert!(std::ptr::eq(core, same));
        assert_eq!(Core::installs(), 2);

        #[cfg(debug_assertions)]
        assert!(std::panic::catch_unwind(|| {
            Core::install(CoreConfig {
                data_dir: PathBuf::from("/other"),
                platform: Platform::Android,
            })
        })
        .is_err());
    }
}
