use std::collections::VecDeque;

use godot::builtin::Callable;
use godot::classes::PhysicsServer3D;
use godot::obj::Singleton;
use godot::prelude::Rid;

/// Generic object pool for reusing allocated resources
pub struct ObjectPool<T> {
    available: VecDeque<T>,
    capacity: usize,
    /// Stats for leak detection
    total_created: usize,
    total_in_use: usize,
}

impl<T> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::new(64)
    }
}

impl<T> ObjectPool<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            available: VecDeque::with_capacity(capacity),
            capacity,
            total_created: 0,
            total_in_use: 0,
        }
    }

    /// Acquire an item from the pool, or create a new one if empty
    /// Returns (item, was_reused) for logging purposes
    pub fn acquire<F: FnOnce() -> T>(&mut self, create: F) -> (T, bool) {
        self.total_in_use += 1;
        if let Some(item) = self.available.pop_front() {
            (item, true)
        } else {
            self.total_created += 1;
            (create(), false)
        }
    }

    /// Return an item to the pool.
    ///
    /// When the pool is already at capacity the item does not fit and is handed back to the
    /// caller as `Some(item)`, which MUST dispose of it. For an `Rid` that means
    /// `free_rid`: dropping the handle here leaks it inside the server, where nothing
    /// tracks it any more (the pool no longer counts it either - `total_created` is
    /// decremented so `created == in_use + pooled` keeps holding).
    #[must_use = "an item that did not fit in the pool must be disposed of by the caller, \
                  otherwise it leaks (for Rid: free_rid)"]
    pub fn release(&mut self, item: T) -> Option<T> {
        self.total_in_use = self.total_in_use.saturating_sub(1);
        if self.available.len() < self.capacity {
            self.available.push_back(item);
            None
        } else {
            self.total_created = self.total_created.saturating_sub(1);
            Some(item)
        }
    }

    /// Hand out every pooled item so the caller can dispose of it, keeping the
    /// `created == in_use + pooled` invariant (the drained items stop existing).
    pub fn drain_available(&mut self) -> Vec<T> {
        let drained: Vec<T> = self.available.drain(..).collect();
        self.total_created = self.total_created.saturating_sub(drained.len());
        drained
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.available.clear();
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.available.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.available.is_empty()
    }

    /// Get stats for leak detection: (total_created, in_use, pooled)
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.total_created, self.total_in_use, self.available.len())
    }
}

/// Pool for PhysicsServer3D Area RIDs
#[allow(dead_code)]
pub struct PhysicsAreaPool {
    areas: ObjectPool<Rid>,
    shapes_box: ObjectPool<Rid>,
    shapes_sphere: ObjectPool<Rid>,
}

impl Default for PhysicsAreaPool {
    fn default() -> Self {
        Self {
            areas: ObjectPool::new(32),
            shapes_box: ObjectPool::new(32),
            shapes_sphere: ObjectPool::new(32),
        }
    }
}

#[allow(dead_code)]
impl PhysicsAreaPool {
    pub fn acquire_area(&mut self) -> Rid {
        let (rid, _reused) = self
            .areas
            .acquire(|| PhysicsServer3D::singleton().area_create());
        rid
    }

    pub fn release_area(&mut self, rid: Rid) {
        let mut server = PhysicsServer3D::singleton();
        // Clear monitor callback to prevent stale events
        server.area_set_monitor_callback(rid, &Callable::invalid());
        server.area_clear_shapes(rid);
        server.area_set_space(rid, Rid::Invalid);
        if let Some(overflow) = self.areas.release(rid) {
            server.free_rid(overflow);
        }
    }

    pub fn acquire_box_shape(&mut self) -> Rid {
        let (rid, _reused) = self
            .shapes_box
            .acquire(|| PhysicsServer3D::singleton().box_shape_create());
        rid
    }

    pub fn acquire_sphere_shape(&mut self) -> Rid {
        let (rid, _reused) = self
            .shapes_sphere
            .acquire(|| PhysicsServer3D::singleton().sphere_shape_create());
        rid
    }

    pub fn release_box_shape(&mut self, rid: Rid) {
        if let Some(overflow) = self.shapes_box.release(rid) {
            PhysicsServer3D::singleton().free_rid(overflow);
        }
    }

