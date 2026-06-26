//! Store build number (Android `versionCode` / iOS `CFBundleVersion`).
//!
//! Scheme: `days_since(2020-01-01 UTC) * 100_000 + seconds_since_midnight_UTC`.
//!
//! Properties:
//! - **Monotonic** as long as the wall clock moves forward (newer build => bigger number),
//!   which is exactly what both stores require.
//! - **1-second resolution** => two builds collide only inside the same UTC second
//!   (effectively never, even with iOS PR builds going to TestFlight).
//! - Stays under the Android `versionCode` cap (2,100,000,000) until ~year 2077.
//! - **Readable**: leading digits are the day index, trailing 5 are the second-of-day
//!   (e.g. `236752200` => day `2367`, second `52200` = 14:30:00 UTC).
//!
//! This number is intentionally **opaque/meaningless** — it is NOT the marketing version
//! (that is the SemVer in `lib/Cargo.toml`). It is an extra coordinate, like the git sha
//! and the `-prod`/`-staging` env suffix.
//!
//! Resolution precedence: explicit `--build-number` > `DCL_BUILD_NUMBER` env > wall clock.
//! In CI a single step computes the value once and exports `DCL_BUILD_NUMBER`, so every
//! `cargo run -- export` invocation in the job stamps the *same* number across artifacts.

use chrono::Utc;

/// 2020-01-01T00:00:00Z as a Unix timestamp (seconds). Midnight-aligned, like the Unix epoch.
const EPOCH_2020: i64 = 1_577_836_800;
const SECONDS_PER_DAY: i64 = 86_400;
/// Day multiplier: leaves 5 decimal digits (0..=86399) for the second-of-day.
const DAY_MULTIPLIER: i64 = 100_000;
/// Android `versionCode` hard cap (Google Play). iOS `CFBundleVersion` has no comparable limit.
pub const ANDROID_VERSION_CODE_MAX: u64 = 2_100_000_000;

/// Pure computation for a given Unix timestamp (seconds, UTC). Split out for testing.
fn build_number_for(now_unix: i64) -> u64 {
    let days = (now_unix - EPOCH_2020).div_euclid(SECONDS_PER_DAY);
    let second_of_day = now_unix.rem_euclid(SECONDS_PER_DAY);
    (days * DAY_MULTIPLIER + second_of_day) as u64
}

/// Compute the build number from the current wall clock (UTC).
pub fn compute() -> u64 {
    build_number_for(Utc::now().timestamp())
}

/// Resolve the build number to stamp into the export.
///
/// `cli` (from `--build-number`) wins; otherwise `DCL_BUILD_NUMBER`; otherwise the clock.
pub fn resolve(cli: Option<u64>) -> u64 {
    let value = cli
        .or_else(|| {
            std::env::var("DCL_BUILD_NUMBER")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
        })
        .unwrap_or_else(compute);

    if value > ANDROID_VERSION_CODE_MAX {
        // ~year 2077 away — a guard, not an expected condition.
        eprintln!(
            "⚠️  build number {value} exceeds the Android versionCode cap ({ANDROID_VERSION_CODE_MAX})"
        );
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_at_epoch() {
        assert_eq!(build_number_for(EPOCH_2020), 0);
    }

    #[test]
    fn one_day_later_is_100000() {
        assert_eq!(build_number_for(EPOCH_2020 + SECONDS_PER_DAY), 100_000);
    }

    #[test]
    fn encodes_day_and_second_of_day() {
        // day 2367, 14:30:00 UTC => 2367*100000 + 52200
        let ts = EPOCH_2020 + 2367 * SECONDS_PER_DAY + (14 * 3600 + 30 * 60);
        assert_eq!(build_number_for(ts), 2367 * 100_000 + 52_200);
    }

    #[test]
    fn monotonic_across_day_boundary() {
        // last second of a day vs first second of the next: +100000 - 86399 = +13601 > 0.
        let last_second = EPOCH_2020 + SECONDS_PER_DAY - 1;
        assert!(build_number_for(last_second) < build_number_for(last_second + 1));
    }

    #[test]
    fn monotonic_second_to_second() {
        let t = EPOCH_2020 + 12345 * SECONDS_PER_DAY + 4242;
        assert!(build_number_for(t) < build_number_for(t + 1));
    }

    #[test]
    fn under_android_cap_until_2077() {
        // Day 20999 (~mid-2077) is the last fully-safe day under the cap.
        let day_20999 = EPOCH_2020 + 20_999 * SECONDS_PER_DAY + (SECONDS_PER_DAY - 1);
        assert!(build_number_for(day_20999) <= ANDROID_VERSION_CODE_MAX);
    }
}
