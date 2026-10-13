use dcl_core::session::store::{SessionSlot, SessionStore};
use dcl_core::Core;
use godot::prelude::*;

/// Static access to the core's `session.json`. Account and guest profile arrive as `var_to_str`
/// text, so the core stores them without knowing what is inside.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclSessionStore {
    _base: Base<RefCounted>,
}

fn store() -> Option<SessionStore> {
    Core::try_get().map(|core| SessionStore::new(core.data_dir()))
}

#[godot_api]
impl DclSessionStore {
    /// True once the core has written `session.json`; before that the old settings file holds
    /// the session and is imported once.
    #[func]
    pub fn exists() -> bool {
        store().is_some_and(|store| store.exists())
    }

    #[func]
    pub fn load_account(suffix: GString) -> GString {
        store()
            .and_then(|store| store.slot(&suffix.to_string()))
            .map(|slot| GString::from(&slot.account))
            .unwrap_or_default()
    }

    #[func]
    pub fn load_guest_profile(suffix: GString) -> GString {
        store()
            .and_then(|store| store.slot(&suffix.to_string()))
            .map(|slot| GString::from(&slot.guest_profile))
            .unwrap_or_default()
    }

    #[func]
    pub fn save_slot(suffix: GString, account: GString, guest_profile: GString) -> bool {
        let Some(store) = store() else {
            return false;
        };
        let slot = SessionSlot {
            account: account.to_string(),
            guest_profile: guest_profile.to_string(),
        };
        match store.set_slot(&suffix.to_string(), slot) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!("session store: failed to save the session: {e}");
                false
            }
        }
    }

    #[func]
    pub fn load_analytics_user_id() -> GString {
        store()
            .map(|store| GString::from(&store.analytics_user_id()))
            .unwrap_or_default()
    }

    #[func]
    pub fn save_analytics_user_id(id: GString) -> bool {
        store().is_some_and(|store| match store.set_analytics_user_id(&id.to_string()) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!("session store: failed to save the analytics id: {e}");
                false
            }
        })
    }
}
