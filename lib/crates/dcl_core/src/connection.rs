//! How good the connection is, and when the player should hear about it.
//!
//! Two layers. Health is a truthful 0..1 score driven only by ping outcomes: a failure lowers it
//! (a hang more than a quick refusal), a success raises it (a slow one heals less), and separate
//! enter and exit thresholds keep a single lucky ping from flapping the state. Feedback decides
//! when to show the toast and the dialog: "retry" only snoozes the dialog (it reconnects nothing),
//! with a quiet window that backs off on each retry and relaxes during calm; only a real recovery
//! dismisses it for good.

use std::sync::Mutex;

pub const FAST_POLL_SECS: f64 = 0.5;
pub const SLOW_POLL_SECS: f64 = 10.0;
pub const REQUEST_TIMEOUT_SECS: f64 = 3.0;

const HEALTH_W_FAIL: f64 = 0.23;
const HEALTH_W_TIMEOUT: f64 = 0.26;
const HEALTH_W_OK: f64 = 0.34;

const BAND_POOR_ENTER: f64 = 0.55;
const BAND_POOR_EXIT: f64 = 0.65;
const BAND_LOST_ENTER: f64 = 0.25;
const BAND_LOST_EXIT: f64 = 0.45;

const SNOOZE_BASE: f64 = 10.0;
const SNOOZE_GROWTH: f64 = 2.0;
const SNOOZE_MAX: f64 = 120.0;
const SNOOZE_DECAY_HALFLIFE: f64 = 45.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Quality {
    Good,
    Poor,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sample {
    Ok { latency_secs: f64 },
    Failed,
    TimedOut,
}

/// What a sample changed, for the shell to show.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Report {
    pub quality: Quality,
    /// Degrading from good to poor while in a world: the toast.
    pub toast: bool,
    /// Lost (or poor with no world to toast in) and not snoozed: the dialog.
    pub alarm: bool,
    /// Back to good from poor or lost: the dialog goes away for good.
    pub restored: bool,
    pub next_poll_ms: u64,
}

struct Inner {
    health: f64,
    quality: Quality,
    snooze_until: f64,
    snooze_next: f64,
    snooze_updated: f64,
}

pub struct Monitor(Mutex<Inner>);

impl Default for Monitor {
    fn default() -> Self {
        Self(Mutex::new(Inner {
            health: 1.0,
            quality: Quality::Good,
            snooze_until: 0.0,
            snooze_next: SNOOZE_BASE,
            snooze_updated: 0.0,
        }))
    }
}

fn classify(current: Quality, health: f64) -> Quality {
    match current {
        Quality::Lost => {
            if health >= BAND_LOST_EXIT {
                if health >= BAND_POOR_EXIT {
                    Quality::Good
                } else {
                    Quality::Poor
                }
            } else {
                Quality::Lost
            }
        }
        Quality::Poor => {
            if health < BAND_LOST_ENTER {
                Quality::Lost
            } else if health >= BAND_POOR_EXIT {
                Quality::Good
            } else {
                Quality::Poor
            }
        }
        Quality::Good => {
            if health < BAND_LOST_ENTER {
                Quality::Lost
            } else if health < BAND_POOR_ENTER {
                Quality::Poor
            } else {
                Quality::Good
            }
        }
    }
}

impl Monitor {
    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_good(&self) -> bool {
        self.inner().quality == Quality::Good
    }

    /// A realm change settles: start from a clean slate.
    pub fn reset(&self) {
        let mut inner = self.inner();
        inner.health = 1.0;
        inner.quality = Quality::Good;
    }

    /// `now` is seconds on a clock that only moves forward; `in_world` says a toast has a place.
    pub fn observe(&self, sample: Sample, now: f64, in_world: bool) -> Report {
        let mut inner = self.inner();
        let before = inner.quality;
        match sample {
            Sample::Failed => inner.health -= HEALTH_W_FAIL,
            Sample::TimedOut => inner.health -= HEALTH_W_TIMEOUT,
            Sample::Ok { latency_secs } => {
                let factor = (1.0 - latency_secs / REQUEST_TIMEOUT_SECS).clamp(0.0, 1.0);
                inner.health += HEALTH_W_OK * factor;
            }
        }
        inner.health = inner.health.clamp(0.0, 1.0);
        inner.quality = classify(before, inner.health);
        let quality = inner.quality;

        let restored = quality == Quality::Good && before != Quality::Good;
        if restored {
            // A later, unrelated problem starts fresh.
            inner.snooze_next = SNOOZE_BASE;
            inner.snooze_updated = now;
        }
        let toast = quality == Quality::Poor && before == Quality::Good && in_world;
        let alarm_state = quality == Quality::Lost || (quality == Quality::Poor && !in_world);
        let alarm = alarm_state && !restored && now >= inner.snooze_until;
        let investigating =
            matches!(sample, Sample::Failed | Sample::TimedOut) || inner.health < BAND_POOR_EXIT;
        Report {
            quality,
            toast,
            alarm,
            restored,
            next_poll_ms: ((if investigating {
                FAST_POLL_SECS
            } else {
                SLOW_POLL_SECS
            }) * 1000.0) as u64,
        }
    }

