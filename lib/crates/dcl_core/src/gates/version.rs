//! The update gate: the running build against the minimum and recommended versions the
//! mobile-bff serves for the platform.

use std::time::Duration;

use serde_json::Value;

use super::fetch::Fetch;
use crate::Platform;

/// The check races the network against this and lets the user in when it loses.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// How long "Later" silences a recommended update.
pub const SNOOZE_SECONDS: i64 = 24 * 60 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum UpdateDecision {
    /// Up to date, snoozed, or the check could not complete.
    Proceed,
    /// A newer build is recommended; the user may dismiss it.
    Optional,
    /// The build is below the minimum; the app cannot be used.
    Required,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct VersionGate {
    pub decision: UpdateDecision,
    /// The running build as `major*100000 + minor*100 + patch`; -1 when it cannot be read.
    pub current: i64,
    pub minimal_required: i64,
    pub recommended: i64,
}

impl VersionGate {
    fn proceed(current: i64) -> Self {
        Self {
            decision: UpdateDecision::Proceed,
            current,
            minimal_required: 0,
            recommended: 0,
        }
    }
}

/// `major.minor.patch[-suffix]` as `major*100000 + minor*100 + patch`, minor and patch clamped to
/// 99, the encoding the server uses for `minimalRequiredVersionNumber`. -1 when unparseable.
pub fn parse_version_number(full: &str) -> i64 {
    let semver = full.split('-').next().unwrap_or_default();
    let parts: Vec<&str> = semver.split('.').collect();
    if parts.len() < 2 {
        return -1;
    }
    // Lenient like the engine's `int()`: text that is not a number reads as 0.
    let number = |text: &str| text.trim().parse::<i64>().unwrap_or(0);
    let major = number(parts[0]);
    let minor = number(parts[1]).clamp(0, 99);
    let patch = parts.get(2).map_or(0, |p| number(p)).clamp(0, 99);
    major * 100_000 + minor * 100 + patch
}

fn platform_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Ios => "ios",
        _ => "android",
    }
}

/// The two thresholds of `{"ok": true, "data": {"<platform>": {...}}}`; `None` on any other shape.
fn thresholds(response: &Value, platform: Platform) -> Option<(i64, i64)> {
    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let entry = response.pointer("/data")?.get(platform_key(platform));
    let number = |key: &str| {
        entry
            .and_then(|e| e.get(key))
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as i64
    };
    Some((
        number("minimalRequiredVersionNumber"),
        number("recommendedVersionNumber"),
    ))
}

/// The pure decision. `snoozed_until` and `now` are unix seconds.
pub fn decide(
    current: i64,
    minimal_required: i64,
    recommended: i64,
    snoozed_until: i64,
    now: i64,
) -> UpdateDecision {
    if current < minimal_required {
        UpdateDecision::Required
    } else if current < recommended && now >= snoozed_until {
        UpdateDecision::Optional
    } else {
        UpdateDecision::Proceed
    }
}

