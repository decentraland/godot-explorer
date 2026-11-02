//! Player settings: the keys the shell shows, their storage and what the engine applies.

mod engine;
mod key;
mod legacy;
mod rules;
mod service;
mod store;
mod value;

#[cfg(test)]
mod tests;

pub use engine::{BuildInfo, ContentCache, EngineApplier};
pub use key::{SettingEntry, SettingKey};
pub use rules::{display_version, max_cache_bytes};
pub use service::{CacheSize, Settings, SettingsError, SettingsListener};
pub use value::{SettingKind, SettingValue};

use std::{path::Path, sync::OnceLock};

static GLOBAL: OnceLock<Settings> = OnceLock::new();

/// The process-wide settings, opened on first use from the core's data dir.
pub fn global(data_dir: &Path) -> &'static Settings {
    GLOBAL.get_or_init(|| Settings::open(data_dir))
}
