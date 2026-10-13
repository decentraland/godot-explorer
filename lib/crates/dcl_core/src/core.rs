use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use tokio::runtime::{Builder, Handle, Runtime};

static CORE: OnceLock<Core> = OnceLock::new();

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

#[derive(Clone, Debug)]
pub struct CoreConfig {
    pub data_dir: PathBuf,
    pub env: String,
    pub platform: Platform,
}

/// Process-wide root of the core: configuration plus the tokio runtime every async task runs on.
pub struct Core {
    config: CoreConfig,
    runtime: Runtime,
}

impl Core {
    /// Installs the core once; a second call keeps the first and returns it.
    pub fn install(config: CoreConfig) -> &'static Core {
        CORE.get_or_init(|| {
            let runtime = Builder::new_multi_thread()
                .enable_all()
                .thread_name("dcl-core-tokio")
                .build()
                .expect("failed to build the core tokio runtime");
            Core { config, runtime }
        })
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

    /// Spawns on the core runtime. Without an installed core (unit tests, early boot) it runs the
    /// future on a short-lived thread with its own runtime instead of dropping it.
    pub fn spawn<F>(future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if let Some(core) = Self::try_get() {
            core.handle().spawn(future);
            return;
        }
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
            env: "org".into(),
            platform: Platform::Desktop,
        });
        assert_eq!(core.data_dir(), Path::new("/tmp/dcl"));
        Core::spawn(async move { tx2.send("core").unwrap() });
        assert_eq!(rx.recv().unwrap(), "core");
        assert!(std::ptr::eq(core, Core::get()));

        let again = Core::install(CoreConfig {
            data_dir: PathBuf::from("/other"),
            env: "zone".into(),
            platform: Platform::Android,
        });
        assert!(std::ptr::eq(core, again));
    }
}
