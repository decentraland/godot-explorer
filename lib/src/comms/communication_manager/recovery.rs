use std::time::{Duration, Instant};

#[cfg(any(feature = "use_pulse", test))]
const OUTAGE_NOTICE_DELAY: Duration = Duration::from_secs(10);
const RESUME_GRACE: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct Recovery {
    pub failures: u32,
    pub stopped: Option<i32>,
    outage_since: Option<Instant>,
    notified: bool,
    resume_until: Option<Instant>,
    reclaimed: bool,
}

impl Recovery {
    #[cfg(any(feature = "use_pulse", test))]
    pub fn unavailable(&mut self, now: Instant) -> bool {
        let since = *self.outage_since.get_or_insert(now);
        if self.stopped.is_none()
            && !self.notified
            && now.duration_since(since) >= OUTAGE_NOTICE_DELAY
        {
            self.notified = true;
            return true;
        }
        false
    }

    #[cfg(any(feature = "use_pulse", test))]
    pub fn established(&mut self) -> bool {
        let notified = self.notified;
        self.reset_outage();
        notified
    }

    pub fn reset_outage(&mut self) {
        self.outage_since = None;
        self.notified = false;
        self.failures = 0;
    }

    pub fn resumed(&mut self, now: Instant) {
        if self.stopped.is_some() || self.resume_until.is_some_and(|until| now < until) {
            return;
        }
        self.resume_until = Some(now + RESUME_GRACE);
        self.reclaimed = false;
        self.reset_outage();
    }

    pub fn in_resume_grace(&self, now: Instant) -> bool {
        self.stopped.is_none() && self.resume_until.is_some_and(|until| now < until)
    }

    #[cfg(any(feature = "use_pulse", test))]
    pub fn reclaim_after_resume(&mut self, now: Instant) -> bool {
        if self.reclaimed || !self.in_resume_grace(now) {
            return false;
        }
        self.reclaimed = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outage_notifies_once_and_only_establishment_resets_it() {
        let now = Instant::now();
        let mut recovery = Recovery::default();
        assert!(!recovery.established());
        assert!(!recovery.unavailable(now));
        assert!(!recovery.unavailable(now + Duration::from_secs(9)));
        assert!(recovery.unavailable(now + Duration::from_secs(10)));
        assert!(!recovery.unavailable(now + Duration::from_secs(60)));
        assert!(recovery.established());
        assert!(!recovery.established());
        assert!(!recovery.unavailable(now + Duration::from_secs(61)));
        assert!(recovery.unavailable(now + Duration::from_secs(71)));
    }

    #[test]
    fn resume_allows_only_one_reclaim_even_with_repeated_focus_events() {
        let now = Instant::now();
        let mut recovery = Recovery::default();
        assert!(!recovery.reclaim_after_resume(now));
        recovery.resumed(now);
        assert!(recovery.reclaim_after_resume(now));
        recovery.resumed(now + Duration::from_secs(1));
        assert!(!recovery.reclaim_after_resume(now + Duration::from_secs(1)));
    }

    #[test]
    fn grace_expires_at_thirty_seconds() {
        let now = Instant::now();
        let mut recovery = Recovery::default();
        recovery.resumed(now);
        assert!(!recovery.reclaim_after_resume(now + RESUME_GRACE));
    }

    #[test]
    fn terminal_decision_survives_resume_and_outage_resets() {
        let now = Instant::now();
        for reason in [0, 5, 4] {
            let mut recovery = Recovery {
                stopped: Some(reason),
                ..Default::default()
            };
            recovery.resumed(now);
            recovery.reset_outage();
            assert!(!recovery.reclaim_after_resume(now));
            assert!(!recovery.unavailable(now));
            assert!(!recovery.unavailable(now + Duration::from_secs(60)));
            assert_eq!(recovery.stopped, Some(reason));
        }
    }
}