    pub fn release_sphere_shape(&mut self, rid: Rid) {
        if let Some(overflow) = self.shapes_sphere.release(rid) {
            PhysicsServer3D::singleton().free_rid(overflow);
        }
    }

    pub fn cleanup(&mut self) {
        let mut server = PhysicsServer3D::singleton();
        for pool in [
            &mut self.areas,
            &mut self.shapes_box,
            &mut self.shapes_sphere,
        ] {
            for rid in pool.drain_available() {
                server.free_rid(rid);
            }
        }
        tracing::info!("[PhysicsAreaPool] CLEANUP: all pooled RIDs freed");
    }

    /// Get stats for areas pool: (created, in_use, pooled)
    pub fn stats_areas(&self) -> (usize, usize, usize) {
        self.areas.stats()
    }

    /// Get stats for box shapes pool: (created, in_use, pooled)
    pub fn stats_box_shapes(&self) -> (usize, usize, usize) {
        self.shapes_box.stats()
    }

    /// Get stats for sphere shapes pool: (created, in_use, pooled)
    pub fn stats_sphere_shapes(&self) -> (usize, usize, usize) {
        self.shapes_sphere.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectPool;

    /// `created == in_use + pooled` is what PoolManager's health check asserts; an
    /// overflow release used to break it silently (and leak the item with it).
    fn assert_balanced(pool: &ObjectPool<u32>) {
        let (created, in_use, pooled) = pool.stats();
        assert_eq!(
            created,
            in_use + pooled,
            "created={created} != in_use={in_use} + pooled={pooled}"
        );
    }

    #[test]
    fn release_within_capacity_pools_the_item() {
        let mut pool: ObjectPool<u32> = ObjectPool::new(2);
        let (a, reused) = pool.acquire(|| 1);
        assert!(!reused);
        assert!(pool.release(a).is_none(), "fits in the pool");
        assert_balanced(&pool);
        assert_eq!(pool.stats(), (1, 0, 1));

        // the next acquire reuses it instead of creating
        let (_, reused) = pool.acquire(|| 2);
        assert!(reused);
        assert_eq!(pool.stats(), (1, 1, 0));
    }

    #[test]
    fn release_over_capacity_hands_the_item_back_for_disposal() {
        let mut pool: ObjectPool<u32> = ObjectPool::new(2);
        let items: Vec<u32> = (0..3).map(|i| pool.acquire(|| i).0).collect();
        assert_eq!(pool.stats(), (3, 3, 0));

        let mut disposed = Vec::new();
        for item in items {
            if let Some(overflow) = pool.release(item) {
                disposed.push(overflow);
            }
        }

        // Exactly one did not fit: it is handed back, not silently dropped.
        assert_eq!(
            disposed.len(),
            1,
            "overflow item must be returned to caller"
        );
        assert_balanced(&pool);
        assert_eq!(pool.stats(), (2, 0, 2));
    }

    #[test]
    fn drain_available_keeps_the_invariant() {
        let mut pool: ObjectPool<u32> = ObjectPool::new(4);
        let items: Vec<u32> = (0..3).map(|i| pool.acquire(|| i).0).collect();
        for item in items {
            assert!(pool.release(item).is_none());
        }
        assert_eq!(pool.stats(), (3, 0, 3));

        let drained = pool.drain_available();
        assert_eq!(drained.len(), 3);
        assert_balanced(&pool);
        assert_eq!(pool.stats(), (0, 0, 0));
    }

    #[test]
    fn churn_beyond_capacity_stays_balanced() {
        let mut pool: ObjectPool<u32> = ObjectPool::new(2);
        for round in 0..10u32 {
            let batch: Vec<u32> = (0..5).map(|i| pool.acquire(|| round * 10 + i).0).collect();
            for item in batch {
                // caller disposes of whatever did not fit
                let _ = pool.release(item);
            }
            assert_balanced(&pool);
        }
        let (created, in_use, pooled) = pool.stats();
        assert_eq!((in_use, pooled), (0, 2));
        assert_eq!(created, 2, "pool must not accumulate untracked items");
    }
}
