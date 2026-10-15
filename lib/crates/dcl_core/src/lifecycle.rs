//! Ties the domain services to the account: every account-bound service forgets the user at
//! sign-out; their change events reach the listeners.

use std::sync::Arc;

use crate::{
    app::{AppEvents, HostEvents},
    hub::{GuestLoginReport, SessionObserver},
    profile,
    report::ReportEvent,
    session::identity::SessionEvent,
    settings::{self, SettingEntry, SettingsListener},
    Core,
};

struct Lifecycle;

impl SessionObserver for Lifecycle {
    fn session_event(&self, event: &SessionEvent) {
        if matches!(event, SessionEvent::Logout) {
            profile::global().reset();
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
    crate::logging::set_error_sink(Box::new(move |message, is_panic| {
        core.hub().report(if is_panic {
            ReportEvent::panic(message)
        } else {
            ReportEvent::logged_error(message)
        });
    }));
    settings::global(core.data_dir()).add_listener(Arc::new(SettingsToEvents(events.clone())));
}
