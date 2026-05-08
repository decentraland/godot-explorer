//! Auto-set `visibility_range_end` on every loaded MeshInstance3D so the
//! engine's frustum culler skips drawing small props before fragment work
//! happens. The cutoff is derived from the mesh's AABB diagonal — a 1 m
//! cube hides past 30 m (invisible at < 1 px), a 30 m building stays
//! visible to 200 m+.
//!
//! Pure perf: meshes that get culled were already too small to perceive
//! at the cutoff distance.

mod classifier;
mod metrics;
mod state;

use std::time::Instant;

use godot::classes::{MeshInstance3D, Node3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::drain_global_stats;
pub use state::AutoDistanceCullState;

use classifier::Classification;

const MAX_ENTITIES_PER_FRAME: usize = 64;

/// `visibility_range_end_margin` (in meters) — Godot fades out from
/// `end - margin` to `end`. Setting margin > 0 avoids the hard pop-in;
/// 4 m is wide enough to be unnoticeable and tight enough to keep the
/// per-pixel savings on the cull side.
const FADE_MARGIN_M: f32 = 4.0;

/// AABB-diagonal-to-distance multiplier. A 1 m mesh fades at 30 m; a 10 m
/// mesh at 200 m. Values tuned so the projected screen size at the cutoff
/// is about 1–2 pixels on a 1080p display.
const DIAG_TO_END_RATIO: f32 = 30.0;
const MIN_END_M: f32 = 30.0;
const MAX_END_M: f32 = 250.0;

pub fn update_auto_distance_cull(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_auto_distance_cull.is_empty() {
            scene.pending_auto_distance_cull.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_auto_distance_cull
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
        scene.pending_auto_distance_cull.remove(entity);
    }

    scene.pending_auto_distance_cull.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().auto_distance_cull_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.auto_distance_cull.classified.insert(*entity) {
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

    for mut mi in mesh_instances {
        let classification = classifier::classify(&mi);
        scene.auto_distance_cull.stats.record(&classification);
        metrics::record_global(&classification);

        if !matches!(classification, Classification::Eligible) {
            continue;
        }

        let Some(mesh) = mi.get_mesh() else { continue };
        let aabb = mesh.get_aabb();
        let size = aabb.size;
        let diag = (size.x * size.x + size.y * size.y + size.z * size.z).sqrt();

        let end = (diag * DIAG_TO_END_RATIO).clamp(MIN_END_M, MAX_END_M);

        // `visibility_range_begin/end` lives on `GeometryInstance3D` (parent
        // of MeshInstance3D). Setting `end` enables fade-out culling; the
        // margin range starts the alpha fade so there's no pop.
        mi.set_visibility_range_end(end);
        mi.set_visibility_range_end_margin(FADE_MARGIN_M);
        mi.set_visibility_range_fade_mode(
            godot::classes::geometry_instance_3d::VisibilityRangeFadeMode::SELF,
        );

        scene.auto_distance_cull.mis_set = scene.auto_distance_cull.mis_set.saturating_add(1);
        metrics::record_set(end);
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
