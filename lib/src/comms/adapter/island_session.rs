use std::time::{Duration, Instant};

const REMOVAL_GRACE: Duration = Duration::from_secs(10);

/// Island tokens belong to a Pulse session. Removal parks the connector until Pulse
/// decides the session outcome or the grace period permits a fresh WS handshake.
#[derive(Default)]
pub(super) struct IslandSession {
    pulse_ready: bool,
    removed: Option<Instant>,
}

impl IslandSession {
    pub fn set_pulse_ready(&mut self, ready: bool) -> bool {
        if self.pulse_ready == ready {
            return false;
        }
        self.pulse_ready = ready;
        self.removed = None;
        true
    }

    pub fn remove(&mut self, now: Instant) {
        self.removed.get_or_insert(now);
    }

    pub fn is_removed(&self) -> bool {
        self.removed.is_some()
    }

    pub fn refresh_after_removal(&mut self, now: Instant) -> bool {
        if self.pulse_ready
            && self
                .removed
                .is_some_and(|at| now.duration_since(at) >= REMOVAL_GRACE)
        {
            self.removed = None;
            return true;
        }
        false
    }

    pub fn can_poll(&self) -> bool {
        self.pulse_ready && self.removed.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_requires_pulse_and_eviction_parks_both_arrival_orders() {
        for island_first in [true, false] {
            let mut session = IslandSession::default();
            assert!(!session.can_poll());
            assert!(session.set_pulse_ready(true));
            assert!(session.can_poll());
            if island_first {
                session.remove(Instant::now());
                assert!(!session.can_poll());
                // A still-established Pulse does not clear island eviction.
                assert!(!session.set_pulse_ready(true));
                assert!(!session.can_poll());
                session.set_pulse_ready(false);
            } else {
                session.set_pulse_ready(false);
                session.remove(Instant::now());
            }
            assert!(!session.can_poll());
        }
    }

    #[test]
    fn surviving_pulse_allows_one_fresh_handshake_after_grace() {
        let now = Instant::now();
        let mut session = IslandSession::default();
        session.set_pulse_ready(true);
        session.remove(now);
        session.remove(now + Duration::from_secs(9));
        assert!(!session.refresh_after_removal(now + Duration::from_secs(9)));
        assert!(session.is_removed());
        assert!(session.refresh_after_removal(now + REMOVAL_GRACE));
        assert!(session.can_poll());
        assert!(!session.refresh_after_removal(now + REMOVAL_GRACE));
        session.remove(now);
        session.set_pulse_ready(false);
        assert!(!session.refresh_after_removal(now + REMOVAL_GRACE));
        assert!(!session.can_poll());
    }

    #[test]
    fn resume_recovery_requires_a_new_pulse_establishment() {
        let mut session = IslandSession::default();
        session.set_pulse_ready(true);
        session.remove(Instant::now());
        assert!(!session.can_poll());
        session.set_pulse_ready(false);
        assert!(!session.can_poll());
        session.set_pulse_ready(true);
        assert!(session.can_poll());
    }
}
