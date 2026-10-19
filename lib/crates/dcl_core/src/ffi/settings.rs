//! FFI for the settings domain: typed keys and values, no JSON. The engine's side (applier,
//! content cache, build info) is registered on `dcl_core::settings::global`.

use crate::settings::{self, CacheSize, SettingEntry, SettingKey, SettingValue, SettingsError};

use super::{core, CoreError};

impl From<SettingsError> for CoreError {
    fn from(error: SettingsError) -> Self {
        Self::InvalidCall {
            detail: error.to_string(),
        }
    }
}

fn store() -> Result<&'static settings::Settings, CoreError> {
    Ok(settings::global(core()?.data_dir()))
}

/// Every setting the shell can read, with its current value.
#[uniffi::export]
pub fn settings() -> Result<Vec<SettingEntry>, CoreError> {
    Ok(store()?.entries())
}

/// The name `settings.cfg` and the shell's keys file use for `key`.
#[uniffi::export]
pub fn setting_key_name(key: SettingKey) -> String {
    key.name().to_string()
}

#[uniffi::export]
pub fn setting_key_from_name(name: String) -> Option<SettingKey> {
    SettingKey::from_name(&name)
}

/// Sets a value (a number of the wrong width is coerced, a wrong type is refused). `commit`
/// saves it to disk; without it the change lasts until the next commit. Returns every entry
/// that changed or was re-announced: the key, then the three settings a graphic profile drives.
#[uniffi::export]
pub fn set_setting(
    key: SettingKey,
    value: SettingValue,
    commit: bool,
) -> Result<Vec<SettingEntry>, CoreError> {
    Ok(store()?.set(key, value, commit)?)
}

#[uniffi::export]
pub fn cache_size() -> Result<CacheSize, CoreError> {
    Ok(store()?.cache_size())
}

/// Empties the content cache and returns the bytes it still holds.
#[uniffi::export]
pub async fn clear_cache() -> Result<u64, CoreError> {
    let core = core()?;
    let store = settings::global(core.data_dir());
    core.handle()
        .spawn(async move { store.clear_cache().await })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?
        .map_err(CoreError::from)
}

/// The version to show: `x.y.z` in production, the full version otherwise.
#[uniffi::export]
pub fn version() -> Result<String, CoreError> {
    Ok(store()?.version()?)
}
