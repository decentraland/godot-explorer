//! Auto-generate `OccluderInstance3D` siblings for big opaque
//! `MeshInstance3D`s loaded by GLTF. Godot's culler tests potential
//! occluders against the frustum, then anything entirely behind the
//! occluder is skipped before fragment shading runs — pure perf,
//! fidelity-neutral.
//!
//! Only meshes with an AABB diagonal ≥ 5 m and reasonable proportions
//! get an occluder. The occluder box is shrunk 80 % of the AABB so a
//! player walking near a building's edge isn't false-culled.

mod classifier;
mod metrics;
mod state;

use std::time::Instant;

use godot::classes::{BoxOccluder3D, MeshInstance3D, Node3D, OccluderInstance3D};
use godot::obj::NewAlloc;
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::drain_global_stats;
pub use state::OccluderGenState;

use classifier::Classification;

const MAX_ENTITIES_PER_FRAME: usize = 32;
/// Inset factor — the BoxOccluder is shrunk by 1 - INSET on each axis so
/// objects close to the actual mesh boundary aren't false-culled.
const BOX_INSET: f32 = 0.8;

pub fn update_occluder_gen(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_occluder_gen.is_empty() {
            scene.pending_occluder_gen.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_occluder_gen
        .iter()
        .copied()
        .take(MAX_ENTITIES_PER_FRAME)
        .collect();

    let mut processed = 0usize;
    for entity in &pending {
        process_entity(scene, entity);
        processed += 1;

        let now_us = (Instant::now() - *ref_time).as_micros() as i64;
        if now_us > end_time_us {
            break;
        }
    }
    for entity in pending.iter().take(processed) {
        scene.pending_occluder_gen.remove(entity);
    }

    scene.pending_occluder_gen.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().occluder_gen_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.occluder_gen.classified.insert(*entity) {
        return;
    }

    let Some(node_3d) = scene.godot_dcl_scene.get_node_or_null_3d(entity).cloned() else {
        return;
    };

    let Some(gltf_container) = node_3d.try_get_node_as::<crate::godot_classes::dcl_gltf_container::DclGltfContainer>("GltfContainer") else {
        return;
    };

    let Some(gltf_root) = gltf_container.bind().get_gltf_resource() else {
        return;
    };

    let mut mesh_instances: Vec<Gd<MeshInstance3D>> = Vec::new();
    collect_mesh_instances(&gltf_root, &mut mesh_instances);

    for mi in mesh_instances {
        let classification = classifier::classify(&mi);
        scene.occluder_gen.stats.record(&classification);
        metrics::record_global(&classification);

        if !matches!(classification, Classification::Eligible) {
            continue;
        }

        let Some(mesh) = mi.get_mesh() else { continue };
        let aabb = mesh.get_aabb();

        let mut box_occluder = BoxOccluder3D::new_gd();
        box_occluder.set_size(aabb.size * BOX_INSET);

        let mut occluder_instance = OccluderInstance3D::new_alloc();
        occluder_instance.set_occluder(&box_occluder.upcast::<godot::classes::Occluder3D>());

        // Position the occluder at the AABB center in MI's local space.
        let mut t = godot::prelude::Transform3D::IDENTITY;
        t.origin = aabb.position + aabb.size * 0.5;
        occluder_instance.set_transform(t);

        // Parent under the MI itself so it's freed with the GLTF unload.
        let mut mi_parent = mi.clone();
        mi_parent.add_child(&occluder_instance.clone().upcast::<godot::classes::Node>());

        scene.occluder_gen.occluders_added =
            scene.occluder_gen.occluders_added.saturating_add(1);
        metrics::record_added(aabb.size.length());
    }
}

fn collect_mesh_instances(node: &Gd<Node3D>, out: &mut Vec<Gd<MeshInstance3D>>) {
    if let Ok(mi) = node.clone().try_cast::<MeshInstance3D>() {
        out.push(mi);
    }
    let child_count = node.get_child_count();
    for i in 0..child_count {
        if let Some(child) = node.get_child(i) {
            if let Ok(child_3d) = child.try_cast::<Node3D>() {
                collect_mesh_instances(&child_3d, out);
            }
        }
    }
}