/// Runs the check once. Fail-open: a timeout, a network error or a malformed answer lets the
/// user in, with no retry.
pub async fn check(
    fetch: &dyn Fetch,
    url: &str,
    app_version: &str,
    platform: Platform,
    snoozed_until: i64,
    now: i64,
) -> VersionGate {
    let current = parse_version_number(app_version);
    if current < 0 {
        return VersionGate::proceed(current);
    }
    let response = match tokio::time::timeout(TIMEOUT, fetch.get_json(url)).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            tracing::warn!("[VersionGate] fetch failed (proceeding): {e}");
            return VersionGate::proceed(current);
        }
        Err(_) => {
            tracing::warn!("[VersionGate] timeout (proceeding)");
            return VersionGate::proceed(current);
        }
    };
    let Some((minimal_required, recommended)) = thresholds(&response, platform) else {
        return VersionGate::proceed(current);
    };
    VersionGate {
        decision: decide(current, minimal_required, recommended, snoozed_until, now),
        current,
        minimal_required,
        recommended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::test_support::FakeFetch;
    use serde_json::json;

    #[test]
    fn version_numbers_follow_the_servers_encoding() {
        for (text, expected) in [
            ("0.64.3-abc1234-dev", 6403),
            ("1.0.0", 100_000),
            ("1.15.0", 101_500),
            ("2.0", 200_000),
            ("1.200.300", 109_999),
            ("1", -1),
            ("", -1),
            ("x.y.z", 0),
        ] {
            assert_eq!(parse_version_number(text), expected, "{text}");
        }
    }

    #[test]
    fn decisions_cover_required_optional_snoozed_and_current() {
        // (current, minimal, recommended, snoozed_until, now) -> decision
        let table = [
            (100, 200, 300, 0, 10, UpdateDecision::Required),
            (200, 200, 300, 0, 10, UpdateDecision::Optional),
            (200, 200, 300, 50, 10, UpdateDecision::Proceed),
            (200, 200, 300, 10, 10, UpdateDecision::Optional),
            (300, 200, 300, 0, 10, UpdateDecision::Proceed),
            (400, 200, 300, 0, 10, UpdateDecision::Proceed),
            (100, 200, 300, 999, 10, UpdateDecision::Required),
        ];
        for (current, min, rec, snooze, now, expected) in table {
            assert_eq!(decide(current, min, rec, snooze, now), expected);
        }
    }

    fn served(ios: (i64, i64), android: (i64, i64)) -> Value {
        json!({"ok": true, "data": {
            "ios": {"minimalRequiredVersionNumber": ios.0, "recommendedVersionNumber": ios.1},
            "android": {"minimalRequiredVersionNumber": android.0, "recommendedVersionNumber": android.1},
        }})
    }

    #[tokio::test]
    async fn reads_the_platforms_own_thresholds() {
        let fetch = FakeFetch::with_get(Ok(served((200_000, 0), (100_000, 0))));
        let url = "https://bff/app-versions/v2";
        let ios = check(&fetch, url, "1.15.0", Platform::Ios, 0, 0).await;
        assert_eq!(ios.decision, UpdateDecision::Required);
        assert_eq!((ios.current, ios.minimal_required), (101_500, 200_000));
        let android = check(&fetch, url, "1.15.0", Platform::Android, 0, 0).await;
        assert_eq!(android.decision, UpdateDecision::Proceed);
        assert_eq!(fetch.gets(), vec![url.to_string(), url.to_string()]);
    }

    #[tokio::test]
    async fn an_optional_update_waits_out_a_snooze() {
        let fetch = FakeFetch::with_get(Ok(served((0, 0), (0, 200_000))));
        let at = |snoozed_until, now| {
            check(&fetch, "u", "1.15.0", Platform::Android, snoozed_until, now)
        };
        assert_eq!(at(0, 1_000).await.decision, UpdateDecision::Optional);
        assert_eq!(at(2_000, 1_000).await.decision, UpdateDecision::Proceed);
    }

    #[tokio::test]
    async fn every_failure_lets_the_user_in() {
        for response in [
            Err("offline".to_string()),
            Ok(json!({"ok": false})),
            Ok(json!({"ok": true})),
            Ok(json!([])),
        ] {
            let fetch = FakeFetch::with_get(response);
            let gate = check(&fetch, "u", "1.15.0", Platform::Ios, 0, 0).await;
            assert_eq!(gate.decision, UpdateDecision::Proceed);
        }
        let gate = check(&FakeFetch::default(), "u", "oops", Platform::Ios, 0, 0).await;
        assert_eq!((gate.decision, gate.current), (UpdateDecision::Proceed, -1));
    }

    #[tokio::test(start_paused = true)]
    async fn a_slow_server_loses_to_the_timeout() {
        let fetch = FakeFetch::hanging();
        let gate = check(&fetch, "u", "1.0.0", Platform::Ios, 0, 0).await;
        assert_eq!(gate.decision, UpdateDecision::Proceed);
    }
}
