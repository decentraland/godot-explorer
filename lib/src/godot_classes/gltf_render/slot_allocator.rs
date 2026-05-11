use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SlotEvent {
    GrowTo(u32),
    SetVisibleInstances(u32),
}

#[derive(Default, Debug)]
pub struct SlotAllocator {
    capacity: u32,
    high_water: u32,
    free_slots: BinaryHeap<u32>,
    pending_events: Vec<SlotEvent>,
}

impl SlotAllocator {
    pub fn with_initial_capacity(initial: u32) -> Self {
        Self {
            capacity: initial,
            high_water: 0,
            free_slots: BinaryHeap::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn high_water(&self) -> u32 {
        self.high_water
    }

    pub fn live_count(&self) -> u32 {
        self.high_water - self.free_slots.len() as u32
    }

    pub fn allocate(&mut self) -> u32 {
        let slot = if let Some(s) = self.free_slots.pop() {
            s
        } else {
            if self.high_water >= self.capacity {
                let new_cap = self.capacity.saturating_mul(2).max(self.capacity + 1);
                self.capacity = new_cap;
                self.pending_events.push(SlotEvent::GrowTo(new_cap));
            }
            let s = self.high_water;
            self.high_water += 1;
            s
        };
        self.pending_events
            .push(SlotEvent::SetVisibleInstances(self.high_water));
        slot
    }

    pub fn release(&mut self, slot: u32) {
        debug_assert!(slot < self.high_water, "release of out-of-range slot");
        self.free_slots.push(slot);
        while let Some(&top) = self.free_slots.peek() {
            if top + 1 != self.high_water {
                break;
            }
            self.free_slots.pop();
            self.high_water = top;
        }
        self.pending_events
            .push(SlotEvent::SetVisibleInstances(self.high_water));
    }

    pub fn drain_events(&mut self) -> Vec<SlotEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_from_zero_in_order() {
        let mut a = SlotAllocator::with_initial_capacity(4);
        assert_eq!(a.allocate(), 0);
        assert_eq!(a.allocate(), 1);
        assert_eq!(a.allocate(), 2);
        assert_eq!(a.high_water(), 3);
        assert_eq!(a.live_count(), 3);
    }

    #[test]
    fn reuses_freed_slots_before_extending_high_water() {
        let mut a = SlotAllocator::with_initial_capacity(4);
        let s0 = a.allocate();
        let _s1 = a.allocate();
        let _s2 = a.allocate();
        a.release(s0);
        assert_eq!(a.allocate(), 0);
        assert_eq!(a.high_water(), 3);
    }

    #[test]
    fn tail_compaction_when_releasing_top_slot() {
        let mut a = SlotAllocator::with_initial_capacity(8);
        let _s0 = a.allocate();
        let _s1 = a.allocate();
        let s2 = a.allocate();
        a.release(s2);
        assert_eq!(a.high_water(), 2);
        assert_eq!(a.live_count(), 2);
    }

    #[test]
    fn tail_compaction_collapses_run_of_free_slots() {
        let mut a = SlotAllocator::with_initial_capacity(8);
        let _s0 = a.allocate();
        let s1 = a.allocate();
        let s2 = a.allocate();
        let s3 = a.allocate();
        a.release(s2);
        a.release(s3);
        assert_eq!(a.high_water(), 2, "two trailing releases collapse together");
        a.release(s1);
        assert_eq!(a.high_water(), 1);
        assert_eq!(a.live_count(), 1);
    }

    #[test]
    fn mid_buffer_release_does_not_collapse_high_water() {
        let mut a = SlotAllocator::with_initial_capacity(8);
        let _s0 = a.allocate();
        let s1 = a.allocate();
        let _s2 = a.allocate();
        a.release(s1);
        assert_eq!(a.high_water(), 3);
        assert_eq!(a.live_count(), 2);
    }

    #[test]
    fn growth_doubles_capacity_when_high_water_hits_cap() {
        let mut a = SlotAllocator::with_initial_capacity(2);
        a.allocate();
        a.allocate();
        a.drain_events();
        a.allocate();
        let events = a.drain_events();
        assert!(events.contains(&SlotEvent::GrowTo(4)));
        assert_eq!(a.capacity(), 4);
    }

    #[test]
    fn set_visible_instances_event_emitted_on_each_op() {
        let mut a = SlotAllocator::with_initial_capacity(4);
        a.allocate();
        let evs = a.drain_events();
        assert_eq!(*evs.last().unwrap(), SlotEvent::SetVisibleInstances(1));
        let s1 = a.allocate();
        a.drain_events();
        a.release(s1);
        let evs = a.drain_events();
        assert_eq!(*evs.last().unwrap(), SlotEvent::SetVisibleInstances(1));
    }
}
