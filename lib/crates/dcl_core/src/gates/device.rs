//! Device support: whether this phone is end-of-support (excluded from the store build, #2936) or
//! below the minimum spec (#2935), and when the matching modal is due.
//!
//! Android decides by RAM, instantly, or by chipset through the mobile-bff's per-SoC
//! `/device-support?soc=` decision. iOS has its own rule on the hardware model.

use std::time::Duration;

use serde_json::Value;

use super::{fetch::Fetch, state::GatesState};
use crate::Platform;

pub const MIN_RAM_MB: i32 = 4096;
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// End-of-support modal re-show schedule, in days since first detected: the first session, then
/// day 5, 10 and 30, then every `REPEAT_DAYS` while the device stays excluded.
const SCHEDULE_DAYS: [i64; 4] = [0, 5, 10, 30];
const REPEAT_DAYS: i64 = 30;
const DAY_SECONDS: i64 = 86_400;

/// What the host can read off the device; empty or negative where it cannot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct DeviceFacts {
    /// Total RAM in MB, -1 when unknown.
    pub total_ram_mb: i32,
    /// Hardware model: iOS `hw.machine` (`iPhone14,5`); informational on Android.
    pub model: String,
    /// Android `Build.SOC_MODEL` (API 31+).
    pub soc_model: String,
    /// Android `ro.board.platform`, the chipset fallback below API 31.
    pub board_platform: String,
    /// Android `Build.HARDWARE`, the second fallback.
    pub hardware: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SupportStatus {
    Supported,
    BelowMinspec,
    EndOfSupport,
}

/// The verdict the launch-time system report carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ReportVerdict {
    Supported,
    BelowMinspec,
    EndOfSupport,
    /// Neither platform check runs here.
    NotChecked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SupportModal {
    EndOfSupport,
    BelowMinspec,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct DeviceSupport {
    /// The Android rule's result; always `Supported` elsewhere.
    pub status: SupportStatus,
    pub verdict: ReportVerdict,
    /// The modal to show this session, per the persisted cadence; `None` when none is due.
    pub modal: Option<SupportModal>,
    /// The chipset id the server was asked about; empty when none.
    pub soc: String,
}

/// The ids the server knows have no vendor prefix.
fn normalize_soc(raw: &str) -> String {
    let mut soc = raw.trim().to_uppercase();
    for prefix in ["QUALCOMM ", "MEDIATEK ", "QTI "] {
        if let Some(rest) = soc.strip_prefix(prefix) {
            soc = rest.to_string();
        }
    }
    soc
}

/// The chipset id this device is looked up by: the first non-empty of SoC model, board platform
/// and hardware.
pub fn normalized_soc(facts: &DeviceFacts) -> String {
    [&facts.soc_model, &facts.board_platform, &facts.hardware]
        .into_iter()
        .map(|raw| normalize_soc(raw))
        .find(|id| !id.is_empty())
        .unwrap_or_default()
}

pub fn is_ram_excluded(facts: &DeviceFacts) -> bool {
    facts.total_ram_mb >= 0 && facts.total_ram_mb < MIN_RAM_MB
}

/// iPhone 12 and older (`iPhone<major>,<minor>` with major <= 13). iPads, iPods, simulators and
/// unrecognized or future models are not low spec.
pub fn is_low_spec_iphone(model: &str) -> bool {
    let Some(rest) = model.strip_prefix("iPhone") else {
        return false;
    };
    rest.split(',')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major <= 13)
}

/// `{"ok": true, "data": {"decision": "exclude" | "below-minspec" | "keep"}}`; anything else is
/// `Supported`.
pub fn parse_decision(response: &Value) -> SupportStatus {
    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        return SupportStatus::Supported;
    }
    match response.pointer("/data/decision").and_then(Value::as_str) {
        Some("exclude") => SupportStatus::EndOfSupport,
        Some("below-minspec") => SupportStatus::BelowMinspec,
        _ => SupportStatus::Supported,
    }
}

/// Asks the server about this chipset. Fail-open to `Supported`.
pub async fn fetch_soc_status(fetch: &dyn Fetch, base_url: &str, soc: &str) -> SupportStatus {
    if soc.is_empty() {
        return SupportStatus::Supported;
    }
    let url = format!("{base_url}?soc={}", crate::realm::uri_encode(soc));
    match tokio::time::timeout(TIMEOUT, fetch.get_json(&url)).await {
        Ok(Ok(response)) => parse_decision(&response),
        Ok(Err(e)) => {
            tracing::warn!("[DeviceSupport] fetch failed (fail-open): {e}");
            SupportStatus::Supported
        }
        Err(_) => {
            tracing::warn!("[DeviceSupport] timeout (fail-open)");
            SupportStatus::Supported
        }
    }
}

