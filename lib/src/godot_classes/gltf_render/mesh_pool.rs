//! MultiMesh batching pool for SDK7 GLTF static surfaces. One `MeshPool` per
//! distinct `(mesh_rid, layer_mask)` key — every container that registers an
//! occurrence of that pair lands in the same MultiMesh, so identical GLBs (or
//! shared submeshes between different GLBs) collapse into one draw call.
//!
//! Mirrors `floating_islands/props_pool.rs` semantics: max-heap freelist with
//! tail compaction, geometric growth on overflow, zero-scale transform on
//! release so the GPU stops drawing freed slots. We don't reuse our pure-Rust
//! `SlotAllocator` here because the floating-islands pattern bakes ordering
//! assumptions (RS calls interleaved with `high_water` mutations during grow)
//! that are easier to read inline than to encode through an event channel.

use std::collections::{BinaryHeap, HashMap};

use godot::builtin::{PackedFloat32Array, Rid, Transform3D, Vector3};
use godot::classes::rendering_server::MultimeshTransformFormat;
use godot::classes::RenderingServer;
use godot::prelude::*;

const INITIAL_CAPACITY: u32 = 32;

/// Batch key. Same mesh + same layer mask collapse into one MultiMesh; if
/// two containers happen to want the same mesh under different masks they
/// fragment naturally into separate pools.
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct PoolKey {
    pub mesh_rid: Rid,
    pub layer_mask: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct PoolSlotId {
    pub key: PoolKey,
    pub slot: u32,
}

struct MeshPool {
    multimesh_rid: Rid,
    instance_rid: Rid,
    capacity: u32,
    high_water: u32,
    free_slots: BinaryHeap<u32>,
}

#[derive(Default)]
pub struct MeshPoolManager {
    pools: HashMap<PoolKey, MeshPool>,
}

impl MeshPoolManager {
    /// Allocate a slot for `key`, creating its MultiMesh on first use.
    /// `world` is the per-instance world transform pushed straight to the
    /// MultiMesh buffer.
    pub fn allocate(&mut self, key: PoolKey, scenario: Rid, world: Transform3D) -> PoolSlotId {
        let pool = self
            .pools
            .entry(key)
            .or_insert_with(|| create_pool(key, scenario));

        let slot = if let Some(s) = pool.free_slots.pop() {
            s
        } else {
            if pool.high_water >= pool.capacity {
                let new_cap = pool.capacity.saturating_mul(2).max(pool.capacity + 1);
                grow_pool(key, pool, new_cap);
            }
            let s = pool.high_water;
            pool.high_water += 1;
            s
        };

        let mut rs = RenderingServer::singleton();
        rs.multimesh_instance_set_transform(pool.multimesh_rid, slot as i32, world);
        rs.multimesh_set_visible_instances(pool.multimesh_rid, pool.high_water as i32);
        PoolSlotId { key, slot }
    }

    pub fn set_transform(&mut self, id: PoolSlotId, world: Transform3D) {
        if let Some(pool) = self.pools.get(&id.key) {
            let mut rs = RenderingServer::singleton();
            rs.multimesh_instance_set_transform(pool.multimesh_rid, id.slot as i32, world);
        }
    }

    /// Free the slot. Trims trailing zero-scaled slots so the GPU stops
    /// iterating them. Mid-buffer freed slots stay zero-scaled until reused.
    pub fn release(&mut self, id: PoolSlotId) {
        let Some(pool) = self.pools.get_mut(&id.key) else {
            return;
        };
        let mut rs = RenderingServer::singleton();
        rs.multimesh_instance_set_transform(
            pool.multimesh_rid,
            id.slot as i32,
            zero_scale_transform(),
        );
        pool.free_slots.push(id.slot);
        while let Some(&top) = pool.free_slots.peek() {
            if top + 1 != pool.high_water {
                break;
            }
            pool.free_slots.pop();
            pool.high_water = top;
        }
        rs.multimesh_set_visible_instances(pool.multimesh_rid, pool.high_water as i32);
    }

    /// Free all multimeshes (called on manager shutdown / scene teardown).
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

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    pub fn live_instance_count(&self) -> usize {
        self.pools
            .values()
            .map(|p| (p.high_water as usize) - p.free_slots.len())
            .sum()
    }
}

fn create_pool(key: PoolKey, scenario: Rid) -> MeshPool {
    let mut rs = RenderingServer::singleton();
    let multimesh = rs.multimesh_create();
    rs.multimesh_allocate_data_ex(
        multimesh,
        INITIAL_CAPACITY as i32,
        MultimeshTransformFormat::TRANSFORM_3D,
    )
    .done();
    rs.multimesh_set_mesh(multimesh, key.mesh_rid);
    rs.multimesh_set_visible_instances(multimesh, 0);

    let instance = rs.instance_create2(multimesh, scenario);
    rs.instance_set_transform(instance, Transform3D::IDENTITY);
    if key.layer_mask != 0 {
        rs.instance_set_layer_mask(instance, key.layer_mask);
    }

    MeshPool {
        multimesh_rid: multimesh,
        instance_rid: instance,
        capacity: INITIAL_CAPACITY,
        high_water: 0,
        free_slots: BinaryHeap::new(),
    }
}

fn grow_pool(key: PoolKey, pool: &mut MeshPool, new_capacity: u32) {
    let mut rs = RenderingServer::singleton();
    let new_mm = rs.multimesh_create();
    rs.multimesh_allocate_data_ex(
        new_mm,
        new_capacity as i32,
        MultimeshTransformFormat::TRANSFORM_3D,
    )
    .done();
    rs.multimesh_set_mesh(new_mm, key.mesh_rid);
    if pool.high_water > 0 {
        // Bulk-copy the live transform buffer (12 floats per instance) instead
        // of re-pushing every slot through GDExtension.
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
