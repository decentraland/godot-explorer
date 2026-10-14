//! The typed surface the RN shell and the native hosts call (D-5), generated into TypeScript,
//! Kotlin and Swift from this module. Exported from `libdclgodot`, the one library the engine
//! also lives in, so there is one core per process (D-6). Keys never cross it.

use std::{sync::Arc, time::Duration};

use crate::{
    app::{AppEvents, State},
    Core,
};

#[derive(Debug, uniffi::Error)]
pub enum CoreError {
    NotInstalled,
    InvalidCall { detail: String },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstalled => write!(f, "the core is not installed"),
            Self::InvalidCall { detail } => write!(f, "{detail}"),
        }
    }
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
}

struct ListenerAdapter(Arc<dyn AppListener>);

impl AppEvents for ListenerAdapter {
    fn state_changed(&self, state: &State) {
        self.0.state_changed(state.clone());
    }
    fn engine_required(&self) {}
    fn avatar_preview_mode(&self, _on: bool) {}
    fn scene_crashed(&self) {
        self.0.scene_crashed();
    }
}

fn core() -> Result<&'static Core, CoreError> {
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
    Ok(core()?.add_app_listener(Arc::new(ListenerAdapter(listener))))
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
