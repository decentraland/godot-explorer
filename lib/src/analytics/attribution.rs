use std::time::{Duration, Instant};

use godot::prelude::*;

use crate::godot_classes::{dcl_android_plugin::DclAndroidPlugin, dcl_ios_plugin::DclIosPlugin};

use super::{
    data_definition::{SegmentEvent, SegmentEventAttributionInit},
    install_attribution::{extract_token, get_string},
};

/// Tenjin SDK key. Empty disables the whole integration.
const TENJIN_SDK_KEY: &str = "";

/// Bound on waiting for `getAttributionInfo`: an organic install can stay pending, and the
/// campaign watcher in `campaigns.gd` waits on this.
const ATTRIBUTION_WAIT: Duration = Duration::from_secs(25);

/// What the settled attribution gave the app.
pub struct AttributionResult {
    pub event: SegmentEvent,
    pub campaign_token: Option<String>,
}

/// Sends the funnel to Tenjin and reads back the install attribution. Safe to call on any
/// platform: without the plugin, or without a key, every method does nothing.
#[derive(Default)]
pub struct Attribution {
    started: bool,
    attribution_wanted: bool,
    deadline: Option<Instant>,
}

impl Attribution {
    #[allow(clippy::const_is_empty)]
    fn enabled() -> bool {
        !TENJIN_SDK_KEY.is_empty()
            && (DclAndroidPlugin::is_available() || DclIosPlugin::is_available())
    }

    /// Starts the SDK once the EULA is accepted. Runs every launch: the SDK counts sessions.
    pub fn start(&mut self, user_id: &str) {
        if self.started || !Self::enabled() {
            return;
        }
        let key = GString::from(TENJIN_SDK_KEY);
        let user_id = GString::from(user_id);
        let ok = if DclAndroidPlugin::is_available() {
            DclAndroidPlugin::tenjin_start(key, user_id)
        } else {
            DclIosPlugin::tenjin_start(key, user_id)
        };
        if !ok {
            tracing::warn!("[attribution] Tenjin failed to start");
            return;
        }
        self.started = true;
        if self.attribution_wanted {
            self.deadline = Some(Instant::now() + ATTRIBUTION_WAIT);
        }
    }

    /// Whether the event was handed to the SDK; the one-shot callers persist their flag on it.
    pub fn track_event(&self, name: &str) -> bool {
        if !self.started {
            return false;
        }
        let name = GString::from(name);
        if DclAndroidPlugin::is_available() {
            DclAndroidPlugin::tenjin_event(name, 0)
        } else {
            DclIosPlugin::tenjin_event(name, 0)
        }
    }

    /// Asks for the install attribution, once per install. Waits for `start` if the EULA is
    /// not accepted yet.
    pub fn request_attribution(&mut self) {
        if self.attribution_wanted || !Self::enabled() {
            return;
        }
        self.attribution_wanted = true;
        if self.started {
            self.deadline = Some(Instant::now() + ATTRIBUTION_WAIT);
        }
    }

    /// Whether the attribution is still unresolved, so waiters keep waiting.
    pub fn is_pending(&self) -> bool {
        self.attribution_wanted
    }

    /// Polls the SDK; returns the result once it settles, `None` while pending.
    pub fn poll(&mut self) -> Option<AttributionResult> {
        let deadline = self.deadline?;
        let info = if DclAndroidPlugin::is_available() {
            DclAndroidPlugin::tenjin_get_attribution_internal()
        } else {
            DclIosPlugin::tenjin_get_attribution_internal()
        }?;

        let status = get_string(&info, "status");
        if status == "pending" && Instant::now() < deadline {
            return None;
        }

        self.attribution_wanted = false;
        self.deadline = None;
        if status != "ok" {
            tracing::warn!("[attribution] Tenjin attribution unavailable (status='{status}')");
            return None;
        }

        let deferred_deeplink_url = get_string(&info, "deferred_deeplink_url");
        let campaign_token = extract_token(&deferred_deeplink_url);
        let non_empty = |key: &str| Some(get_string(&info, key)).filter(|v| !v.is_empty());
        let event = SegmentEvent::AttributionInit(SegmentEventAttributionInit {
            ad_network: non_empty("ad_network"),
            campaign_id: non_empty("campaign_id"),
            campaign_name: non_empty("campaign_name"),
            click_id: non_empty("click_id"),
            creative_name: non_empty("creative_name"),
            deferred_deeplink_url: Some(deferred_deeplink_url).filter(|v| !v.is_empty()),
            analytics_installation_id: non_empty("analytics_installation_id"),
            campaign_token: campaign_token.clone(),
        });
        Some(AttributionResult {
            event,
            campaign_token,
        })
    }
}