/// The platform's rule except the chipset lookup, `None` where the answer needs that lookup.
pub fn instant_status(platform: Platform, facts: &DeviceFacts) -> Option<SupportStatus> {
    match platform {
        Platform::Android if is_ram_excluded(facts) => Some(SupportStatus::EndOfSupport),
        Platform::Android => None,
        _ => Some(SupportStatus::Supported),
    }
}

pub fn report_verdict(
    platform: Platform,
    status: SupportStatus,
    facts: &DeviceFacts,
) -> ReportVerdict {
    match platform {
        Platform::Ios if is_low_spec_iphone(&facts.model) => ReportVerdict::BelowMinspec,
        Platform::Ios => ReportVerdict::Supported,
        Platform::Desktop => ReportVerdict::NotChecked,
        Platform::Android => match status {
            SupportStatus::EndOfSupport => ReportVerdict::EndOfSupport,
            SupportStatus::BelowMinspec => ReportVerdict::BelowMinspec,
            SupportStatus::Supported => ReportVerdict::Supported,
        },
    }
}

fn days_elapsed(first_detected: i64, now: i64) -> i64 {
    (now - first_detected) / DAY_SECONDS
}

fn next_due_day(shown_count: u32) -> i64 {
    let index = shown_count as usize;
    if index < SCHEDULE_DAYS.len() {
        return SCHEDULE_DAYS[index];
    }
    let extra = (index - (SCHEDULE_DAYS.len() - 1)) as i64;
    SCHEDULE_DAYS[SCHEDULE_DAYS.len() - 1] + REPEAT_DAYS * extra
}

/// Whether the end-of-support modal is due again. An unstamped anchor (<= 0) is "not due": with
/// nothing to measure days against, staying quiet is the safe direction.
pub fn end_of_support_modal_due(shown_count: u32, first_detected: i64, now: i64) -> bool {
    first_detected > 0 && days_elapsed(first_detected, now) >= next_due_day(shown_count)
}

/// The shown count to persist right after showing the modal: past every tier the calendar gap
/// already satisfies, so a device back after a long absence catches up in the one show.
pub fn catch_up_shown_count(first_detected: i64, now: i64) -> u32 {
    let elapsed = days_elapsed(first_detected, now);
    let mut count = 0;
    while next_due_day(count) <= elapsed {
        count += 1;
    }
    count
}

/// The modal due this session; end-of-support wins over below-minspec on a device matching both.
pub fn modal_due(status: SupportStatus, state: &GatesState, now: i64) -> Option<SupportModal> {
    match status {
        SupportStatus::EndOfSupport
            if end_of_support_modal_due(
                state.end_of_device_support_modal_shown_count,
                state.end_of_device_support_first_detected_unix,
                now,
            ) =>
        {
            Some(SupportModal::EndOfSupport)
        }
        SupportStatus::BelowMinspec if !state.below_minspec_modal_shown => {
            Some(SupportModal::BelowMinspec)
        }
        _ => None,
    }
}

/// The state after the host showed `modal`.
pub fn after_modal_shown(state: &GatesState, modal: SupportModal, now: i64) -> GatesState {
    match modal {
        SupportModal::EndOfSupport => GatesState {
            end_of_device_support_modal_shown_count: catch_up_shown_count(
                state.end_of_device_support_first_detected_unix,
                now,
            ),
            ..state.clone()
        },
        SupportModal::BelowMinspec => GatesState {
            below_minspec_modal_shown: true,
            ..state.clone()
        },
    }
}

