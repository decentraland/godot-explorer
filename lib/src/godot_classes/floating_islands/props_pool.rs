use std::collections::HashMap;

use godot::builtin::{PackedFloat32Array, Rid, Transform3D, Vector3};
use godot::classes::rendering_server::MultimeshTransformFormat;
use godot::classes::{Mesh, RenderingServer};
use godot::obj::Gd;
use godot::prelude::*;

const INITIAL_CAPACITY: u32 = 32;

#[derive(Clone, Copy, Debug)]
pub struct PropSlotId {
    pub mesh_rid: Rid,
    pub slot: u32,
}

struct PropPool {
    mesh_rid: Rid,
    multimesh_rid: Rid,
    instance_rid: Rid,
    capacity: u32,
    high_water: u32,
    free_slots: Vec<u32>,
}

#[derive(Default)]
pub struct PropPoolManager {
    pools: HashMap<Rid, PropPool>,
}

impl PropPoolManager {
    pub fn allocate_slot(
        &mut self,
        mesh: &Gd<Mesh>,
        scenario: Rid,
        world_transform: Transform3D,
    ) -> Option<PropSlotId> {
        let mesh_rid = mesh.get_rid();
        let pool = self
            .pools
            .entry(mesh_rid)
            .or_insert_with(|| create_pool(mesh_rid, scenario));

        let slot = if let Some(s) = pool.free_slots.pop() {
            s
        } else {
            if pool.high_water >= pool.capacity {
                grow_pool(pool, pool.capacity.saturating_mul(2).max(pool.capacity + 1));
            }
            let s = pool.high_water;
            pool.high_water += 1;
            s
        };

        let mut rs = RenderingServer::singleton();
        rs.multimesh_instance_set_transform(pool.multimesh_rid, slot as i32, world_transform);
        rs.multimesh_set_visible_instances(pool.multimesh_rid, pool.high_water as i32);
        Some(PropSlotId { mesh_rid, slot })
    }

    pub fn release_slot(&mut self, id: PropSlotId) {
        let Some(pool) = self.pools.get_mut(&id.mesh_rid) else {
            return;
        };
        let mut rs = RenderingServer::singleton();
        rs.multimesh_instance_set_transform(
            pool.multimesh_rid,
            id.slot as i32,
            zero_scale_transform(),
        );
        pool.free_slots.push(id.slot);
        // Trim trailing free slots so the GPU stops iterating instances that
        // will only ever be zero-scaled. We can't reorder mid-buffer slots
        // without re-issuing PropSlotIds, but tail compaction is cheap.
        while pool.high_water > 0 && pool.free_slots.iter().any(|&s| s + 1 == pool.high_water) {
            let top = pool.high_water - 1;
            pool.free_slots.retain(|&s| s != top);
            pool.high_water = top;
        }
        rs.multimesh_set_visible_instances(pool.multimesh_rid, pool.high_water as i32);
    }

    pub fn clear(&mut self) {
        let mut rs = RenderingServer::singleton();
        for pool in self.pools.values() {
            if pool.instance_rid.is_valid() {
                rs.free_rid(pool.instance_rid);
            }
            if pool.multimesh_rid.is_valid() {
                rs.free_rid(pool.multimesh_rid);
            }
        }
        self.pools.clear();
    }
}

fn create_pool(mesh_rid: Rid, scenario: Rid) -> PropPool {
    let mut rs = RenderingServer::singleton();
    let multimesh = rs.multimesh_create();
    rs.multimesh_allocate_data_ex(
        multimesh,
        INITIAL_CAPACITY as i32,
        MultimeshTransformFormat::TRANSFORM_3D,
    )
    .done();
    rs.multimesh_set_mesh(multimesh, mesh_rid);
    rs.multimesh_set_visible_instances(multimesh, 0);

    let instance = rs.instance_create2(multimesh, scenario);
    rs.instance_set_transform(instance, Transform3D::IDENTITY);

    PropPool {
        mesh_rid,
        multimesh_rid: multimesh,
        instance_rid: instance,
        capacity: INITIAL_CAPACITY,
        high_water: 0,
        free_slots: Vec::new(),
    }
}

fn grow_pool(pool: &mut PropPool, new_capacity: u32) {
    let mut rs = RenderingServer::singleton();
    let new_mm = rs.multimesh_create();
    rs.multimesh_allocate_data_ex(
        new_mm,
        new_capacity as i32,
        MultimeshTransformFormat::TRANSFORM_3D,
    )
    .done();
    rs.multimesh_set_mesh(new_mm, pool.mesh_rid);
    if pool.high_water > 0 {
        // Bulk-copy the live transform buffer (12 floats per instance) instead
        // of round-tripping each slot through the GDExtension boundary.
        let buffer: PackedFloat32Array = rs.multimesh_get_buffer(pool.multimesh_rid);
        let live_floats = (pool.high_water as usize) * 12;
        if buffer.len() >= live_floats {
            let mut new_buffer = PackedFloat32Array::new();
            new_buffer.resize(live_floats);
            new_buffer
                .as_mut_slice()
                .copy_from_slice(&buffer.as_slice()[..live_floats]);
            rs.multimesh_set_buffer(new_mm, &new_buffer);
        }
    }
    rs.multimesh_set_visible_instances(new_mm, pool.high_water as i32);
    rs.instance_set_base(pool.instance_rid, new_mm);
    rs.free_rid(pool.multimesh_rid);
    pool.multimesh_rid = new_mm;
    pool.capacity = new_capacity;
}

fn zero_scale_transform() -> Transform3D {
    Transform3D::IDENTITY.scaled(Vector3::ZERO)
}
