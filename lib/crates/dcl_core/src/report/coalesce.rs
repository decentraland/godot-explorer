//! Batches report-tag changes so a burst (the engine sets one per resource load) reaches the
//! shell as the latest value per key, at most once per interval.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// A tag change: `None` removes the tag.
pub type TagChange = (String, Option<String>);

/// What the caller does after [`TagCoalescer::push`].
#[derive(Debug, PartialEq, Eq)]
pub enum Push {
    /// Deliver these now.
    Deliver(Vec<TagChange>),
    /// A flush is due after this long: call [`TagCoalescer::flush`] then.
    FlushIn(Duration),
    /// A flush is already scheduled and will carry this change.
    Queued,
}

#[derive(Default)]
struct State {
    pending: Vec<TagChange>,
    last_sent: Option<Instant>,
    flush_scheduled: bool,
}

pub struct TagCoalescer {
    interval: Duration,
    state: Mutex<State>,
}

impl TagCoalescer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            state: Mutex::new(State::default()),
        }
    }

    pub fn push(&self, key: &str, value: Option<&str>, now: Instant) -> Push {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let change = (key.to_string(), value.map(str::to_string));
        match state.pending.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => *slot = change,
            None => state.pending.push(change),
        }
        if state.flush_scheduled {
            return Push::Queued;
        }
        let elapsed = state.last_sent.map(|t| now.saturating_duration_since(t));
        match elapsed {
            Some(elapsed) if elapsed < self.interval => {
                state.flush_scheduled = true;
                Push::FlushIn(self.interval - elapsed)
            }
            _ => {
                state.last_sent = Some(now);
                Push::Deliver(std::mem::take(&mut state.pending))
            }
        }
    }

    /// Takes everything pending; the scheduled flush calls it.
    pub fn flush(&self, now: Instant) -> Vec<TagChange> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.flush_scheduled = false;
        state.last_sent = Some(now);
        std::mem::take(&mut state.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: Duration = Duration::from_millis(250);

    fn change(key: &str, value: Option<&str>) -> TagChange {
        (key.to_string(), value.map(str::to_string))
    }

    #[test]
    fn first_change_is_delivered_at_once() {
        let c = TagCoalescer::new(INTERVAL);
        let push = c.push("loading_resource", Some("a"), Instant::now());
        assert_eq!(
            push,
            Push::Deliver(vec![change("loading_resource", Some("a"))])
        );
    }

    #[test]
    fn a_burst_collapses_to_the_latest_value_in_one_flush() {
        let c = TagCoalescer::new(INTERVAL);
        let t0 = Instant::now();
        c.push("k", Some("a"), t0);
        let at = |ms| t0 + Duration::from_millis(ms);
        assert_eq!(
            c.push("k", None, at(1)),
            Push::FlushIn(INTERVAL - Duration::from_millis(1))
        );
        assert_eq!(c.push("k", Some("b"), at(2)), Push::Queued);
        assert_eq!(c.push("k", Some("c"), at(3)), Push::Queued);
        assert_eq!(c.flush(at(251)), vec![change("k", Some("c"))]);
    }

    #[test]
    fn keys_are_independent_and_keep_their_first_position() {
        let c = TagCoalescer::new(INTERVAL);
        let t0 = Instant::now();
        c.push("x", Some("1"), t0);
        c.push("a", Some("1"), t0 + Duration::from_millis(1));
        c.push("b", Some("2"), t0 + Duration::from_millis(2));
        c.push("a", None, t0 + Duration::from_millis(3));
        assert_eq!(
            c.flush(t0 + INTERVAL),
            vec![change("a", None), change("b", Some("2"))]
        );
    }

    #[test]
    fn after_the_interval_the_next_change_is_delivered_at_once() {
        let c = TagCoalescer::new(INTERVAL);
        let t0 = Instant::now();
        c.push("k", Some("a"), t0);
        let later = t0 + INTERVAL;
        assert_eq!(
            c.push("k", Some("b"), later),
            Push::Deliver(vec![change("k", Some("b"))])
        );
    }

    #[test]
    fn a_flush_with_nothing_pending_is_empty() {
        let c = TagCoalescer::new(INTERVAL);
        assert!(c.flush(Instant::now()).is_empty());
    }
}
