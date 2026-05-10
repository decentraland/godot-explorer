//! Aggressive offline-style asset preprocessing run at GLTF post-load.
//!
//! Designed as the client-side first pass; the same pipeline will move
//! server-side once stable. The output is a transformed scene tree with:
//!  - Decimated meshes (target much lower than runtime mesh_lod allows)
//!  - Stripped vertex streams (UV2, tangents, colors when unused)
//!  - Mesh-shaped occluders (`ArrayOccluder3D`) for big opaques
//!
//! Each stage is gated by its own bool so we can A/B test individually.
//! The whole module is gated by `--asset-preproc` (default OFF).

mod decimator;
mod mesh_occluder;
mod metrics;
mod state;
mod vertex_strip;

use std::time::Instant;

use godot::classes::{MeshInstance3D, Node3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::drain_global_stats;
pub use state::AssetPreprocessorState;

const MAX_ENTITIES_PER_FRAME: usize = 16;

pub fn update_asset_preprocessor(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_asset_preprocessor.is_empty() {
            scene.pending_asset_preprocessor.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_asset_preprocessor
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
        scene.pending_asset_preprocessor.remove(entity);
    }

    scene.pending_asset_preprocessor.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().asset_preproc_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.asset_preprocessor.classified.insert(*entity) {
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
        // Skip avatars / HUD / invisible — same rules as the other modules.
        if !mi.is_visible_in_tree() {
            continue;
        }
        if mi.get_layer_mask() != 1 {
            continue;
        }
        if has_avatar_ancestor(&mi) {
            continue;
        }

        let Some(mesh) = mi.get_mesh() else { continue };
        let Ok(array_mesh) = mesh.try_cast::<godot::classes::ArrayMesh>() else {
            continue;
        };

        // 1. Aggressive decimation. Replaces `mi.mesh` with a much-decimated
        //    ArrayMesh whose LOD0 ≈ runtime LOD2 (~25% triangles).
        if let Some(decimated) = decimator::aggressive_decimate(&array_mesh) {
            metrics::record_decimated(decimated.source_idx, decimated.target_idx);
            mi.set_mesh(&decimated.mesh.upcast::<godot::classes::Mesh>());
            scene.asset_preprocessor.meshes_decimated =
                scene.asset_preprocessor.meshes_decimated.saturating_add(1);
        }

        // 2. Strip unused vertex streams (UV2, tangents if no normal-map use,
        //    vertex colors if all white, bones if not skinned).
        if let Some(refreshed) = mi.get_mesh() {
            if let Ok(am) = refreshed.try_cast::<godot::classes::ArrayMesh>() {
                if let Some((stripped, bytes)) = vertex_strip::strip_unused(&am) {
                    metrics::record_stripped(bytes);
                    mi.set_mesh(&stripped.upcast::<godot::classes::Mesh>());
                    scene.asset_preprocessor.meshes_stripped =
                        scene.asset_preprocessor.meshes_stripped.saturating_add(1);
                }
            }
        }

        // 3. Spawn a mesh-shaped occluder for big opaque meshes. Skips small
        //    or transparent ones internally.
        if let Some(refreshed) = mi.get_mesh() {
            if let Ok(am) = refreshed.try_cast::<godot::classes::ArrayMesh>() {
                if mesh_occluder::try_spawn_for(&mut mi, &am) {
                    metrics::record_occluder();
                    scene.asset_preprocessor.occluders_added =
                        scene.asset_preprocessor.occluders_added.saturating_add(1);
                }
            }
        }
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
    let mut current: Option<Gd<godot::classes::Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node
            .clone()
            .try_cast::<crate::godot_classes::dcl_avatar::DclAvatar>()
            .is_ok()
        {
            return true;
        }
        current = node.get_parent();
    }
    false
}
