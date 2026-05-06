use std::collections::HashMap;

use godot::builtin::{Rid, Transform3D};
use godot::classes::{AnimationPlayer, INode, MeshInstance3D, Node, Node3D, Skeleton3D};
use godot::obj::InstanceId;
use godot::prelude::*;

use super::gltf_render::mesh_pool::{MeshPoolManager, PoolKey, PoolSlotId};
use super::gltf_render::promotion_tracker::{PromotionTracker, Tier};

type ContainerHandle = i64;

const REGISTER_FALLBACK: i64 = -1;

/// Minimum global instances of a `(mesh_rid, layer_mask)` key before we route
/// it through MultiMesh batching. Below this, the per-MultiMesh overhead and
/// the loss of Godot's internal MeshInstance3D batching outweigh the savings
/// — so we leave low-dup MIs alone and let the engine render them.
/// Calibrated against PR #1992 GP stats: 24 (21+ dups) + 100 (6-20) keys
/// cross this threshold; the other ~828 unique + ~238 (2-5 dup) keys stay
/// as legacy MIs.
const BATCH_THRESHOLD: usize = 6;

#[derive(Clone, Copy)]
enum SlotMode {
    /// MI is still in-tree, visible, rendering via Godot's normal pipeline.
    /// We track its `InstanceId` so we can hide+migrate it on promotion.
    Untouched(InstanceId),
    /// MI is hidden; its mesh renders via the MultiMesh pool slot.
    Pooled(PoolSlotId),
}

struct InstanceRecord {
    mode: SlotMode,
    /// Mesh pose relative to the container at register time. Static GLBs never
    /// change this; per-frame world for pooled records is `container.global *
    /// local`. Untouched records don't need world updates — Godot's scene
    /// graph keeps them in sync via parent transform inheritance.
    local_transform: Transform3D,
    /// Batch key — kept on the record so future promotion of this `(mesh,
    /// layer)` group doesn't need a tree walk.
    key: PoolKey,
}

struct ContainerEntry {
    container_id: InstanceId,
    instances: Vec<InstanceRecord>,
    /// Last container world transform we pushed to RS for pooled records.
    /// Without this, every frame would dirty every MultiMesh and force a
    /// full GPU re-upload (measured −48% FPS on Android GP without).
    last_world: Transform3D,
}

/// Per-key index back into `Untouched` instance records. Promotion walks this
/// to migrate every still-MI record into the MultiMesh pool, in one pass,
/// with no scene-tree traversal.
type UntouchedIndex = HashMap<PoolKey, Vec<(ContainerHandle, usize)>>;

/// Owns RenderingServer-direct rendering for SDK7 GltfContainers when
/// `--rs-gltf-direct` is on.
///
/// Step 4 v6 policy:
/// - Walk each loaded GLB's MeshInstance3D's; skip animated, skip MIs with
///   `surface_override_material`, skip invisible-collider MIs.
/// - Eligible MIs are tracked, but NOT migrated immediately — they keep
///   rendering through Godot's MeshInstance3D path so the engine's internal
///   batching (~1.15 MIs per draw on GP baseline) is preserved.
/// - When a global `(mesh_rid, layer_mask)` count crosses `BATCH_THRESHOLD`,
///   the manager hides every MI of that key and routes them all through one
///   shared MultiMesh batch.
/// - Future registrations of an already-batched key go straight to the pool.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclGltfRenderManager {
    base: Base<Node>,
    next_handle: i64,
    containers: HashMap<ContainerHandle, ContainerEntry>,
    pool: MeshPoolManager,
    promotion: PromotionTracker<PoolKey>,
    untouched_index: UntouchedIndex,
    scenario: Rid,
}

