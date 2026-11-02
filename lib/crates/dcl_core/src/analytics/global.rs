//! The process-wide analytics service: one per process, installed once by the host after the
//! core, and reached by the shell's FFI and the engine's adapters.

use std::sync::{Arc, OnceLock};

use super::{
    service::{Analytics, AnalyticsConfig, DEFAULT_WRITE_KEY},
    transport::ReqwestSegmentTransport,
};
use crate::{session::store::SessionStore, Core};

static ANALYTICS: OnceLock<Arc<Analytics>> = OnceLock::new();

/// What the host knows that the core does not.
#[derive(Clone, Debug)]
pub struct AnalyticsInstall {
    pub session_id: String,
    /// The platform name Godot reports ("Android", "iOS", "Linux", ...).
    pub os_name: String,
    pub renderer_version: String,
    pub utc_offset_minutes: i32,
}

/// Installs the service once (a second call returns the first). It reads or creates the
/// per-install anonymous id, follows the session's identity and starts the periodic flush. Call
/// it before any sign-in, so the first wallet is seen.
pub fn install(core: &'static Core, install: AnalyticsInstall) -> &'static Arc<Analytics> {
    ANALYTICS.get_or_init(|| {
        let config = AnalyticsConfig {
            write_key: DEFAULT_WRITE_KEY.to_string(),
            user_id: user_id(&SessionStore::new(core.data_dir())),
            session_id: install.session_id,
            os_name: install.os_name,
            renderer_version: install.renderer_version,
            utc_offset_minutes: install.utc_offset_minutes,
        };
        let analytics = Arc::new(Analytics::new(
            config,
            Arc::new(ReqwestSegmentTransport::new()),
        ));
        core.hub().add_observer(analytics.clone());
        analytics.start_flush_loop(core.handle());
        analytics
    })
}

pub fn analytics() -> Option<&'static Arc<Analytics>> {
    ANALYTICS.get()
}

/// The saved anonymous id, or a new one saved for the next launch.
fn user_id(store: &SessionStore) -> String {
    let saved = store.analytics_user_id();
    if !saved.is_empty() {
        return saved;
    }
    let fresh = uuid::Uuid::new_v4().to_string();
    if let Err(e) = store.set_analytics_user_id(&fresh) {
        tracing::warn!("could not save the analytics user id: {e}");
    }
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_id_is_created_once_and_then_kept() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::new(dir.path());
        let first = user_id(&store);
        assert_eq!(first.len(), 36);
        assert_eq!(user_id(&store), first);
        assert_eq!(store.analytics_user_id(), first);
    }

    #[test]
    fn a_saved_id_is_used_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::new(dir.path());
        store.set_analytics_user_id("legacy-id").unwrap();
        assert_eq!(user_id(&store), "legacy-id");
    }
}
