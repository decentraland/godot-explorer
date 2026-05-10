//! Disable shadow casting on small / decoration / distant-only meshes.
//!
//! Per-pass instrumentation (see godot/src/tools/gp_benchmark_runner.gd's
//! `_collect_sample`) showed Genesis Plaza's shadow pass renders ~955k
//! primitives — more than the visible pass (448k) — because directional
//! shadow cascades draw every shadow caster at LOD0. Most of those are
//! small props whose shadow is < 1 px on screen.
//!
//! At GLTF post-load we walk MeshInstance3Ds, and any whose AABB diagonal
//! is below `MIN_SHADOW_DIAG_M` (2 m) gets `cast_shadow=OFF`. They still
//! render in the visible pass; only the shadow-pass overhead disappears.
//!
//! Pure perf, fidelity-neutral: a 1 m prop at typical eye level projects a
//! shadow whose long axis is ~1 m on the ground; at 5+ m view distance it
//! occupies < 2 px of the shadow map — invisible vs ambient occlusion.

mod metrics;
mod state;

use std::time::Instant;

use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::{MeshInstance3D, Node, Node3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::drain_global_stats;
pub use state::AutoShadowCullState;

const MAX_ENTITIES_PER_FRAME: usize = 64;
/// Below this AABB diagonal (meters), the mesh's shadow is too small to
/// notice. 2 m kept after 4 m experiment: at 4 m the cull set includes
/// 1785 meshes (-305 shadow draws) but fps regressed -2 vs off — at 4 m
/// we start culling medium props whose shadows ARE visible at 5-15 m.
/// 2 m is the safe sweet spot.
pub const MIN_SHADOW_DIAG_M: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    NoMesh,
    NotVisible,
    AvatarAncestor,
    HudOrUi,
    AlreadyOff,
    LargeEnough,
}

#[derive(Debug, Clone, Copy)]
pub enum Classification {
    Cull,
    Skip(SkipReason),
}

pub fn update_auto_shadow_cull(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_auto_shadow_cull.is_empty() {
            scene.pending_auto_shadow_cull.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_auto_shadow_cull
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
        scene.pending_auto_shadow_cull.remove(entity);
    }

    scene.pending_auto_shadow_cull.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().auto_shadow_cull_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.auto_shadow_cull.classified.insert(*entity) {
        return;
    }

    let Some(node_3d) = scene.godot_dcl_scene.get_node_or_null_3d(entity).cloned() else {
        return;
    };

    let Some(gltf_container) = node_3d
        .try_get_node_as::<crate::godot_classes::dcl_gltf_container::DclGltfContainer>(
            "GltfContainer",
        )
    else {
        return;
    };

    let Some(gltf_root) = gltf_container.bind().get_gltf_resource() else {
        return;
    };

    let mut mesh_instances: Vec<Gd<MeshInstance3D>> = Vec::new();
    collect_mesh_instances(&gltf_root, &mut mesh_instances);

    for mut mi in mesh_instances {
        let classification = classify(&mi);
        scene.auto_shadow_cull.stats.record(&classification);
        metrics::record_global(&classification);

        if matches!(classification, Classification::Cull) {
            mi.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            scene.auto_shadow_cull.shadows_disabled =
                scene.auto_shadow_cull.shadows_disabled.saturating_add(1);
            metrics::record_disabled();
        }
    }
}

fn classify(mi: &Gd<MeshInstance3D>) -> Classification {
    if !mi.is_visible_in_tree() {
        return Classification::Skip(SkipReason::NotVisible);
    }

    if mi.get_layer_mask() != 1 {
        return Classification::Skip(SkipReason::HudOrUi);
    }

    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return Classification::Skip(SkipReason::AvatarAncestor);
        }
        current = node.get_parent();
    }

    if mi.get_cast_shadows_setting() == ShadowCastingSetting::OFF {
        return Classification::Skip(SkipReason::AlreadyOff);
    }

    let Some(mesh) = mi.get_mesh() else {
        return Classification::Skip(SkipReason::NoMesh);
    };

    let aabb = mesh.get_aabb();
    let diag =
        (aabb.size.x * aabb.size.x + aabb.size.y * aabb.size.y + aabb.size.z * aabb.size.z).sqrt();

    if diag >= MIN_SHADOW_DIAG_M {
        return Classification::Skip(SkipReason::LargeEnough);
    }

    Classification::Cull
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
