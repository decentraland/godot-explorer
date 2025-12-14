use std::collections::VecDeque;

use godot::engine::PhysicsServer3D;
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

    pub fn release(&mut self, item: T) {
        self.total_in_use = self.total_in_use.saturating_sub(1);
        if self.available.len() < self.capacity {
            self.available.push_back(item);
        }
        // Note: if at capacity, item is dropped (for RIDs this would be a leak if not freed)
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
        let (rid, reused) = self.areas
            .acquire(|| PhysicsServer3D::singleton().area_create());
        let (created, in_use, pooled) = self.areas.stats();
        tracing::debug!(
            "[PhysicsAreaPool] ACQUIRE area: rid={:?}, reused={}, stats=(created={}, in_use={}, pooled={})",
            rid, reused, created, in_use, pooled
        );
        rid
    }

    pub fn release_area(&mut self, rid: Rid) {
        let mut server = PhysicsServer3D::singleton();
        server.area_clear_shapes(rid);
        server.area_set_space(rid, Rid::Invalid);
        self.areas.release(rid);
        let (created, in_use, pooled) = self.areas.stats();
        tracing::debug!(
            "[PhysicsAreaPool] RELEASE area: rid={:?}, stats=(created={}, in_use={}, pooled={})",
            rid, created, in_use, pooled
        );
    }

    pub fn acquire_box_shape(&mut self) -> Rid {
        let (rid, reused) = self.shapes_box
            .acquire(|| PhysicsServer3D::singleton().box_shape_create());
        let (created, in_use, pooled) = self.shapes_box.stats();
        tracing::debug!(
            "[PhysicsAreaPool] ACQUIRE box_shape: rid={:?}, reused={}, stats=(created={}, in_use={}, pooled={})",
            rid, reused, created, in_use, pooled
        );
        rid
    }

    pub fn acquire_sphere_shape(&mut self) -> Rid {
        let (rid, reused) = self.shapes_sphere
            .acquire(|| PhysicsServer3D::singleton().sphere_shape_create());
        let (created, in_use, pooled) = self.shapes_sphere.stats();
        tracing::debug!(
            "[PhysicsAreaPool] ACQUIRE sphere_shape: rid={:?}, reused={}, stats=(created={}, in_use={}, pooled={})",
            rid, reused, created, in_use, pooled
        );
        rid
    }

    pub fn release_box_shape(&mut self, rid: Rid) {
        self.shapes_box.release(rid);
        let (created, in_use, pooled) = self.shapes_box.stats();
        tracing::debug!(
            "[PhysicsAreaPool] RELEASE box_shape: rid={:?}, stats=(created={}, in_use={}, pooled={})",
            rid, created, in_use, pooled
        );
    }

    pub fn release_sphere_shape(&mut self, rid: Rid) {
        self.shapes_sphere.release(rid);
        let (created, in_use, pooled) = self.shapes_sphere.stats();
        tracing::debug!(
            "[PhysicsAreaPool] RELEASE sphere_shape: rid={:?}, stats=(created={}, in_use={}, pooled={})",
            rid, created, in_use, pooled
        );
    }

    pub fn cleanup(&mut self) {
        let mut server = PhysicsServer3D::singleton();
        for rid in self.areas.available.drain(..) {
            server.free_rid(rid);
        }
        for rid in self.shapes_box.available.drain(..) {
            server.free_rid(rid);
        }
        for rid in self.shapes_sphere.available.drain(..) {
            server.free_rid(rid);
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
