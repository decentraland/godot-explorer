use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use crate::app::{App, AppEvents, HostEvents};
use tokio::runtime::{Builder, Handle, Runtime};

static CORE: OnceLock<Core> = OnceLock::new();
static INSTALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    host_events: Arc<HostEvents>,
    app: Mutex<App>,
}

impl Core {
    /// Installs the core once, with the process logger and panic hook. A second call returns the
    /// first core; in debug builds it must pass the same config, since a different one means two
    /// parts of the process disagree about who owns the core.
    pub fn install(config: CoreConfig) -> &'static Core {
        INSTALLS.fetch_add(1, Ordering::Relaxed);
        let core = CORE.get_or_init(|| {
            crate::logging::init(config.platform);
            let runtime = Builder::new_multi_thread()
                .enable_all()
                .thread_name("dcl-core-tokio")
                .build()
                .expect("failed to build the core tokio runtime");
            let host_events = Arc::new(HostEvents::default());
            Core {
                config: config.clone(),
                runtime,
                app: Mutex::new(App::new(host_events.clone())),
                host_events,
            }
        });
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

    /// Runs `f` on the app state machine, one caller at a time. `f` must not call back into the
    /// core's app, and the registered listener runs inside it.
    pub fn with_app<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        f(&mut self.app.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// Where the app's events go (the shell and the host register here).
    pub fn set_app_listener(&self, listener: Arc<dyn AppEvents>) {
        self.host_events.set_listener(listener);
    }

    /// Spawns on the core runtime. Without an installed core it fails in debug builds; release
    /// builds and `dcl_core`'s own tests run the future on a short-lived thread with its own
    /// runtime instead of dropping it.
    pub fn spawn<F>(future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if let Some(core) = Self::try_get() {
            core.handle().spawn(future);
            return;
        }
        #[cfg(all(debug_assertions, not(test)))]
        panic!("Core::spawn called before Core::install");
        #[allow(unreachable_code)]
        std::thread::spawn(move || match Runtime::new() {
            Ok(runtime) => {
                runtime.block_on(future);
            }
            Err(e) => tracing::error!("failed to create a fallback runtime: {e}"),
        });
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
