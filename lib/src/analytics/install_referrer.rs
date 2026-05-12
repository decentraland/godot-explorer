use std::collections::HashMap;

use godot::prelude::*;

use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;

use super::data_definition::{SegmentEvent, SegmentEventInstallAttribution};

/// Tracks install attribution from the Google Play Install Referrer API.
/// Self-contained: handles fetching, parsing, and event creation.
/// Persistence (fire-once-per-install) is handled by GDScript via config flag.
pub struct InstallReferrer {
    done: bool,
}

impl InstallReferrer {
    /// Create and start the referrer fetch.
    pub fn start() -> Self {
        // Trigger the async fetch on the Android side
        let _ = DclAndroidPlugin::get_install_referrer_internal();
        Self { done: false }
    }

    /// Poll for the referrer result. Returns a SegmentEvent when ready.
    /// Returns `None` while still pending or after already completed.
    pub fn poll(&mut self) -> Option<SegmentEvent> {
        if self.done {
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
            tracing::warn!("Install referrer not available: status='{status}' error='{error}'");
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
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        if b == b'+' {
            out.push(b' ');
        } else {
            out.push(b);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
