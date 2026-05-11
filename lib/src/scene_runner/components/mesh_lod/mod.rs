//! Runtime mesh-LOD pass: replays each mergeable `MeshInstance3D`'s
//! source `ArrayMesh` through `ImporterMesh::generate_lods` so the
//! viewport can swap to lower-poly chains at distance.
//!
//! Hooks into the same post-load promotion pattern as the material atlas
//! and the textureless merger. Cached by source-mesh RID so two MIs
//! pointing at the same mesh share one LOD-baked output (preserves
//! Godot's auto-instancing).

mod classifier;
pub mod lod_baker;
mod metrics;
mod state;

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use godot::classes::{ArrayMesh, MeshInstance3D, Node3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::drain_global_stats;
pub use state::MeshLodState;

use classifier::Classification;

const MAX_ENTITIES_PER_FRAME: usize = 32;

struct GlobalCache {
    /// Source `ArrayMesh` RID → baked LOD'd `ArrayMesh`. Two MIs that
    /// reference the same source mesh resolve to the same baked handle,
    /// so Godot keeps batching their draws together.
    baked: HashMap<i64, Gd<ArrayMesh>>,
    /// Source `ArrayMesh` RID → baked shadow proxy. Reused across MIs
    /// the same way as `baked`. Currently unused (shadow_mesh assignment
    /// disabled — see comment in `process_entity`); retained for the
    /// re-enable path.
    #[allow(dead_code)]
    shadow: HashMap<i64, Gd<ArrayMesh>>,
}

thread_local! {
    static GLOBAL: RefCell<Option<GlobalCache>> = const { RefCell::new(None) };
}

fn with_global<R>(f: impl FnOnce(&mut GlobalCache) -> R) -> R {
    GLOBAL.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if borrow.is_none() {
            *borrow = Some(GlobalCache {
                baked: HashMap::new(),
                shadow: HashMap::new(),
            });
        }
        f(borrow.as_mut().expect("just initialized"))
    })
}

pub fn update_mesh_lod(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_mesh_lod.is_empty() {
            scene.pending_mesh_lod.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_mesh_lod
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
        scene.pending_mesh_lod.remove(entity);
    }

    scene.pending_mesh_lod.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().mesh_lod_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.mesh_lod.classified.insert(*entity) {
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
        let classification = classifier::classify(&mi, scene, *entity);
        scene.mesh_lod.stats.record(&classification);
        metrics::record_global(&classification);

        if !matches!(classification, Classification::Eligible) {
            continue;
        }

        let Some(mesh) = mi.get_mesh() else {
            continue;
        };
        let Ok(array_mesh) = mesh.try_cast::<ArrayMesh>() else {
            continue;
        };

        let source_rid = array_mesh.get_rid().to_u64() as i64;

        // Cache hit → reuse the baked mesh so identical MIs keep their
        // shared `mesh_rid` (and the auto-batching it enables).
        let cached = with_global(|g| g.baked.get(&source_rid).cloned());
        if let Some(cached_mesh) = cached {
            metrics::record_cache_hit();
            mi.set_mesh(&cached_mesh.upcast::<godot::classes::Mesh>());
            scene.mesh_lod.meshes_baked = scene.mesh_lod.meshes_baked.saturating_add(1);
            continue;
        }

        let Some(result) = lod_baker::bake_lods(&array_mesh) else {
            metrics::record_bake_failed();
            continue;
        };

        // Visible decimation: when the cli flag is on, use the
        // SurfaceTool-decimated ArrayMesh straight as `mi.mesh`. Skips
        // the ImporterMesh-based LOD chain entirely; `visible_prim`
        // drops to ~`1/STRIDE` of the source. Reuses the existing
        // shadow-proxy bake function — same producer, same reliability.
        let mut chosen_mesh = result.mesh.clone();
        if shadow_mesh_enabled() {
            let visible_decim = lod_baker::bake_shadow_mesh(&array_mesh);
            if let Some(sr) = visible_decim {
                chosen_mesh = sr.mesh;
            }
        }
        if shadow_mesh_enabled() {
            let shadow_cached = with_global(|g| g.shadow.get(&source_rid).cloned());
            let shadow_mesh = shadow_cached.or_else(|| {
                lod_baker::bake_shadow_mesh(&array_mesh).map(|sr| {
                    metrics::record_shadow_bake(sr.source_index_total, sr.shadow_index_total);
                    with_global(|g| g.shadow.insert(source_rid, sr.mesh.clone()));
                    sr.mesh
                })
            });
            if let Some(sm) = shadow_mesh {
                chosen_mesh.set_shadow_mesh(Some(&sm));
            }
        }
        with_global(|g| {
            g.baked.insert(source_rid, chosen_mesh.clone());
        });
        metrics::record_bake(result.source_index_total, result.lod0_index_total);
        mi.set_mesh(&chosen_mesh.upcast::<godot::classes::Mesh>());
        scene.mesh_lod.meshes_baked = scene.mesh_lod.meshes_baked.saturating_add(1);
    }
}

fn shadow_mesh_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().shadow_mesh_enabled)
        .unwrap_or(false)
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
