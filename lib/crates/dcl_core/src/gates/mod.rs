//! The boot gates, the URLs and the destination rules the shell asks the core for (D-5): typed
//! records, no JSON across the boundary, hosts never spelled by the shell.
//!
//! Persisted counters live in `gates.json` (see `state`); the network goes through `Fetch`.

pub mod campaign;
pub mod destination;
pub mod device;
pub mod fetch;
pub mod state;
pub mod urls;
pub mod version;

/// Bump when the terms change: everyone is asked again.
pub const TERMS_VERSION: u32 = 1;

#[cfg(test)]
mod test_support;

use std::{
    io,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Map, Value};
use tokio::sync::OnceCell;

use self::{
    campaign::CampaignResolution,
    destination::ResolvedDestination,
    device::{DeviceFacts, DeviceSupport, SupportModal, SupportStatus},
    fetch::Fetch,
    state::StateStore,
    version::{VersionGate, SNOOZE_SECONDS},
};
use crate::{urls as backend_urls, Platform};

fn system_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The gates of one process. The chipset lookup and the campaign map are fetched once per run: a
/// device's hardware does not change mid-session, and a campaign is retired server side.
pub struct Gates {
    fetch: Arc<dyn Fetch>,
    store: StateStore,
    platform: Platform,
    app_version: String,
    now: fn() -> i64,
    soc_status: OnceCell<SupportStatus>,
    campaigns: OnceCell<Option<Map<String, Value>>>,
    /// The last device-support verdict, which the update gate defers to.
    device_status: Mutex<Option<SupportStatus>>,
}

impl Gates {
    pub fn new(
        fetch: Arc<dyn Fetch>,
        store: StateStore,
        platform: Platform,
        app_version: &str,
    ) -> Self {
        Self::with_clock(fetch, store, platform, app_version, system_now)
    }

    pub fn with_clock(
        fetch: Arc<dyn Fetch>,
        store: StateStore,
        platform: Platform,
        app_version: &str,
        now: fn() -> i64,
    ) -> Self {
        Self {
            fetch,
            store,
            platform,
            app_version: app_version.to_string(),
            now,
            soc_status: OnceCell::new(),
            campaigns: OnceCell::new(),
            device_status: Mutex::new(None),
        }
    }

    /// The update gate. An end-of-support device never checks for an update, let alone sees the
    /// prompt for one it cannot install: once `device_support` has found it excluded, this lets
    /// it through without asking the server.
    pub async fn version_gate(&self) -> VersionGate {
        let current = version::parse_version_number(&self.app_version);
        if self.device_status() == Some(SupportStatus::EndOfSupport) {
            return VersionGate {
                decision: version::UpdateDecision::Proceed,
                current,
                minimal_required: 0,
                recommended: 0,
            };
        }
        version::check(
            self.fetch.as_ref(),
            &backend_urls::app_versions(),
            &self.app_version,
            self.platform,
            self.store.get().version_gate_snooze_until,
            (self.now)(),
        )
        .await
    }

    /// The terms version the player accepted; 0 when they have not.
    pub fn terms_accepted_version(&self) -> u32 {
        self.store.get().terms_accepted_version
    }

    pub fn accept_terms(&self, version: u32) -> io::Result<()> {
        self.store.update(|s| state::GatesState {
            terms_accepted_version: version,
            ..s.clone()
        })
    }

    /// "Later" on the optional update: silent for a day.
    pub fn snooze_version_gate(&self) -> io::Result<()> {
        let until = (self.now)() + SNOOZE_SECONDS;
        self.store.update(|s| state::GatesState {
            version_gate_snooze_until: until,
            ..s.clone()
        })
    }

    /// Decides the device's support and which modal, if any, is due. Stamps the anchor of the
    /// end-of-support schedule the first time the device is found excluded.
    pub async fn device_support(&self, facts: &DeviceFacts) -> io::Result<DeviceSupport> {
        let soc = device::normalized_soc(facts);
        let status = match device::instant_status(self.platform, facts) {
            Some(status) => status,
            None => {
                let url = backend_urls::device_support();
                *self
                    .soc_status
                    .get_or_init(|| device::fetch_soc_status(self.fetch.as_ref(), &url, &soc))
                    .await
            }
        };
        *self.device_status.lock().unwrap_or_else(|e| e.into_inner()) = Some(status);

        let now = (self.now)();
        self.store
            .update(|s| device::after_detected(s, status, now))?;
        Ok(DeviceSupport {
            status,
            verdict: device::report_verdict(self.platform, status, facts),
            modal: device::modal_due(status, &self.store.get(), now),
            soc,
        })
    }

    /// The host showed `modal`: record it so the cadence moves on.
    pub fn device_support_modal_shown(&self, modal: SupportModal) -> io::Result<()> {
        let now = (self.now)();
        self.store
            .update(|s| device::after_modal_shown(s, modal, now))
    }

    fn device_status(&self) -> Option<SupportStatus> {
        *self.device_status.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Remembers the campaign token of a link or of the install attribution. The first one wins
    /// and is never replaced; a malformed one is ignored. `true` when it was stored.
    pub fn capture_campaign_token(&self, token: &str) -> io::Result<bool> {
        let token = token.trim();
        let now = (self.now)();
        if !campaign::is_valid_token(token) || !self.store.get().campaign_token.is_empty() {
            return Ok(false);
        }
        self.store.update(|s| state::GatesState {
            campaign_token: token.to_string(),
            campaign_token_captured_at: now,
            ..s.clone()
        })?;
        Ok(true)
    }

    /// Resolves the stored token into the campaign that should drive this launch. Waiting for the
    /// Android install attribution to settle before asking is the host's job.
    pub async fn campaign_resolution(&self) -> CampaignResolution {
        let state = self.store.get();
        if let Err(reason) = campaign::precheck(&state, (self.now)()) {
            return CampaignResolution {
                token: state.campaign_token,
                campaign: None,
                fallback: Some(reason),
            };
        }
        // Lazy: fetching on every boot would put every install on the campaigns endpoint.
        let url = backend_urls::campaigns();
        let map = self
            .campaigns
            .get_or_init(|| campaign::fetch_map(self.fetch.as_ref(), &url))
            .await;
        campaign::resolve(map.as_ref(), &state)
    }

    /// One campaign per install: once a launch booted into the target, it never runs again.
    pub fn consume_campaign(&self) -> io::Result<()> {
        if self.store.get().campaign_consumed {
            return Ok(());
        }
        self.store.update(|s| state::GatesState {
            campaign_consumed: true,
            ..s.clone()
        })
    }

    /// `address` is the signed-in wallet, empty when signed out.
    pub async fn resolve_destination(
        &self,
        input: &str,
        secret: Option<&str>,
        address: &str,
    ) -> ResolvedDestination {
        destination::resolve(self.fetch.as_ref(), address, input, secret).await
    }
}

#[cfg(test)]
mod tests;
