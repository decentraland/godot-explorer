//! Tweak `BaseMaterial3D` mode flags to a "cheap PBR" preset at GLTF
//! post-load. NO custom shader — just configure the existing
//! StandardMaterial3D to use cheaper math via its public mode setters.
//!
//! Changes per material (when applicable):
//! - `diffuse_mode = LAMBERT`  (was BURLEY): half-Lambert is ~2-3× cheaper
//!   in fragment ALU vs Burley's energy-conserving variant.
//! - `specular_mode = DISABLED` for non-metallic, low-roughness materials:
//!   GGX specular is the dominant per-fragment cost; killing it for matte
//!   surfaces is free perf.
//!
//! Universal across DCL scenes — every BaseMaterial3D loaded from GLTF
//! gets the cheaper preset. Default OFF; gated behind --cheap-pbr CLI
//! flag and the cheap-pbr deeplink param.

mod metrics;
mod state;

use std::time::Instant;

use godot::classes::base_material_3d::{DiffuseMode, SpecularMode, Transparency};
use godot::classes::{BaseMaterial3D, MeshInstance3D, Node, Node3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::drain_global_stats;
pub use state::CheapPbrState;

const MAX_ENTITIES_PER_FRAME: usize = 32;
/// Materials with metallic above this stay GGX — they need real specular
/// to look right (chrome, polished surfaces). Below this they go Phong-style.
const METALLIC_GGX_THRESHOLD: f32 = 0.1;
/// Materials with roughness above this skip specular entirely (matte paint,
/// fabric, dirt). Below this they keep specular but use cheaper diffuse.
const ROUGHNESS_NO_SPECULAR_THRESHOLD: f32 = 0.7;

pub fn update_cheap_pbr_materials(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_cheap_pbr.is_empty() {
            scene.pending_cheap_pbr.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_cheap_pbr
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
        scene.pending_cheap_pbr.remove(entity);
    }

    scene.pending_cheap_pbr.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().cheap_pbr_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.cheap_pbr.classified.insert(*entity) {
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

    for mi in mesh_instances {
        if has_avatar_ancestor(&mi) {
            metrics::record_skipped_avatar();
            continue;
        }
        if mi.get_layer_mask() != 1 {
            metrics::record_skipped_hud();
            continue;
        }

        let Some(mesh) = mi.get_mesh() else {
            continue;
        };
        let surface_count = mesh.get_surface_count();
        for s in 0..surface_count {
            let material = mi
                .get_active_material(s)
                .or_else(|| mi.get_surface_override_material(s))
                .or_else(|| mesh.surface_get_material(s));
            let Some(mat) = material else { continue };
            if mat.has_meta("dcl_cheap_pbr_applied") {
                metrics::record_skipped_already();
                continue;
            }
            let Ok(mut base) = mat.try_cast::<BaseMaterial3D>() else {
                metrics::record_skipped_not_base_mat();
                continue;
            };
            apply_cheap_preset(&mut base);
            base.set_meta("dcl_cheap_pbr_applied", &true.to_variant());
            scene.cheap_pbr.materials_tweaked = scene.cheap_pbr.materials_tweaked.saturating_add(1);
        }
    }
}

fn apply_cheap_preset(base: &mut Gd<BaseMaterial3D>) {
    // Always switch diffuse to LAMBERT — cheaper than the default BURLEY,
    // visually almost indistinguishable on opaque mobile content.
    base.set_diffuse_mode(DiffuseMode::LAMBERT);
    metrics::record_lambert();

    let metallic = base.get_metallic();
    let roughness = base.get_roughness();
    let transparency = base.get_transparency();

    // Transparent surfaces keep specular — visual feedback there matters
    // (glass, screens, signs).
    if transparency != Transparency::DISABLED {
        return;
    }

    // Disable specular on matte surfaces (low metallic + high roughness).
    // GGX specular is the dominant per-fragment cost on Mali.
    if metallic < METALLIC_GGX_THRESHOLD && roughness > ROUGHNESS_NO_SPECULAR_THRESHOLD {
        base.set_specular_mode(SpecularMode::DISABLED);
        metrics::record_specular_disabled();
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

fn has_avatar_ancestor(mi: &Gd<MeshInstance3D>) -> bool {
    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return true;
        }
        current = node.get_parent();
    }
    false
}
