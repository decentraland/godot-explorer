use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Tier {
    Singleton,
    Batched,
}

#[derive(Debug)]
pub struct PromotionTracker<K: Eq + Hash + Clone> {
    threshold: usize,
    counts: HashMap<K, (usize, Tier)>,
}

impl<K: Eq + Hash + Clone> PromotionTracker<K> {
    pub fn with_threshold(threshold: usize) -> Self {
        debug_assert!(threshold >= 2, "threshold of 1 makes singletons useless");
        Self {
            threshold,
            counts: HashMap::new(),
        }
    }

    pub fn record_add(&mut self, key: K) -> Tier {
        let entry = self.counts.entry(key).or_insert((0, Tier::Singleton));
        entry.0 += 1;
        entry.1
    }

    pub fn should_promote(&self, key: &K) -> bool {
        match self.counts.get(key) {
            Some((count, Tier::Singleton)) => *count >= self.threshold,
            _ => false,
        }
    }

    pub fn mark_promoted(&mut self, key: &K) {
        if let Some(entry) = self.counts.get_mut(key) {
            entry.1 = Tier::Batched;
        }
    }

    pub fn record_remove(&mut self, key: &K) -> Option<Tier> {
        let entry = self.counts.get_mut(key)?;
        if entry.0 > 0 {
            entry.0 -= 1;
        }
        if entry.0 == 0 {
            self.counts.remove(key);
            None
        } else {
            Some(entry.1)
        }
    }

    pub fn count(&self, key: &K) -> usize {
        self.counts.get(key).map(|(c, _)| *c).unwrap_or(0)
    }

    pub fn tier(&self, key: &K) -> Option<Tier> {
        self.counts.get(key).map(|(_, t)| *t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_add_returns_current_tier_without_auto_flipping() {
        let mut t: PromotionTracker<u32> = PromotionTracker::with_threshold(6);
        for _ in 0..7 {
            assert_eq!(t.record_add(42), Tier::Singleton);
        }
        assert!(t.should_promote(&42));
        t.mark_promoted(&42);
        assert_eq!(t.record_add(42), Tier::Batched);
    }

    #[test]
    fn should_promote_returns_true_only_for_pending_singletons_at_threshold() {
        let mut t: PromotionTracker<u32> = PromotionTracker::with_threshold(3);
        t.record_add(7);
        t.record_add(7);
        assert!(!t.should_promote(&7));
        t.record_add(7);
        assert!(t.should_promote(&7));
        t.mark_promoted(&7);
        assert!(!t.should_promote(&7));
    }

    #[test]
    fn distinct_keys_track_independently() {
        let mut t: PromotionTracker<u32> = PromotionTracker::with_threshold(2);
        t.record_add(1);
        t.record_add(2);
        assert_eq!(t.tier(&1), Some(Tier::Singleton));
        t.record_add(1);
        assert!(t.should_promote(&1));
        assert!(!t.should_promote(&2));
        t.mark_promoted(&1);
        assert_eq!(t.tier(&1), Some(Tier::Batched));
        assert_eq!(t.tier(&2), Some(Tier::Singleton));
    }

    #[test]
    fn remove_drops_to_singleton_count_and_evicts_at_zero() {
        let mut t: PromotionTracker<u32> = PromotionTracker::with_threshold(2);
        t.record_add(9);
        t.record_add(9);
        t.record_remove(&9);
        assert_eq!(t.count(&9), 1);
        t.record_remove(&9);
        assert_eq!(t.count(&9), 0);
        assert_eq!(t.tier(&9), None);
    }

    #[test]
    fn removed_then_re_added_starts_singleton_again() {
        let mut t: PromotionTracker<u32> = PromotionTracker::with_threshold(3);
        t.record_add(5);
        t.record_add(5);
        t.record_add(5);
        t.mark_promoted(&5);
        assert_eq!(t.tier(&5), Some(Tier::Batched));
        t.record_remove(&5);
        t.record_remove(&5);
        t.record_remove(&5);
        assert_eq!(t.tier(&5), None);
        assert_eq!(t.record_add(5), Tier::Singleton);
    }
}
