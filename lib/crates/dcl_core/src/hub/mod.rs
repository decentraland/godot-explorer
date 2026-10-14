//! The account hub: the app state machine, the session, its persistence and the sign-in flows,
//! one object so a flow can move the state, the wallet and the saved account together. `Core`
//! owns the process's hub; tests build their own with a fake backend.

mod backend;
mod signin;

#[cfg(test)]
mod tests;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crate::{
    app::{App, HostEvents},
    session::{
        account_codec,
        identity::{Session, SessionEvent},
        store::{SessionSlot, SessionStore},
    },
};

pub use backend::{LiveBackend, SignInBackend};
pub use signin::SignInError;

/// Observes what the sign-in flows do to the session. The engine's identity node mirrors it into
/// Godot signals for the engine-side GDScript that still reads the account.
pub trait SessionObserver: Send + Sync {
    fn session_event(&self, event: &SessionEvent);
    /// One report per silent guest-login attempt, for analytics.
    fn guest_login(&self, report: &GuestLoginReport);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestLoginReport {
    pub success: bool,
    pub is_new_user: bool,
    /// Stable failure code; empty on success.
    pub failure_reason: String,
    pub http_status: Option<u16>,
    pub duration_ms: u64,
}

pub struct Hub {
    app: Mutex<App>,
    session: Mutex<Session>,
    events: Arc<HostEvents>,
    store: SessionStore,
    profile_suffix: Mutex<String>,
    device_anchor: Mutex<String>,
    /// Off for throwaway sessions (previews, benchmarks): they must not overwrite a saved account.
    persist: AtomicBool,
    backend: Arc<dyn SignInBackend>,
    observers: Mutex<Vec<Arc<dyn SessionObserver>>>,
    wallet_pending: Mutex<Option<signin::PendingWallet>>,
}

impl Hub {
    pub fn new(data_dir: &std::path::Path, backend: Arc<dyn SignInBackend>) -> Self {
        let events = Arc::new(HostEvents::default());
        Self {
            app: Mutex::new(App::new(events.clone())),
            session: Mutex::new(Session::default()),
            events,
            store: SessionStore::new(data_dir),
            profile_suffix: Mutex::new(String::new()),
            device_anchor: Mutex::new(String::new()),
            persist: AtomicBool::new(true),
            backend,
            observers: Mutex::new(Vec::new()),
            wallet_pending: Mutex::new(None),
        }
    }

    /// Runs `f` on the app state machine, one caller at a time. `f` must not call back into the
    /// hub's app, and the registered listeners run inside it.
    pub fn with_app<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        f(&mut self.app.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// Runs `f` on the wallet, its ephemeral auth chain and the sign-in bookkeeping.
    pub fn with_session<R>(&self, f: impl FnOnce(&mut Session) -> R) -> R {
        f(&mut self.session.lock().unwrap_or_else(|e| e.into_inner()))
    }

    pub fn events(&self) -> &Arc<HostEvents> {
        &self.events
    }

    pub fn add_observer(&self, observer: Arc<dyn SessionObserver>) {
        self.observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(observer);
    }

    /// Which saved profile slot this run uses (`--profile`, or the deep link's).
    pub fn set_profile_suffix(&self, suffix: &str) {
        *self
            .profile_suffix
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = suffix.to_string();
    }

    /// The platform's stable device id the guest wallet derives from; empty falls back to a
    /// per-install file.
    pub fn set_device_anchor(&self, anchor: &str) {
        *self.device_anchor.lock().unwrap_or_else(|e| e.into_inner()) = anchor.to_string();
    }

    pub fn set_persist_session(&self, persist: bool) {
        self.persist.store(persist, Ordering::Relaxed);
    }

    /// Ends `Booting`: restores the saved account of the slot, if it is still valid.
    pub fn boot(&self) {
        let recovered = self
            .store
            .slot(&self.suffix())
            .and_then(|slot| account_codec::decode(&slot.account))
            .and_then(|account| self.with_session(|s| s.recover(account)));
        let signed_in = recovered.is_some();
        if let Some(event) = recovered {
            self.notify(&event);
        }
        if let Err(e) = self.with_app(|app| app.finish_boot(signed_in)) {
            tracing::warn!("boot: {e}");
        }
    }

    pub(crate) fn suffix(&self) -> String {
        self.profile_suffix
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(crate) fn notify(&self, event: &SessionEvent) {
        let observers = self
            .observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for observer in observers {
            observer.session_event(event);
        }
    }

    fn notify_guest_login(&self, report: &GuestLoginReport) {
        let observers = self
            .observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for observer in observers {
            observer.guest_login(report);
        }
    }

    /// Writes the running account to its slot, keeping the slot's guest profile.
    pub(crate) fn save_account(&self) {
        if !self.persist.load(Ordering::Relaxed) {
            return;
        }
        let account = self
            .with_session(|s| s.recoverable_account())
            .map(|account| account_codec::encode(&account))
            .unwrap_or_default();
        self.write_slot_account(account);
    }

    pub(crate) fn clear_saved_account(&self) {
        if self.persist.load(Ordering::Relaxed) {
            self.write_slot_account(String::new());
        }
    }

    fn write_slot_account(&self, account: String) {
        let suffix = self.suffix();
        let guest_profile = self
            .store
            .slot(&suffix)
            .map(|slot| slot.guest_profile)
            .unwrap_or_default();
        if let Err(e) = self.store.set_slot(
            &suffix,
            SessionSlot {
                account,
                guest_profile,
            },
        ) {
            tracing::error!("session: failed to save the account: {e}");
        }
    }
}
