/// Island tokens belong to a Pulse session. Removal parks the connector until that
/// session is replaced; reconnecting LiveKit alone cannot reclaim a revoked token.
#[derive(Default)]
pub(super) struct IslandSession {
    pulse_ready: bool,
    removed: bool,
}

impl IslandSession {
    pub fn set_pulse_ready(&mut self, ready: bool) -> bool {
        if self.pulse_ready == ready {
            return false;
        }
        self.pulse_ready = ready;
        self.removed = false;
        true
    }

    pub fn remove(&mut self) {
        self.removed = true;
    }

    pub fn can_poll(&self) -> bool {
        self.pulse_ready && !self.removed
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
                session.remove();
                assert!(!session.can_poll());
                // A still-established Pulse does not clear island eviction.
                assert!(!session.set_pulse_ready(true));
                assert!(!session.can_poll());
                session.set_pulse_ready(false);
            } else {
                session.set_pulse_ready(false);
                session.remove();
            }
            assert!(!session.can_poll());
        }
    }

    #[test]
    fn resume_recovery_requires_a_new_pulse_establishment() {
        let mut session = IslandSession::default();
        session.set_pulse_ready(true);
        session.remove();
        assert!(!session.can_poll());
        session.set_pulse_ready(false);
        assert!(!session.can_poll());
        session.set_pulse_ready(true);
        assert!(session.can_poll());
    }
}
