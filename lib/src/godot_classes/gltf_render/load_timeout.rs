use std::collections::BTreeMap;

#[derive(Default, Debug)]
pub struct LoadTimeoutQueue<Token: Clone + Eq> {
    deadlines: BTreeMap<u64, Vec<Token>>,
    next_seq: u64,
}

impl<Token: Clone + Eq> LoadTimeoutQueue<Token> {
    pub fn new() -> Self {
        Self {
            deadlines: BTreeMap::new(),
            next_seq: 0,
        }
    }

    pub fn schedule(&mut self, token: Token, deadline_ms: u64) {
        self.deadlines.entry(deadline_ms).or_default().push(token);
        self.next_seq = self.next_seq.wrapping_add(1);
    }

    pub fn cancel(&mut self, token: &Token) -> bool {
        let mut found = false;
        let mut empty_keys: Vec<u64> = Vec::new();
        for (k, v) in self.deadlines.iter_mut() {
            let before = v.len();
            v.retain(|t| t != token);
            if v.len() != before {
                found = true;
            }
            if v.is_empty() {
                empty_keys.push(*k);
            }
        }
        for k in empty_keys {
            self.deadlines.remove(&k);
        }
        found
    }

    pub fn drain_expired(&mut self, now_ms: u64) -> Vec<Token> {
        let mut out: Vec<Token> = Vec::new();
        let live: BTreeMap<u64, Vec<Token>> = self.deadlines.split_off(&(now_ms + 1));
        let expired = std::mem::replace(&mut self.deadlines, live);
        for (_, mut tokens) in expired {
            out.append(&mut tokens);
        }
        out
    }

    pub fn len(&self) -> usize {
        self.deadlines.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.deadlines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedules_and_drains_in_order() {
        let mut q: LoadTimeoutQueue<u32> = LoadTimeoutQueue::new();
        q.schedule(1, 100);
        q.schedule(2, 200);
        q.schedule(3, 300);
        let fired = q.drain_expired(150);
        assert_eq!(fired, vec![1]);
        let fired = q.drain_expired(250);
        assert_eq!(fired, vec![2]);
        let fired = q.drain_expired(1_000);
        assert_eq!(fired, vec![3]);
    }

    #[test]
    fn drain_expired_returns_all_at_or_below_deadline() {
        let mut q: LoadTimeoutQueue<u32> = LoadTimeoutQueue::new();
        q.schedule(1, 100);
        q.schedule(2, 100);
        q.schedule(3, 50);
        let mut fired = q.drain_expired(100);
        fired.sort();
        assert_eq!(fired, vec![1, 2, 3]);
        assert!(q.is_empty());
    }

    #[test]
    fn cancel_removes_token_before_fire() {
        let mut q: LoadTimeoutQueue<u32> = LoadTimeoutQueue::new();
        q.schedule(1, 100);
        q.schedule(2, 100);
        assert!(q.cancel(&1));
        let fired = q.drain_expired(200);
        assert_eq!(fired, vec![2]);
    }

    #[test]
    fn cancel_returns_false_when_token_unknown() {
        let mut q: LoadTimeoutQueue<u32> = LoadTimeoutQueue::new();
        q.schedule(1, 100);
        assert!(!q.cancel(&99));
    }

    #[test]
    fn nothing_fires_when_now_is_before_first_deadline() {
        let mut q: LoadTimeoutQueue<u32> = LoadTimeoutQueue::new();
        q.schedule(1, 100);
        let fired = q.drain_expired(99);
        assert!(fired.is_empty());
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn empty_bucket_is_purged_after_all_cancels() {
        let mut q: LoadTimeoutQueue<u32> = LoadTimeoutQueue::new();
        q.schedule(1, 100);
        q.schedule(2, 100);
        q.cancel(&1);
        q.cancel(&2);
        assert!(q.is_empty());
    }
}