#[godot_api]
impl INode for DclGltfRenderManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            next_handle: 1,
            containers: HashMap::new(),
            pool: MeshPoolManager::default(),
            promotion: PromotionTracker::with_threshold(BATCH_THRESHOLD),
            untouched_index: UntouchedIndex::new(),
            scenario: Rid::Invalid,
        }
    }

    fn process(&mut self, _delta: f64) {
        if self.containers.is_empty() {
            return;
        }
        let mut dead: Vec<ContainerHandle> = Vec::new();
        let mut moved: Vec<(ContainerHandle, Transform3D)> = Vec::new();
        for (h, entry) in &self.containers {
            match Gd::<Node3D>::try_from_instance_id(entry.container_id) {
                Ok(n) => {
                    let current = n.get_global_transform();
                    if current != entry.last_world {
                        moved.push((*h, current));
                    }
                }
                Err(_) => dead.push(*h),
            }
        }
        for h in dead {
            self.unregister_container_internal(h);
        }
        for (h, container_world) in moved {
            // Only pooled records need an explicit transform push; Godot keeps
            // Untouched MIs aligned via standard parent-transform inheritance.
            let updates: Vec<(PoolSlotId, Transform3D)> = match self.containers.get(&h) {
                Some(entry) => entry
                    .instances
                    .iter()
                    .filter_map(|r| match r.mode {
                        SlotMode::Pooled(slot) => Some((slot, container_world * r.local_transform)),
                        SlotMode::Untouched(_) => None,
                    })
                    .collect(),
                None => continue,
            };
            for (slot, world) in updates {
                self.pool.set_transform(slot, world);
            }
            if let Some(entry) = self.containers.get_mut(&h) {
                entry.last_world = container_world;
            }
        }
    }
}

#[godot_api]
impl DclGltfRenderManager {
    #[func]
    fn register_static_container(
        &mut self,
        container: Gd<Node3D>,
        gltf_node: Gd<Node3D>,
        visible_mask: u32,
        invisible_mask: u32,
    ) -> i64 {
        if has_skeleton_or_anim(gltf_node.clone().upcast::<Node>()) {
            return REGISTER_FALLBACK;
        }
        if !self.ensure_scenario() {
            return REGISTER_FALLBACK;
        }

        let mut mi_list: Vec<Gd<MeshInstance3D>> = Vec::new();
        collect_mesh_instances(gltf_node.upcast::<Node>(), &mut mi_list);
        if mi_list.is_empty() {
            return REGISTER_FALLBACK;
        }

        let layer_mask = visible_mask | invisible_mask;
        let container_world = container.get_global_transform();
        let container_world_inv = container_world.affine_inverse();

        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if self.next_handle <= 0 {
            self.next_handle = 1;
        }

        let mut instances: Vec<InstanceRecord> = Vec::new();
        let mut keys_touched: Vec<PoolKey> = Vec::new();

        for mut mi in mi_list {
            // Invisible collider MIs (visible=false set by the GLB importer
            // at lib/src/content/gltf/scene.rs:100-107). Migrating would
            // expose collider geometry to rendering.
            if !mi.is_visible_in_tree() {
                continue;
            }
            let Some(mesh) = mi.get_mesh() else { continue };
            let mesh_rid = mesh.get_rid();
            if !mesh_rid.is_valid() {
                continue;
            }
            // surface_override_material → keep legacy. MultiMesh has no
            // per-instance override slot.
            let override_count = mi.get_surface_override_material_count();
            let has_override =
                (0..override_count).any(|i| mi.get_surface_override_material(i).is_some());
            if has_override {
                continue;
            }

            let mi_world = mi.get_global_transform();
            let local = container_world_inv * mi_world;
            let key = PoolKey {
                mesh_rid,
                layer_mask,
            };

            let tier = self.promotion.record_add(key);
            keys_touched.push(key);

            let mode = match tier {
                Tier::Singleton => {
                    // Below threshold (or not yet promoted): leave MI rendering
                    // via Godot. Track for possible future promotion.
                    SlotMode::Untouched(mi.instance_id())
                }
                Tier::Batched => {
                    // Already-batched key: hide MI, allocate pool slot.
                    let world = container_world * local;
                    let slot = self.pool.allocate(key, self.scenario, world);
                    mi.set_visible(false);
                    SlotMode::Pooled(slot)
                }
            };

            let idx = instances.len();
            instances.push(InstanceRecord {
                mode,
                local_transform: local,
                key,
            });
            if matches!(mode, SlotMode::Untouched(_)) {
                self.untouched_index
                    .entry(key)
                    .or_default()
                    .push((handle, idx));
            }
        }

        if instances.is_empty() {
            for k in &keys_touched {
                self.promotion.record_remove(k);
            }
            return REGISTER_FALLBACK;
        }

        self.containers.insert(
            handle,
            ContainerEntry {
                container_id: container.instance_id(),
                instances,
                last_world: container_world,
            },
        );

        // Promotion sweep: any key that just crossed `BATCH_THRESHOLD` gets
        // its still-Untouched MIs hidden and shifted into the pool, in one
        // pass.
        let to_promote: Vec<PoolKey> = keys_touched
            .into_iter()
            .filter(|k| self.promotion.should_promote(k))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        for key in to_promote {
            self.promote_key(key);
        }

        handle
    }

