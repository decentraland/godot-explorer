//! What the engine provides to the settings: the apply side effects, the content cache and the
//! build's version strings. The engine registers them when it attaches.

use async_trait::async_trait;

use super::{key::SettingKey, value::SettingValue};

/// Executes a setting in the engine (camera far plane, FPS limit, bus volumes, locale...). Called
/// outside the settings lock, after the value is stored. For `GraphicProfile` the engine applies
/// the whole profile, including the settings it keeps to itself.
pub trait EngineApplier: Send + Sync {
    fn apply(&self, key: SettingKey, value: &SettingValue);
}

/// The content cache folder, which the engine reads and writes.
#[async_trait]
pub trait ContentCache: Send + Sync {
    fn total_bytes(&self) -> u64;
    async fn clear(&self);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInfo {
    /// `1.14.0.1844`: the marketing version plus the build number.
    pub short_version: String,
    /// `1.14.0.1844-6cefb54-staging`.
    pub full_version: String,
    pub production: bool,
}