    /// The player pressed retry: quiet for a window that grows with each retry and relaxes in calm.
    pub fn retry(&self, now: f64) {
        let mut inner = self.inner();
        let dt = now - inner.snooze_updated;
        inner.snooze_updated = now;
        let excess = inner.snooze_next - SNOOZE_BASE;
        if dt > 0.0 && excess > 0.0 {
            inner.snooze_next = SNOOZE_BASE + excess * 0.5f64.powf(dt / SNOOZE_DECAY_HALFLIFE);
        }
        inner.snooze_until = now + inner.snooze_next;
        inner.snooze_next = (inner.snooze_next * SNOOZE_GROWTH).min(SNOOZE_MAX);
    }
}

pub fn global() -> &'static Monitor {
    static MONITOR: std::sync::OnceLock<Monitor> = std::sync::OnceLock::new();
    MONITOR.get_or_init(Monitor::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: Sample = Sample::Ok { latency_secs: 0.1 };

    fn fail(monitor: &Monitor, times: usize, in_world: bool) -> Report {
        let mut last = None;
        for i in 0..times {
            last = Some(monitor.observe(Sample::Failed, i as f64, in_world));
        }
        last.unwrap()
    }

    #[test]
    fn two_failures_make_it_poor_and_four_make_it_lost() {
        let monitor = Monitor::default();
        assert_eq!(fail(&monitor, 1, true).quality, Quality::Good);
        let poor = monitor.observe(Sample::Failed, 1.0, true);
        assert_eq!(poor.quality, Quality::Poor);
        assert!(poor.toast, "the first step down in a world warns");
        assert!(!poor.alarm);
        monitor.observe(Sample::Failed, 2.0, true);
        let lost = monitor.observe(Sample::Failed, 3.0, true);
        assert_eq!(lost.quality, Quality::Lost);
        assert!(lost.alarm && !lost.toast);
    }

    #[test]
    fn with_no_world_to_toast_in_poor_alarms_at_once() {
        let monitor = Monitor::default();
        monitor.observe(Sample::Failed, 0.0, false);
        let report = monitor.observe(Sample::Failed, 1.0, false);
        assert_eq!(report.quality, Quality::Poor);
        assert!(report.alarm && !report.toast);
    }

    #[test]
    fn a_lucky_ping_does_not_leave_lost_and_two_clean_ones_do() {
        let monitor = Monitor::default();
        fail(&monitor, 5, true);
        let one = monitor.observe(OK, 10.0, true);
        assert_eq!(one.quality, Quality::Lost, "hysteresis");
        let two = monitor.observe(OK, 11.0, true);
        assert_eq!(two.quality, Quality::Good);
        assert!(two.restored && !two.alarm);
        assert!(monitor.is_good());
    }

    #[test]
    fn a_slow_success_heals_less_than_a_fast_one() {
        let slow = Monitor::default();
        let fast = Monitor::default();
        for monitor in [&slow, &fast] {
            fail(monitor, 4, true);
        }
        slow.observe(Sample::Ok { latency_secs: 2.9 }, 10.0, true);
        fast.observe(Sample::Ok { latency_secs: 0.05 }, 10.0, true);
        assert!(slow.inner().health < fast.inner().health);
    }

    #[test]
    fn it_polls_fast_while_degraded_and_slowly_when_solidly_good() {
        let monitor = Monitor::default();
        assert_eq!(monitor.observe(OK, 0.0, true).next_poll_ms, 10_000);
        assert_eq!(monitor.observe(Sample::Failed, 1.0, true).next_poll_ms, 500);
        for t in 2..5 {
            monitor.observe(OK, t as f64, true);
        }
        assert_eq!(monitor.observe(OK, 6.0, true).next_poll_ms, 10_000);
    }

    #[test]
    fn retry_snoozes_the_dialog_with_a_growing_window() {
        let monitor = Monitor::default();
        fail(&monitor, 4, true);
        assert!(monitor.observe(Sample::Failed, 10.0, true).alarm);
        monitor.retry(10.0);
        assert!(
            !monitor.observe(Sample::Failed, 19.0, true).alarm,
            "quiet for 10 s"
        );
        assert!(monitor.observe(Sample::Failed, 20.0, true).alarm);
        monitor.retry(20.0);
        // the window doubled to 20 s, less what relaxed during the 10 s of calm in between
        assert!(
            !monitor.observe(Sample::Failed, 35.0, true).alarm,
            "longer the second time"
        );
        assert!(monitor.observe(Sample::Failed, 40.0, true).alarm);
    }

    #[test]
    fn the_snooze_window_is_capped_and_relaxes_during_calm() {
        let monitor = Monitor::default();
        let mut now = 0.0;
        for _ in 0..10 {
            monitor.retry(now);
            now += 1.0;
        }
        assert_eq!(monitor.inner().snooze_next, SNOOZE_MAX);
        // a long calm brings the next window back toward the base
        monitor.retry(now + 10.0 * SNOOZE_DECAY_HALFLIFE);
        assert!(monitor.inner().snooze_until - (now + 10.0 * SNOOZE_DECAY_HALFLIFE) < 11.0);
    }
}
