use std::collections::HashMap;

use godot::{
    classes::{file_access::ModeFlags, FileAccess},
    prelude::*,
};

use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;

use super::data_definition::{SegmentEvent, SegmentEventInstallAttribution};

const FLAG_PATH: &str = "user://install_referrer_sent";

/// Tracks install attribution from the Google Play Install Referrer API.
/// Self-contained: handles fetching, parsing, persistence, and event creation.
pub struct InstallReferrer {
    requested: bool,
    done: bool,
}

impl InstallReferrer {
    /// Create and start the referrer fetch if needed.
    /// Returns `None` if already sent in a previous session.
    pub fn start() -> Option<Self> {
        if FileAccess::file_exists(&GString::from(FLAG_PATH)) {
            tracing::info!("Install referrer already sent in a previous session, skipping");
            return None;
        }

        // Trigger the async fetch on the Android side
        DclAndroidPlugin::get_install_referrer_internal();
        tracing::info!("Install referrer fetch requested");

        Some(Self {
            requested: true,
            done: false,
        })
    }

    /// Poll for the referrer result. Returns a SegmentEvent when ready.
    /// Returns `None` while still pending or after already completed.
    pub fn poll(&mut self) -> Option<SegmentEvent> {
        if self.done || !self.requested {
            return None;
        }

        let dict = DclAndroidPlugin::get_install_referrer_internal()?;

        let status = dict
            .get("status")
            .and_then(|v| v.try_to::<GString>().ok())
            .unwrap_or_default()
            .to_string();

        if status == "pending" {
            return None;
        }

        self.done = true;

        if status != "ok" {
            let error = dict
                .get("error")
                .and_then(|v| v.try_to::<GString>().ok())
                .unwrap_or_default()
                .to_string();
            tracing::warn!("Install referrer not available: {status} - {error}");
            return None;
        }

        let referrer = dict
            .get("referrer")
            .and_then(|v| v.try_to::<GString>().ok())
            .unwrap_or_default()
            .to_string();

        let click_timestamp = dict
            .get("click_timestamp")
            .and_then(|v| v.try_to::<i64>().ok())
            .unwrap_or(0);

        let install_timestamp = dict
            .get("install_timestamp")
            .and_then(|v| v.try_to::<i64>().ok())
            .unwrap_or(0);

        let google_play_instant = dict
            .get("google_play_instant")
            .and_then(|v| v.try_to::<bool>().ok())
            .unwrap_or(false);

        let utm = parse_utm_params(&referrer);

        tracing::info!(
            "Install attribution tracked: source={:?}, campaign={:?}, referrer={referrer}",
            utm.get("utm_source"),
            utm.get("utm_campaign"),
        );

        // Persist so we never send again
        if let Some(mut fa) = FileAccess::open(&GString::from(FLAG_PATH), ModeFlags::WRITE) {
            fa.store_string(&GString::from("1"));
        }

        Some(SegmentEvent::InstallAttribution(
            SegmentEventInstallAttribution {
                referrer,
                utm_source: utm.get("utm_source").cloned(),
                utm_medium: utm.get("utm_medium").cloned(),
                utm_campaign: utm.get("utm_campaign").cloned(),
                utm_content: utm.get("utm_content").cloned(),
                utm_term: utm.get("utm_term").cloned(),
                click_timestamp,
                install_timestamp,
                google_play_instant,
            },
        ))
    }
}

fn parse_utm_params(referrer: &str) -> HashMap<String, String> {
    referrer
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            if key.starts_with("utm_") {
                Some((key.to_string(), percent_decode(value)))
            } else {
                None
            }
        })
        .collect()
}

fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut bytes = input.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().and_then(|c| (c as char).to_digit(16));
            let lo = bytes.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(h), Some(l)) = (hi, lo) {
                result.push((h * 16 + l) as u8 as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}