/// The state once the device is known to be end-of-support: the anchor of the schedule is stamped
/// the first time, and never moves.
pub fn after_detected(state: &GatesState, status: SupportStatus, now: i64) -> GatesState {
    if status == SupportStatus::EndOfSupport && state.end_of_device_support_first_detected_unix == 0
    {
        return GatesState {
            end_of_device_support_first_detected_unix: now,
            ..state.clone()
        };
    }
    state.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::test_support::FakeFetch;
    use serde_json::json;

    const DAY: i64 = DAY_SECONDS;

    fn facts(ram: i32) -> DeviceFacts {
        DeviceFacts {
            total_ram_mb: ram,
            ..Default::default()
        }
    }

    #[test]
    fn ram_below_the_floor_is_end_of_support_without_a_lookup() {
        for (ram, expected) in [
            (2048, Some(SupportStatus::EndOfSupport)),
            (4095, Some(SupportStatus::EndOfSupport)),
            (4096, None),
            (8192, None),
            (-1, None),
        ] {
            assert_eq!(
                instant_status(Platform::Android, &facts(ram)),
                expected,
                "{ram}"
            );
        }
        assert_eq!(
            instant_status(Platform::Ios, &facts(1)),
            Some(SupportStatus::Supported)
        );
        assert_eq!(
            instant_status(Platform::Desktop, &facts(1)),
            Some(SupportStatus::Supported)
        );
    }

    #[test]
    fn the_chipset_is_the_first_filled_id_without_its_vendor_prefix() {
        let soc = |a: &str, b: &str, c: &str| {
            normalized_soc(&DeviceFacts {
                soc_model: a.into(),
                board_platform: b.into(),
                hardware: c.into(),
                ..Default::default()
            })
        };
        assert_eq!(soc("sm6125", "x", "y"), "SM6125");
        assert_eq!(soc("  ", "Qualcomm kona", "y"), "KONA");
        assert_eq!(soc("", "", "mediatek MT6785"), "MT6785");
        assert_eq!(soc("qti sm8250", "", "exynos990"), "SM8250");
        assert_eq!(soc("", "", ""), "");
    }

    #[test]
    fn low_spec_iphones_are_the_twelve_and_older() {
        for (model, low) in [
            ("iPhone13,2", true),
            ("iPhone12,1", true),
            ("iPhone14,5", false),
            ("iPhone15,2", false),
            ("iPad13,1", false),
            ("iPod9,1", false),
            ("x86_64", false),
            ("iPhone", false),
            ("iPhoneX,1", false),
            ("", false),
        ] {
            assert_eq!(is_low_spec_iphone(model), low, "{model}");
        }
    }

    #[test]
    fn decisions_map_and_anything_else_is_supported() {
        let decide =
            |decision: Value| parse_decision(&json!({"ok": true, "data": {"decision": decision}}));
        assert_eq!(decide(json!("exclude")), SupportStatus::EndOfSupport);
        assert_eq!(decide(json!("below-minspec")), SupportStatus::BelowMinspec);
        assert_eq!(decide(json!("keep")), SupportStatus::Supported);
        assert_eq!(decide(json!("new-thing")), SupportStatus::Supported);
        assert_eq!(decide(json!(3)), SupportStatus::Supported);
        assert_eq!(
            parse_decision(&json!({"ok": false})),
            SupportStatus::Supported
        );
        assert_eq!(
            parse_decision(&json!({"ok": true})),
            SupportStatus::Supported
        );
        assert_eq!(parse_decision(&json!(null)), SupportStatus::Supported);
    }

    #[tokio::test]
    async fn the_lookup_asks_for_the_encoded_chipset_and_fails_open() {
        let fetch = FakeFetch::with_get(Ok(json!({"ok": true, "data": {"decision": "exclude"}})));
        assert_eq!(
            fetch_soc_status(&fetch, "https://bff/device-support", "SM 6125").await,
            SupportStatus::EndOfSupport
        );
        assert_eq!(
            fetch.gets(),
            vec!["https://bff/device-support?soc=SM%206125".to_string()]
        );
        assert_eq!(
            fetch_soc_status(&fetch, "u", "").await,
            SupportStatus::Supported
        );
        let offline = FakeFetch::with_get(Err("offline".into()));
        assert_eq!(
            fetch_soc_status(&offline, "u", "X").await,
            SupportStatus::Supported
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_slow_lookup_fails_open() {
        assert_eq!(
            fetch_soc_status(&FakeFetch::hanging(), "u", "X").await,
            SupportStatus::Supported
        );
    }

    #[test]
    fn the_end_of_support_schedule_is_first_session_then_5_10_30_and_every_30() {
        let first = 1_000_000;
        // (shown_count, days since first detection, due)
        let table = [
            (0, 0, true),
            (1, 4, false),
            (1, 5, true),
            (2, 9, false),
            (2, 10, true),
            (3, 29, false),
            (3, 30, true),
            (4, 59, false),
            (4, 60, true),
            (5, 89, false),
            (5, 90, true),
        ];
        for (count, days, due) in table {
            assert_eq!(
                end_of_support_modal_due(count, first, first + days * DAY),
                due,
                "count {count} at day {days}"
            );
        }
    }

    #[test]
    fn an_unstamped_anchor_is_never_due() {
        assert!(!end_of_support_modal_due(0, 0, 10 * DAY));
        assert!(!end_of_support_modal_due(0, -5, 10 * DAY));
    }

    #[test]
    fn catching_up_skips_every_tier_the_gap_already_covers() {
        let first = 1_000_000;
        for (days, count) in [
            (0, 1),
            (4, 1),
            (5, 2),
            (9, 2),
            (10, 3),
            (29, 3),
            (30, 4),
            (100, 6),
            (120, 7),
        ] {
            assert_eq!(
                catch_up_shown_count(first, first + days * DAY),
                count,
                "day {days}"
            );
        }
        // After catching up the next tier is not due yet.
        let now = first + 100 * DAY;
        assert!(!end_of_support_modal_due(
            catch_up_shown_count(first, now),
            first,
            now
        ));
    }

    #[test]
    fn end_of_support_wins_and_below_minspec_shows_once() {
        let first = 5_000;
        let state = GatesState {
            end_of_device_support_first_detected_unix: first,
            ..Default::default()
        };
        assert_eq!(
            modal_due(SupportStatus::EndOfSupport, &state, first),
            Some(SupportModal::EndOfSupport)
        );
        let shown = after_modal_shown(&state, SupportModal::EndOfSupport, first);
        assert_eq!(shown.end_of_device_support_modal_shown_count, 1);
        assert_eq!(
            modal_due(SupportStatus::EndOfSupport, &shown, first + DAY),
            None
        );
        assert_eq!(
            modal_due(SupportStatus::EndOfSupport, &shown, first + 5 * DAY),
            Some(SupportModal::EndOfSupport)
        );

        assert_eq!(
            modal_due(SupportStatus::BelowMinspec, &state, first),
            Some(SupportModal::BelowMinspec)
        );
        let seen = after_modal_shown(&state, SupportModal::BelowMinspec, first);
        assert!(seen.below_minspec_modal_shown);
        assert_eq!(
            modal_due(SupportStatus::BelowMinspec, &seen, first + 400 * DAY),
            None
        );
        assert_eq!(modal_due(SupportStatus::Supported, &state, first), None);
    }

    #[test]
    fn the_anchor_is_stamped_once_and_only_for_end_of_support() {
        let fresh = GatesState::default();
        let stamped = after_detected(&fresh, SupportStatus::EndOfSupport, 777);
        assert_eq!(stamped.end_of_device_support_first_detected_unix, 777);
        assert_eq!(
            after_detected(&stamped, SupportStatus::EndOfSupport, 999)
                .end_of_device_support_first_detected_unix,
            777
        );
        assert_eq!(
            after_detected(&fresh, SupportStatus::BelowMinspec, 5),
            fresh
        );
        assert_eq!(after_detected(&fresh, SupportStatus::Supported, 5), fresh);
    }

    #[test]
    fn the_report_verdict_follows_the_platform() {
        let iphone = |model: &str| DeviceFacts {
            model: model.into(),
            ..Default::default()
        };
        let v = |p, s, f: &DeviceFacts| report_verdict(p, s, f);
        assert_eq!(
            v(
                Platform::Ios,
                SupportStatus::Supported,
                &iphone("iPhone12,1")
            ),
            ReportVerdict::BelowMinspec
        );
        assert_eq!(
            v(
                Platform::Ios,
                SupportStatus::Supported,
                &iphone("iPhone15,2")
            ),
            ReportVerdict::Supported
        );
        assert_eq!(
            v(Platform::Desktop, SupportStatus::Supported, &iphone("")),
            ReportVerdict::NotChecked
        );
        assert_eq!(
            v(Platform::Android, SupportStatus::EndOfSupport, &iphone("")),
            ReportVerdict::EndOfSupport
        );
        assert_eq!(
            v(Platform::Android, SupportStatus::BelowMinspec, &iphone("")),
            ReportVerdict::BelowMinspec
        );
        assert_eq!(
            v(Platform::Android, SupportStatus::Supported, &iphone("")),
            ReportVerdict::Supported
        );
    }
}
