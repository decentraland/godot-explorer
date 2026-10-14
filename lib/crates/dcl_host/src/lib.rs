//! The host's side of the core (D-41), generated into Kotlin and Swift from `libdclgodot`.

uniffi::setup_scaffolding!();

mod attestation;
pub use attestation::{
    attestation_status, install_attestation, AttestationFailure, AttestationService,
    AttestationStatus,
};

use std::{path::PathBuf, sync::Arc, time::Duration};

use dcl_core::{app::AppEvents, Core, CoreConfig, Platform};

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum HostPlatform {
    Android,
    Ios,
}

#[derive(Debug, uniffi::Error)]
pub enum HostError {
    NotInstalled,
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the core is not installed")
    }
}

impl std::error::Error for HostError {}

/// Implemented in Kotlin/Swift. Called from a core thread.
#[uniffi::export(with_foreign)]
pub trait HostListener: Send + Sync {
    /// The host creates the Godot view; the engine then attaches itself (D-42).
    fn engine_required(&self);
    /// The host flips the surface order.
    fn avatar_preview_mode(&self, on: bool);
}

struct HostAdapter(Arc<dyn HostListener>);

impl AppEvents for HostAdapter {
    fn state_changed(&self, _state: &dcl_core::app::State) {}
    fn engine_required(&self) {
        self.0.engine_required();
    }
    fn avatar_preview_mode(&self, on: bool) {
        self.0.avatar_preview_mode(on);
    }
    fn scene_crashed(&self) {}
}

pub(crate) fn core() -> Result<&'static Core, HostError> {
    Core::try_get().ok_or(HostError::NotInstalled)
}

/// Creates the process's one core at app launch, with the data dir the Godot-exported app used
/// (D-19), restores the saved account and starts analytics. `device_anchor` is the platform's
/// stable device id (SSAID, Keychain item) the guest wallet derives from; `app_version` is the
/// store version. Calling it again with the same arguments is a no-op.
#[uniffi::export]
pub fn install(
    data_dir: String,
    platform: HostPlatform,
    device_anchor: String,
    app_version: String,
) {
    let first = Core::try_get().is_none();
    let core = Core::install(CoreConfig {
        data_dir: PathBuf::from(data_dir),
        platform: match platform {
            HostPlatform::Android => Platform::Android,
            HostPlatform::Ios => Platform::Ios,
        },
    });
    if first {
        core.hub().set_device_anchor(&device_anchor);
        dcl_core::analytics::global::install(
            core,
            dcl_core::analytics::global::AnalyticsInstall {
                session_id: uuid::Uuid::new_v4().to_string(),
                os_name: match platform {
                    HostPlatform::Android => "Android",
                    HostPlatform::Ios => "iOS",
                }
                .to_string(),
                renderer_version: app_version,
                utc_offset_minutes: chrono::Local::now().offset().local_minus_utc() / 60,
            },
        );
        core.hub().boot();
    }
}

/// Registers the host's receiver of `engine_required` and `avatar_preview_mode`.
#[uniffi::export]
pub fn add_host_listener(listener: Arc<dyn HostListener>) -> Result<u64, HostError> {
    Ok(core()?.add_app_listener(Arc::new(HostAdapter(listener))))
}

#[uniffi::export]
pub fn remove_host_listener(id: u64) -> Result<(), HostError> {
    core()?.remove_app_listener(id);
    Ok(())
}

/// Sends `count` `avatar_preview_mode` events (alternating on and off) to every listener from a
/// core thread, `per_second` each second. Proves a Kotlin/Swift trait implementation is called
/// from a tokio thread.
#[uniffi::export]
pub async fn debug_fire_host_events(count: u32, per_second: u32) -> Result<u32, HostError> {
    let core = core()?;
    let interval = Duration::from_secs_f64(1.0 / per_second.max(1) as f64);
    let task = core.handle().spawn(async move {
        for i in 0..count {
            core.broadcast_avatar_preview_mode(i % 2 == 0);
            tokio::time::sleep(interval).await;
        }
        count
    });
    task.await.map_err(|_| HostError::NotInstalled)
}