    #[func]
    fn unregister_container(&mut self, handle: i64) {
        self.unregister_container_internal(handle);
    }

    #[func]
    fn active_container_count(&self) -> i64 {
        self.containers.len() as i64
    }

    #[func]
    fn active_pool_count(&self) -> i64 {
        self.pool.pool_count() as i64
    }

    #[func]
    fn active_untouched_count(&self) -> i64 {
        self.containers
            .values()
            .flat_map(|e| e.instances.iter())
            .filter(|r| matches!(r.mode, SlotMode::Untouched(_)))
            .count() as i64
    }

    #[func]
    fn active_pooled_count(&self) -> i64 {
        self.containers
            .values()
            .flat_map(|e| e.instances.iter())
            .filter(|r| matches!(r.mode, SlotMode::Pooled(_)))
            .count() as i64
    }
}

impl DclGltfRenderManager {
    fn ensure_scenario(&mut self) -> bool {
        if self.scenario.is_valid() {
            return true;
        }
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let Some(world) = viewport.find_world_3d() else {
            return false;
        };
        self.scenario = world.get_scenario();
        self.scenario.is_valid()
    }

    /// Hide every still-Untouched MI under `key` and replace each with a
    /// MultiMesh pool slot. Drops the key from the untouched index.
    fn promote_key(&mut self, key: PoolKey) {
        let Some(entries) = self.untouched_index.remove(&key) else {
            return;
        };
        for (handle, idx) in entries {
            let Some(entry) = self.containers.get(&handle) else {
                continue;
            };
            let Some(record) = entry.instances.get(idx) else {
                continue;
            };
            let SlotMode::Untouched(mi_id) = record.mode else {
                continue;
            };
            let world = entry.last_world * record.local_transform;
            // Hide the MI before allocating, so we never have one frame of
            // double-render.
            if let Ok(mut mi) = Gd::<MeshInstance3D>::try_from_instance_id(mi_id) {
                mi.set_visible(false);
            }
            let new_slot = self.pool.allocate(key, self.scenario, world);
            if let Some(entry_mut) = self.containers.get_mut(&handle) {
                if let Some(rec_mut) = entry_mut.instances.get_mut(idx) {
                    rec_mut.mode = SlotMode::Pooled(new_slot);
                }
            }
        }
        self.promotion.mark_promoted(&key);
    }

    fn unregister_container_internal(&mut self, handle: ContainerHandle) {
        let Some(entry) = self.containers.remove(&handle) else {
            return;
        };
        for (idx, record) in entry.instances.iter().enumerate() {
            self.promotion.record_remove(&record.key);
            match record.mode {
                SlotMode::Untouched(_) => {
                    if let Some(list) = self.untouched_index.get_mut(&record.key) {
                        list.retain(|&(h, i)| !(h == handle && i == idx));
                        if list.is_empty() {
                            self.untouched_index.remove(&record.key);
                        }
                    }
                }
                SlotMode::Pooled(slot) => {
                    self.pool.release(slot);
                }
            }
        }
    }
}

fn has_skeleton_or_anim(node: Gd<Node>) -> bool {
    if node.clone().try_cast::<Skeleton3D>().is_ok() {
        return true;
    }
    if node.clone().try_cast::<AnimationPlayer>().is_ok() {
        return true;
    }
    let count = node.get_child_count();
    for i in 0..count {
        if let Some(child) = node.get_child(i) {
            if has_skeleton_or_anim(child) {
                return true;
            }
        }
    }
    false
}

fn collect_mesh_instances(node: Gd<Node>, out: &mut Vec<Gd<MeshInstance3D>>) {
    if let Ok(mi) = node.clone().try_cast::<MeshInstance3D>() {
        out.push(mi);
    }
    let count = node.get_child_count();
    for i in 0..count {
        if let Some(child) = node.get_child(i) {
            collect_mesh_instances(child, out);
        }
    }
}
