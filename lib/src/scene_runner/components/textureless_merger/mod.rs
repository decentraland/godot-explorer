//! Textureless mesh merger.
//!
//! Per-frame flow:
//! 1. Drain `pending_textureless_promotion` (cap MAX_ENTITIES_PER_FRAME).
//! 2. For each entity: walk GLTF tree, classify each MeshInstance3D, route
//!    `Mergeable` parts into the matching `(transparency, cull_mode, cell)`
//!    bucket.
//! 3. Flush buckets that have `≥ MIN_BUCKET_SIZE` parts: build a combined
//!    ArrayMesh, spawn one MeshInstance3D under the scene's root_node_3d,
//!    queue the source MIs for suppression next frame.
//! 4. Suppress queued originals (`mesh = null`) — the only API that drops
//!    the draw counter.
//!
//! The classifier excludes entities with active Tween / Modifier / Skeleton
//! / AnimationPlayer / DclAvatar ancestor / blend shapes / shader material
//! / textures. No demote logic — entities that gain a Tween after being
//! merged stay merged.
//!
//! Design doc: `docs/bench/material-atlas-mesh-merge-design.md`.

mod cell_grid;
mod classifier;
mod combiner;
mod metrics;
mod state;

use std::time::Instant;

use godot::classes::{MeshInstance3D, Node, Node3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::{drain_global_stats, snapshot_global_stats, MergerStats};
pub use state::TexturelessMergerState;

use cell_grid::{cell_center, cell_for, BucketKey, MIN_BUCKET_SIZE};
use classifier::Classification;
use combiner::{build_merged_mesh, MeshPart};

/// Cap on entities promoted per-frame so the classifier never busts the
/// scene_runner time budget on a fresh load (every other handler does the
/// same — see `update_gltf_node_modifiers`).
const MAX_ENTITIES_PER_FRAME: usize = 64;

pub fn update_textureless_merger(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    let enabled = is_enabled();
    if !enabled {
        // Silent drain: the promotion queue keeps re-filling every CRDT tick
        // even when merger is off (scene_runner pushes entities without a
        // gate). Logging each clear spams the console at ~30 lines/sec.
        scene.pending_textureless_promotion.clear();
        return true;
    }
    if !scene.pending_textureless_promotion.is_empty() {
        tracing::info!(
            target: "textureless_merger",
            "tick: pending={} buckets={} merged_nodes={}",
            scene.pending_textureless_promotion.len(),
            scene.textureless_merger.cell_buckets.len(),
            scene.textureless_merger.merged_nodes.len(),
        );
    }

    // Step 1: suppress originals queued from the previous frame's flush.
    // Doing this BEFORE classifying new entities matters: the merged node
    // is already up, so nulling sources leaves the scene visually intact
    // (no one-frame gap).
    suppress_pending(scene);

    // Step 2: classify new entities and route mergeable parts into buckets.
    let pending: Vec<SceneEntityId> = scene
        .pending_textureless_promotion
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
        scene.pending_textureless_promotion.remove(entity);
    }

    // Step 3: flush buckets that crossed the threshold.
    flush_ready_buckets(scene);

    scene.pending_textureless_promotion.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().textureless_merge_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.textureless_merger.classified.insert(*entity) {
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

    for mi in &mesh_instances {
        let classification = classifier::classify(mi, scene, *entity);
        scene.textureless_merger.stats.record(&classification);
        metrics::record_global(&classification);

        if let Classification::Mergeable {
            albedo_color,
            transparency,
            cull_mode,
        } = classification
        {
            let world_transform = mi.get_global_transform();
            let (cx, cz) = cell_for(world_transform.origin);
            let key = BucketKey {
                transparency,
                cull_mode,
                cx,
                cz,
            };
            let bucket = scene
                .textureless_merger
                .cell_buckets
                .entry(key)
                .or_default();
            // Don't accept new parts into a bucket that's already flushed —
            // those become orphans for now (rendered standalone).
            if bucket.flushed {
                continue;
            }
            bucket.parts.push(MeshPart {
                source_mi: mi.clone(),
                albedo_color,
                world_transform,
            });
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

fn flush_ready_buckets(scene: &mut Scene) {
    let ready_keys: Vec<BucketKey> = scene
        .textureless_merger
        .cell_buckets
        .iter()
        .filter_map(|(k, b)| {
            if !b.flushed && b.parts.len() >= MIN_BUCKET_SIZE {
                Some(*k)
            } else {
                None
            }
        })
        .collect();

    if ready_keys.is_empty() {
        return;
    }

    // Take ownership of the scene root once — we're about to add children
    // and need a stable Gd<Node3D> handle.
    let mut root_node = scene
        .godot_dcl_scene
        .root_node_3d
        .clone()
        .upcast::<Node3D>();

    for key in ready_keys {
        let bucket = match scene.textureless_merger.cell_buckets.get_mut(&key) {
            Some(b) => b,
            None => continue,
        };
        let parts = std::mem::take(&mut bucket.parts);
        bucket.flushed = true;

        let Some(built) =
            build_merged_mesh(&parts, (key.cx, key.cz), key.transparency, key.cull_mode)
        else {
            continue;
        };

        let mut merged_mi = MeshInstance3D::new_alloc();
        merged_mi.set_name(&format!(
            "_tm_merged_{}_{}_{}_{}",
            key.transparency, key.cull_mode, key.cx, key.cz
        ));
        merged_mi.set_mesh(&built.mesh.upcast::<godot::classes::Mesh>());
        merged_mi.set_surface_override_material(0, &built.material);
        merged_mi.set_position(cell_center(key.cx, key.cz));

        root_node.add_child(&merged_mi.clone().upcast::<Node>());

        tracing::info!(
            target: "textureless_merger",
            "flush bucket transp={} cull={} cell=({},{}) parts={} verts={} idx={}",
            key.transparency,
            key.cull_mode,
            key.cx,
            key.cz,
            parts.len(),
            built.vertex_count,
            built.index_count,
        );

        scene.textureless_merger.merged_nodes.insert(key, merged_mi);

        scene.textureless_merger.merged_vertex_total = scene
            .textureless_merger
            .merged_vertex_total
            .saturating_add(built.vertex_count as u64);
        scene.textureless_merger.buckets_flushed =
            scene.textureless_merger.buckets_flushed.saturating_add(1);

        // Queue source originals for suppression next frame.
        for part in parts {
            scene
                .textureless_merger
                .pending_suppress
                .push(part.source_mi);
        }
    }
}

fn suppress_pending(scene: &mut Scene) {
    if scene.textureless_merger.pending_suppress.is_empty() {
        return;
    }
    let queue = std::mem::take(&mut scene.textureless_merger.pending_suppress);
    let count = queue.len();
    for mut mi in queue {
        // `mesh = null` is the only suppression that sticks in the draw
        // counter (validated by the GDScript prototype runs tm-c5 → tm-mn2).
        mi.set_mesh(Gd::<godot::classes::Mesh>::null_arg());
    }
    scene.textureless_merger.originals_suppressed = scene
        .textureless_merger
        .originals_suppressed
        .saturating_add(count as u32);
}
