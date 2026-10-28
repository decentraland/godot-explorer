//! The account hub: the app state machine, the session, its persistence and the sign-in flows,
//! one object so a flow can move the state, the wallet and the saved account together. `Core`
//! owns the process's hub; tests build their own with a fake backend.

mod backend;
mod deletion;
mod links;
mod report;
mod signin;

#[cfg(test)]
mod deletion_tests;
#[cfg(test)]
mod tests;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crate::{
    app::{App, HostEvents},
    deep_link_route::{DeepLinkRouter, SystemClock},
    report::{BuildEnvironment, ReportPolicy, ReportTags, TagCoalescer},
    session::{
        account_codec,
        identity::{Session, SessionEvent},
        signin_verifier::VerifierStore,
        store::{SessionSlot, SessionStore},
    },
};

/// How often at most the shell hears about report-tag changes.
const REPORT_TAG_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

pub use backend::{LiveBackend, SignInBackend};
pub use deletion::{DeletionPlan, GUEST_DEVICE_FILES};
pub use links::LinkOutcome;
pub use report::ReportPayload;
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
    /// Secrets that bind a browser sign-in to this install.
    verifiers: VerifierStore,
    /// `authRequestId` of the browser sign-in this process started, so a cancel can drop its
    /// verifier.
    browser_request: Mutex<Option<String>>,
    profile_suffix: Mutex<String>,
    device_anchor: Mutex<String>,
    /// Off for throwaway sessions (previews, benchmarks): they must not overwrite a saved account.
    persist: AtomicBool,
    backend: Arc<dyn SignInBackend>,
    observers: Mutex<Vec<Arc<dyn SessionObserver>>>,
    wallet_pending: Mutex<Option<signin::PendingWallet>>,
    router: Mutex<DeepLinkRouter>,
    /// The keep/drop policy and the tags every kept report carries.
    report: Mutex<(ReportPolicy, ReportTags)>,
    report_tags: Arc<TagCoalescer>,
    /// Gates developer-only link parameters; on until the engine says this is a dev build.
    production: AtomicBool,
    /// Entity ids of the crashed scenes already reported: one issue per scene, and a cap.
    scene_crashes: Mutex<std::collections::HashSet<String>>,
    /// A developer link allowed deleting an upgraded guest; never on in production.
    upgraded_deletion: AtomicBool,
    data_dir: std::path::PathBuf,
}

impl Hub {
    pub fn new(data_dir: &std::path::Path, backend: Arc<dyn SignInBackend>) -> Self {
        let events = Arc::new(HostEvents::default());
        Self {
            app: Mutex::new(App::new(events.clone())),
            session: Mutex::new(Session::default()),
            events,
            store: SessionStore::new(data_dir),
            verifiers: VerifierStore::new(data_dir),
            browser_request: Mutex::new(None),
            profile_suffix: Mutex::new(String::new()),
            device_anchor: Mutex::new(String::new()),
            persist: AtomicBool::new(true),
            backend,
            observers: Mutex::new(Vec::new()),
            wallet_pending: Mutex::new(None),
            router: Mutex::new(Self::new_router(Arc::new(SystemClock::new()))),
            report: Mutex::new((
                ReportPolicy::with_thread_random(BuildEnvironment::Production),
                ReportTags::default(),
            )),
            report_tags: Arc::new(TagCoalescer::new(REPORT_TAG_INTERVAL)),
            production: AtomicBool::new(true),
            scene_crashes: Mutex::new(Default::default()),
            upgraded_deletion: AtomicBool::new(false),
            data_dir: data_dir.to_path_buf(),
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

    /// Whether this is a production build; defaults to yes, the safe side for developer links.
    pub fn set_production(&self, production: bool) {
        self.production.store(production, Ordering::Relaxed);
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
        self.tag_reports_with_identity(event);
    }

    /// Every report says whether a guest was playing; a signed-out player counts as one.
    fn tag_reports_with_identity(&self, event: &SessionEvent) {
        let guest = match event {
            SessionEvent::WalletConnected { .. } => {
                self.with_session(|s| s.is_thirdweb_guest || s.is_disposable_guest())
            }
            SessionEvent::Logout => true,
        };
        self.set_report_tag("is_guest", if guest { "true" } else { "false" });
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
