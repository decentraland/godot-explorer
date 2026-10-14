//! Attestation: exchanges the OS integrity proof for a session token (port of attestation_service.gd).
//!
//! Concurrent callers share one attestation cycle. A failed attempt retries with backoff forever
//! and never blocks the UI; callers on unsupported devices get `AttestError::Unsupported` instead.

mod flow;
mod host;
mod store;
#[cfg(test)]
mod tests;
mod transport;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{sync::broadcast, time::Instant};

pub use flow::{Failure, Timings};
pub use host::AttestationHost;
pub use store::{CacheLoad, Session, EXPIRY_MARGIN_SECS, SESSION_FILE};
pub use transport::{HttpFailure, ReqwestTransport, Transport};

use crate::{urls, Platform};

pub const EULA_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Attestation traffic of a production build is pinned here whatever `dclenv` says: the
/// attestation gate is the anti-phishing control, so a deeplink must not redirect it.
const PROD_ATTESTATION_BFF: &str = "https://mobile-bff.decentraland.org";

/// True once the player has accepted the current terms and conditions.
pub type EulaCheck = Arc<dyn Fn() -> bool + Send + Sync>;

/// One line per attempt, for analytics.
pub type AttemptObserver = Arc<dyn Fn(&AttemptReport) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestError {
    Unsupported,
    CoreNotInstalled,
    /// The attestation was dropped before a token was produced.
    Closed,
}

impl std::fmt::Display for AttestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "attestation is not supported on this device"),
            Self::CoreNotInstalled => write!(f, "core not installed, no data dir for the session"),
            Self::Closed => write!(f, "attestation closed before a token was produced"),
        }
    }
}

impl std::error::Error for AttestError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    WaitingEula,
    Attesting,
    Backoff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Boot,
    OnDemand,
    ForceReattest,
}

impl Trigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::OnDemand => "on_demand",
            Self::ForceReattest => "force_reattest",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptReport {
    pub cycle_id: String,
    /// 1-based.
    pub attempt: u32,
    pub trigger: Trigger,
    pub duration_ms: u64,
    pub timings: Timings,
    /// The remaining lifetime of the new session in seconds on success, the failure otherwise.
    pub outcome: Result<i64, Failure>,
}

struct Shared {
    state: State,
    session: Option<Session>,
    trigger: Trigger,
    last_failure: Option<Failure>,
}

struct Inner {
    host: Arc<dyn AttestationHost>,
    transport: Arc<dyn Transport>,
    eula: EulaCheck,
    path: PathBuf,
    production: bool,
    cache_load: CacheLoad,
    shared: Mutex<Shared>,
    refreshed: broadcast::Sender<Session>,
    observer: Mutex<Option<AttemptObserver>>,
}

