//! The Day 1 welcome: one local notification per install, 24 h after the first launch that has
//! the notification permission. The fail-open `day1-notification` flag is its kill switch: off
//! disarms it and keeps the trigger time, so on again re-arms it for the same moment. Event
//! reminders and the permission prompt are not gated by it. The shell does the OS calls.

use crate::settings::{SettingKey, SettingValue, Settings};

pub const ID: &str = "day1_welcome";
pub const FLAG: &str = "day1-notification";
pub const DELAY_SECONDS: i64 = 24 * 60 * 60;
pub const DEEP_LINK: &str = "decentraland://open?position=0,0";

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Day1Notification {
    /// Nothing to do: already fired, not armed yet without permission, or 1.x armed it.
    Keep,
    /// Schedule it `delay_seconds` from now; the same id replaces one already armed.
    Arm {
        id: String,
        delay_seconds: u32,
        deep_link: String,
    },
    /// Cancel it in the OS.
    Disarm { id: String },
}

/// What is stored per install. `at` is 0 when 1.x scheduled it: armed, at an unknown time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Day1State {
    pub scheduled: bool,
    pub at: i64,
}

fn arm(delay: i64) -> Day1Notification {
    Day1Notification::Arm {
        id: ID.into(),
        delay_seconds: delay.clamp(1, u32::MAX as i64) as u32,
        deep_link: DEEP_LINK.into(),
    }
}

/// The action for now, and the state to store when it changes.
pub fn decide(
    state: Day1State,
    enabled: bool,
    permission: bool,
    now: i64,
) -> (Day1Notification, Option<Day1State>) {
    if !state.scheduled {
        if !enabled || !permission {
            return (Day1Notification::Keep, None);
        }
        let armed = Day1State {
            scheduled: true,
            at: now + DELAY_SECONDS,
        };
        return (arm(DELAY_SECONDS), Some(armed));
    }
    let pending = state.at == 0 || state.at > now;
    match (pending, enabled) {
        (false, _) => (Day1Notification::Keep, None),
        (true, false) => (Day1Notification::Disarm { id: ID.into() }, None),
        (true, true) if state.at == 0 => (Day1Notification::Keep, None),
        (true, true) => (arm(state.at - now), None),
    }
}

/// Reads the stored state and the flag, decides, and stores the new state.
pub fn sync(settings: &Settings, permission: bool, now: i64) -> Day1Notification {
    let state = Day1State {
        scheduled: settings.get(SettingKey::Day1NotificationScheduled) == SettingValue::Bool(true),
        at: match settings.get(SettingKey::Day1NotificationAt) {
            SettingValue::Int(at) => at,
            _ => 0,
        },
    };
    let enabled = crate::flags::is_enabled(FLAG, true);
    let (action, stored) = decide(state, enabled, permission, now);
    if let Some(stored) = stored {
        let at = settings.set(
            SettingKey::Day1NotificationAt,
            SettingValue::Int(stored.at),
            false,
        );
        let scheduled = settings.set(
            SettingKey::Day1NotificationScheduled,
            SettingValue::Bool(stored.scheduled),
            true,
        );
        if let Err(error) = at.and(scheduled) {
            tracing::error!("day1 notification: could not store: {error}");
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_000;
    const NEVER: Day1State = Day1State {
        scheduled: false,
        at: 0,
    };

    fn armed_at(at: i64) -> Day1State {
        Day1State {
            scheduled: true,
            at,
        }
    }

    fn delay(action: &Day1Notification) -> u32 {
        match action {
            Day1Notification::Arm { delay_seconds, .. } => *delay_seconds,
            other => panic!("expected Arm, got {other:?}"),
        }
    }

    #[test]
    fn the_first_launch_with_permission_arms_it_24_hours_out() {
        let (action, stored) = decide(NEVER, true, true, NOW);
        assert_eq!(
            action,
            Day1Notification::Arm {
                id: "day1_welcome".into(),
                delay_seconds: 86_400,
                deep_link: "decentraland://open?position=0,0".into(),
            }
        );
        assert_eq!(stored, Some(armed_at(NOW + 86_400)));
    }

    #[test]
    fn without_permission_or_with_the_flag_off_nothing_is_scheduled_or_stored() {
        assert_eq!(
            decide(NEVER, true, false, NOW),
            (Day1Notification::Keep, None)
        );
        assert_eq!(
            decide(NEVER, false, true, NOW),
            (Day1Notification::Keep, None)
        );
    }

    #[test]
    fn the_flag_off_disarms_it_and_on_re_arms_it_for_the_original_time() {
        let state = armed_at(NOW + 86_400);
        let later = NOW + 3_600;
        assert_eq!(
            decide(state, false, true, later),
            (Day1Notification::Disarm { id: ID.into() }, None),
            "the trigger time is kept"
        );
        let (action, stored) = decide(state, true, true, later);
        assert_eq!(delay(&action), 86_400 - 3_600);
        assert_eq!(stored, None);
    }

    #[test]
    fn once_fired_it_never_comes_back() {
        let state = armed_at(NOW);
        assert_eq!(
            decide(state, true, true, NOW),
            (Day1Notification::Keep, None)
        );
        assert_eq!(
            decide(state, false, true, NOW + 1),
            (Day1Notification::Keep, None)
        );
    }

    #[test]
    fn one_armed_by_1x_is_left_alone_unless_the_flag_turns_it_off() {
        let legacy = armed_at(0);
        assert_eq!(
            decide(legacy, true, true, NOW),
            (Day1Notification::Keep, None)
        );
        assert_eq!(
            decide(legacy, false, true, NOW).0,
            Day1Notification::Disarm { id: ID.into() }
        );
    }

    #[test]
    fn the_install_marker_survives_a_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::open(dir.path());
        assert!(matches!(
            sync(&settings, true, NOW),
            Day1Notification::Arm { .. }
        ));
        let reopened = Settings::open(dir.path());
        assert_eq!(
            reopened.get(SettingKey::Day1NotificationAt),
            SettingValue::Int(NOW + 86_400)
        );
        assert_eq!(delay(&sync(&reopened, true, NOW + 10)), 86_400 - 10);
    }
}
