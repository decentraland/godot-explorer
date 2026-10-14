//! Ties the domain services to the account: friends start after a real sign-in and every
//! account-bound service forgets the user at sign-out; their change events reach the listeners.

use std::sync::Arc;

use crate::{
    app::{AppEvents, HostEvents},
    hub::{GuestLoginReport, SessionObserver},
    profile,
    session::identity::SessionEvent,
    settings::{self, SettingEntry, SettingsListener},
    social, Core,
};

struct Lifecycle;

impl SessionObserver for Lifecycle {
    fn session_event(&self, event: &SessionEvent) {
        match event {
            // Guests have no social service.
            SessionEvent::WalletConnected {
                is_guest: false, ..
            } => {
                if let Err(e) = social::friends().start() {
                    tracing::warn!("friends: could not start: {e}");
                }
            }
            SessionEvent::WalletConnected { .. } => {}
            SessionEvent::Logout => {
                social::friends().reset();
                profile::global().reset();
            }
        }
    }

    fn guest_login(&self, _report: &GuestLoginReport) {}
}

struct SettingsToEvents(Arc<HostEvents>);

impl SettingsListener for SettingsToEvents {
    fn settings_changed(&self, entry: &SettingEntry) {
        self.0.settings_changed(entry);
    }
}

/// Run once from `Core::install`, after the core exists.
pub(crate) fn wire(core: &'static Core) {
    let events = core.hub().events().clone();
    core.hub().add_observer(Arc::new(Lifecycle));
    settings::global(core.data_dir()).add_listener(Arc::new(SettingsToEvents(events.clone())));
    social::friends().add_friends_changed_listener(Arc::new(move || events.friends_changed()));
}