#[derive(Clone)]
pub struct Attestation {
    inner: Arc<Inner>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Attestation {
    /// Uses the core data dir and the real HTTP transport. `production` pins the mobile-bff.
    pub fn new(
        host: Arc<dyn AttestationHost>,
        eula: EulaCheck,
        production: bool,
    ) -> Result<Self, AttestError> {
        let path = crate::data_file(SESSION_FILE).ok_or(AttestError::CoreNotInstalled)?;
        Ok(Self::with_parts(
            host,
            Arc::new(ReqwestTransport::new()),
            eula,
            path,
            production,
        ))
    }

    /// `session_path` is the session file; a file written by the GDScript service loads as is.
    pub fn with_parts(
        host: Arc<dyn AttestationHost>,
        transport: Arc<dyn Transport>,
        eula: EulaCheck,
        session_path: PathBuf,
        production: bool,
    ) -> Self {
        let (session, cache_load) = store::load(&session_path, unix_now());
        let (refreshed, _) = broadcast::channel(4);
        Self {
            inner: Arc::new(Inner {
                host,
                transport,
                eula,
                path: session_path,
                production,
                cache_load,
                shared: Mutex::new(Shared {
                    state: State::Idle,
                    session,
                    trigger: Trigger::Boot,
                    last_failure: None,
                }),
                refreshed,
                observer: Mutex::new(None),
            }),
        }
    }

    pub fn on_attempt(&self, observer: AttemptObserver) {
        *lock(&self.inner.observer) = Some(observer);
    }

    /// True iff this device can produce attestation artifacts.
    pub fn is_supported(&self) -> bool {
        self.inner.host.is_supported()
    }

    /// A non-expired token is in memory; false within `EXPIRY_MARGIN_SECS` of expiry.
    pub fn has_valid_session(&self) -> bool {
        self.cached_token().is_some()
    }

    pub fn state(&self) -> State {
        lock(&self.inner.shared).state
    }

    /// How the session file on disk looked when this was created.
    pub fn cache_load(&self) -> CacheLoad {
        self.inner.cache_load
    }

    /// The failure of the latest attempt, cleared by the next success.
    pub fn last_failure(&self) -> Option<Failure> {
        lock(&self.inner.shared).last_failure.clone()
    }

    /// Warms the session once the EULA is accepted. No-op when unsupported or already valid.
    pub fn start(&self) {
        if !self.is_supported() || self.has_valid_session() {
            return;
        }
        let this = self.clone();
        tokio::spawn(async move { this.boot_dispatch().await });
    }

    /// A valid token: from the cache, or from a new cycle (shared with concurrent callers).
    pub async fn valid_jwt(&self) -> Result<String, AttestError> {
        self.await_token(Trigger::OnDemand, false).await
    }

    /// Drops the cached session and attests again, after a downstream call rejected the token.
    pub async fn force_reattest(&self) -> Result<String, AttestError> {
        self.clear_session();
        self.await_token(Trigger::ForceReattest, true).await
    }

    async fn await_token(&self, trigger: Trigger, skip_cache: bool) -> Result<String, AttestError> {
        loop {
            if !skip_cache {
                if let Some(token) = self.cached_token() {
                    return Ok(token);
                }
            }
            if !self.is_supported() {
                return Err(AttestError::Unsupported);
            }
            let mut refreshed = self.inner.refreshed.subscribe();
            self.kick(trigger);
            match refreshed.recv().await {
                Ok(session) => return Ok(session.token),
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if self.cached_token().is_some() {
                        continue;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return Err(AttestError::Closed),
            }
        }
    }

    fn cached_token(&self) -> Option<String> {
        let shared = lock(&self.inner.shared);
        let session = shared.session.as_ref()?;
        session
            .is_valid_at(unix_now())
            .then(|| session.token.clone())
    }

    fn clear_session(&self) {
        lock(&self.inner.shared).session = None;
        store::clear(&self.inner.path);
    }

    async fn boot_dispatch(&self) {
        if self.has_valid_session() {
            return;
        }
        if !(self.inner.eula)() {
            {
                let mut shared = lock(&self.inner.shared);
                if shared.state != State::Idle {
                    return;
                }
                shared.state = State::WaitingEula;
            }
            while !(self.inner.eula)() {
                tokio::time::sleep(EULA_POLL_INTERVAL).await;
            }
            lock(&self.inner.shared).state = State::Idle;
        }
        self.kick(Trigger::Boot);
    }

    /// Starts a cycle iff idle: not while waiting for the EULA or with a cycle in flight.
    fn kick(&self, trigger: Trigger) {
        {
            let mut shared = lock(&self.inner.shared);
            shared.trigger = trigger;
            if shared.state != State::Idle || !self.is_supported() {
                tracing::debug!(
                    "[Attestation] kick({}) skipped, state={:?}",
                    trigger.as_str(),
                    shared.state
                );
                return;
            }
            shared.state = State::Attesting;
        }
        let this = self.clone();
        tokio::spawn(async move { this.run_cycle().await });
    }

    async fn run_cycle(&self) {
        let cycle_id = uuid::Uuid::new_v4().to_string();
        let trigger = lock(&self.inner.shared).trigger;
        let bff = self.bff_url();
        let mut attempt: u32 = 0;
        loop {
            let started = Instant::now();
            let result = flow::attempt(&*self.inner.host, &*self.inner.transport, &bff).await;
            let duration_ms = started.elapsed().as_millis() as u64;
            attempt += 1;
            match result {
                Ok((session, timings)) => {
                    self.report(
                        &cycle_id,
                        attempt,
                        trigger,
                        duration_ms,
                        timings,
                        Ok(session.expires_at - unix_now()),
                    );
                    self.finish(session);
                    return;
                }
                Err(failure) => {
                    let failure = *failure;
                    let delay = flow::RETRY_BACKOFF
                        [(attempt as usize - 1).min(flow::RETRY_BACKOFF.len() - 1)];
                    tracing::warn!(
                        "[Attestation] cycle[{}] attempt #{attempt} failed in {duration_ms}ms ({}) -> retry in {delay:?}",
                        &cycle_id[..8],
                        failure.message
                    );
                    self.report(
                        &cycle_id,
                        attempt,
                        trigger,
                        duration_ms,
                        failure.timings.clone(),
                        Err(failure.clone()),
                    );
                    {
                        let mut shared = lock(&self.inner.shared);
                        shared.last_failure = Some(failure);
                        shared.state = State::Backoff;
                    }
                    tokio::time::sleep(delay).await;
                    lock(&self.inner.shared).state = State::Attesting;
                }
            }
        }
    }

    fn finish(&self, session: Session) {
        store::save(
            &self.inner.path,
            &session,
            platform_name(self.inner.host.platform()),
        );
        tracing::info!(
            "[Attestation] session ready token={}",
            flow::mask(&session.token)
        );
        {
            let mut shared = lock(&self.inner.shared);
            shared.session = Some(session.clone());
            shared.last_failure = None;
            shared.state = State::Idle;
        }
        // No receivers just means nobody is waiting.
        let _ = self.inner.refreshed.send(session);
    }

    fn report(
        &self,
        cycle_id: &str,
        attempt: u32,
        trigger: Trigger,
        duration_ms: u64,
        timings: Timings,
        outcome: Result<i64, Failure>,
    ) {
        let observer = lock(&self.inner.observer).clone();
        if let Some(observer) = observer {
            observer(&AttemptReport {
                cycle_id: cycle_id.to_owned(),
                attempt,
                trigger,
                duration_ms,
                timings,
                outcome,
            });
        }
    }

    fn bff_url(&self) -> String {
        let resolved = urls::attestation_bff();
        if self.inner.production {
            if resolved != PROD_ATTESTATION_BFF {
                tracing::warn!("[Attestation] production build pins {PROD_ATTESTATION_BFF}, ignoring {resolved}");
            }
            return PROD_ATTESTATION_BFF.to_owned();
        }
        resolved
    }
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Android => "android",
        Platform::Ios => "ios",
        Platform::Desktop => "desktop",
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
