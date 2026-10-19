use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use super::{
    engine::{BuildInfo, ContentCache, EngineApplier},
    key::{SettingEntry, SettingKey},
    rules::{self, PROFILE_DEPENDENT},
    store::{SettingsStore, Values},
    value::SettingValue,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// The value cannot become the key's type.
    InvalidValue(SettingKey),
    UnsupportedLocale(String),
    /// The engine has not registered what the call needs.
    EngineUnavailable,
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue(key) => write!(f, "invalid value for {}", key.name()),
            Self::UnsupportedLocale(locale) => write!(f, "unsupported locale {locale}"),
            Self::EngineUnavailable => write!(f, "the engine is not attached"),
        }
    }
}

impl std::error::Error for SettingsError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct CacheSize {
    pub bytes: u64,
    pub max_bytes: u64,
}

/// Called after any change, from the thread that made it and outside the settings lock.
pub trait SettingsListener: Send + Sync {
    fn settings_changed(&self, entry: &SettingEntry);
}

#[derive(Default)]
struct Engine {
    applier: Option<Arc<dyn EngineApplier>>,
    cache: Option<Arc<dyn ContentCache>>,
    build: Option<BuildInfo>,
}

pub struct Settings {
    store: SettingsStore,
    values: Mutex<Values>,
    engine: Mutex<Engine>,
    listeners: Mutex<Vec<(u64, Arc<dyn SettingsListener>)>>,
    next_listener: Mutex<u64>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Settings {
    pub fn open(data_dir: &Path) -> Self {
        let store = SettingsStore::new(data_dir);
        let values = store.load();
        Self {
            store,
            values: Mutex::new(values),
            engine: Mutex::new(Engine::default()),
            listeners: Mutex::new(Vec::new()),
            next_listener: Mutex::new(1),
        }
    }

    pub fn set_applier(&self, applier: Arc<dyn EngineApplier>) {
        lock(&self.engine).applier = Some(applier);
    }

    pub fn set_content_cache(&self, cache: Arc<dyn ContentCache>) {
        lock(&self.engine).cache = Some(cache);
    }

    pub fn set_build(&self, build: BuildInfo) {
        lock(&self.engine).build = Some(build);
    }

    pub fn add_listener(&self, listener: Arc<dyn SettingsListener>) -> u64 {
        let mut next = lock(&self.next_listener);
        let id = *next;
        *next += 1;
        lock(&self.listeners).push((id, listener));
        id
    }

    pub fn remove_listener(&self, id: u64) {
        lock(&self.listeners).retain(|(listener_id, _)| *listener_id != id);
    }

    pub fn get(&self, key: SettingKey) -> SettingValue {
        lock(&self.values)
            .get(&key)
            .cloned()
            .unwrap_or_else(|| key.default_value())
    }

    /// Every key with its value, in key order.
    pub fn entries(&self) -> Vec<SettingEntry> {
        let values = lock(&self.values);
        SettingKey::ALL
            .into_iter()
            .filter(|key| key.shell_visible())
            .map(|key| SettingEntry {
                key,
                value: values
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| key.default_value()),
            })
            .collect()
    }

    /// Stores the value, applies it in the engine and announces it. `commit` also saves to disk
    /// (every stored value, not only this one); the locale always saves. Returns the announced
    /// entries: the key itself, then, for a graphic profile, the three settings it drives.
    pub fn set(
        &self,
        key: SettingKey,
        value: SettingValue,
        commit: bool,
    ) -> Result<Vec<SettingEntry>, SettingsError> {
        let value = value
            .coerce(key.kind())
            .ok_or(SettingsError::InvalidValue(key))?;
        if let SettingValue::Text(locale) = &value {
            if key == SettingKey::Locale && !rules::locale_supported(locale, self.allows_pseudo()) {
                return Err(SettingsError::UnsupportedLocale(locale.clone()));
            }
        }
        let announced = self.store_value(key, &value, commit || key == SettingKey::Locale);
        if key.needs_apply() {
            if let Some(applier) = self.applier() {
                applier.apply(key, &value);
            }
        }
        self.announce(&announced);
        Ok(announced)
    }

    /// Takes a value the engine changed on its own (dynamic graphics picking a profile): stored
    /// and announced, not applied back and not saved.
    pub fn adopt(
        &self,
        key: SettingKey,
        value: SettingValue,
    ) -> Result<Vec<SettingEntry>, SettingsError> {
        let value = value
            .coerce(key.kind())
            .ok_or(SettingsError::InvalidValue(key))?;
        let announced = self.store_value(key, &value, false);
        self.announce(&announced);
        Ok(announced)
    }

    /// Writes every stored value to disk.
    pub fn save(&self) {
        let values = lock(&self.values);
        if let Err(error) = self.store.save(&values) {
            tracing::error!("settings: could not save: {error}");
        }
    }

    /// Applies every stored value; the engine calls it once when it attaches.
    pub fn apply_all(&self) {
        let Some(applier) = self.applier() else {
            return;
        };
        for entry in self.entries().iter().filter(|e| e.key.needs_apply()) {
            applier.apply(entry.key, &entry.value);
        }
    }

    pub fn cache_size(&self) -> CacheSize {
        let max_bytes = match self.get(SettingKey::MaxCacheSize) {
            SettingValue::Int(index) => rules::max_cache_bytes(index),
            _ => rules::max_cache_bytes(1),
        };
        let bytes = lock(&self.engine)
            .cache
            .as_ref()
            .map_or(0, |c| c.total_bytes());
        CacheSize { bytes, max_bytes }
    }

    /// Empties the cache folder and returns what is left in it.
    pub async fn clear_cache(&self) -> Result<u64, SettingsError> {
        let cache = lock(&self.engine)
            .cache
            .clone()
            .ok_or(SettingsError::EngineUnavailable)?;
        cache.clear().await;
        Ok(cache.total_bytes())
    }

    pub fn version(&self) -> Result<String, SettingsError> {
        let engine = lock(&self.engine);
        let build = engine
            .build
            .as_ref()
            .ok_or(SettingsError::EngineUnavailable)?;
        Ok(rules::display_version(build))
    }

    fn allows_pseudo(&self) -> bool {
        lock(&self.engine)
            .build
            .as_ref()
            .is_some_and(|b| !b.production)
    }

    fn applier(&self) -> Option<Arc<dyn EngineApplier>> {
        lock(&self.engine).applier.clone()
    }

    fn store_value(&self, key: SettingKey, value: &SettingValue, save: bool) -> Vec<SettingEntry> {
        let mut values = lock(&self.values);
        values.insert(key, value.clone());
        let mut announced = vec![SettingEntry {
            key,
            value: value.clone(),
        }];
        if key == SettingKey::GraphicProfile {
            if let SettingValue::Int(profile) = value {
                for (dependent, preset) in rules::profile_values(*profile) {
                    values.insert(dependent, preset);
                }
            }
            for dependent in PROFILE_DEPENDENT {
                announced.push(SettingEntry {
                    key: dependent,
                    value: values
                        .get(&dependent)
                        .cloned()
                        .unwrap_or_else(|| dependent.default_value()),
                });
            }
        }
        if save {
            if let Err(error) = self.store.save(&values) {
                tracing::error!("settings: could not save: {error}");
            }
        }
        announced
    }

    fn announce(&self, entries: &[SettingEntry]) {
        let listeners: Vec<_> = lock(&self.listeners)
            .iter()
            .map(|(_, l)| l.clone())
            .collect();
        for entry in entries.iter().filter(|entry| entry.key.shell_visible()) {
            for listener in &listeners {
                listener.settings_changed(entry);
            }
        }
    }
}
