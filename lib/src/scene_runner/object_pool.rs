use std::collections::VecDeque;

use godot::engine::PhysicsServer3D;
use godot::prelude::Rid;

/// Generic object pool for reusing allocated resources
pub struct ObjectPool<T> {
    available: VecDeque<T>,
    capacity: usize,
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
        }
    }

    pub fn acquire<F: FnOnce() -> T>(&mut self, create: F) -> T {
        self.available.pop_front().unwrap_or_else(create)
    }

    pub fn release(&mut self, item: T) {
        if self.available.len() < self.capacity {
            self.available.push_back(item);
        }
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
        self.areas
            .acquire(|| PhysicsServer3D::singleton().area_create())
    }

    pub fn release_area(&mut self, rid: Rid) {
        let mut server = PhysicsServer3D::singleton();
        server.area_clear_shapes(rid);
        server.area_set_space(rid, Rid::Invalid);
        self.areas.release(rid);
    }

    pub fn acquire_box_shape(&mut self) -> Rid {
        self.shapes_box
            .acquire(|| PhysicsServer3D::singleton().box_shape_create())
    }

    pub fn acquire_sphere_shape(&mut self) -> Rid {
        self.shapes_sphere
            .acquire(|| PhysicsServer3D::singleton().sphere_shape_create())
    }

    pub fn release_box_shape(&mut self, rid: Rid) {
        self.shapes_box.release(rid);
    }

    pub fn release_sphere_shape(&mut self, rid: Rid) {
        self.shapes_sphere.release(rid);
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
    }
}
