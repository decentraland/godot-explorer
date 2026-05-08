//! Material atlas: collapse N PBR-with-albedo-only materials onto a single
//! shared `ShaderMaterial` whose albedo is a `Texture2DArray`. Each source
//! mesh gets a clone with a `CUSTOM0` per-vertex layer id; the shader
//! samples the array using that index and looks up per-material params
//! from accompanying 1×N textures.
//!
//! The whole point is **identity sharing**: every mergeable MI ends up
//! with the same `Gd<ShaderMaterial>` handle, which is what Godot needs to
//! collapse them via internal MultiMesh instancing when their `mesh_rid`
//! also matches.

mod atlas;
mod classifier;
mod metrics;
mod state;
mod vertex_baker;

use std::cell::RefCell;
use std::time::Instant;

use godot::classes::{MeshInstance3D, Node3D, Shader, ShaderMaterial};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::dcl::crdt::SceneCrdtState;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::scene::Scene;

pub use metrics::{drain_global_stats, MergerStats};
pub use state::MaterialAtlasState;

use atlas::MaterialAtlas;
use classifier::Classification;

const MAX_ENTITIES_PER_FRAME: usize = 64;
const SHADER_PATH: &str = "res://assets/shaders/material_atlas.gdshader";

struct GlobalAtlas {
    atlas: MaterialAtlas,
    shader_material: Gd<ShaderMaterial>,
    /// Cache: `(source_mesh_rid, layer)` → baked clone. Two `MeshInstance3D`s
    /// pointing at the same source mesh and the same atlas layer share the
    /// SAME baked `ArrayMesh` handle, which is what Godot needs to keep
    /// auto-instancing those MIs into a single draw call.
    baked_cache: std::collections::HashMap<(i64, u32), Gd<godot::classes::ArrayMesh>>,
}

// Gd<T> is !Send, so a static Mutex won't compile. The scene runner update
// is single-threaded for Godot calls, so a thread_local is the right fit.
thread_local! {
    static GLOBAL: RefCell<Option<GlobalAtlas>> = const { RefCell::new(None) };
}

fn with_global<R>(f: impl FnOnce(&mut GlobalAtlas) -> R) -> Option<R> {
    GLOBAL.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if borrow.is_none() {
            let shader = godot::tools::try_load::<Shader>(SHADER_PATH).ok()?;
            let atlas = MaterialAtlas::new();
            let mut sm = ShaderMaterial::new_gd();
            sm.set_shader(&shader);
            sm.set_shader_parameter("albedo_atlas", &atlas.albedo_array.to_variant());
            sm.set_shader_parameter("material_params", &atlas.params_tex.to_variant());
            sm.set_shader_parameter("material_colors", &atlas.colors_tex.to_variant());
            sm.set_shader_parameter("layer_count", &(atlas.layer_count as i32).to_variant());
            *borrow = Some(GlobalAtlas {
                atlas,
                shader_material: sm,
                baked_cache: std::collections::HashMap::new(),
            });
        }
        let g = borrow.as_mut()?;
        Some(f(g))
    })
}

pub fn update_material_atlas(
    scene: &mut Scene,
    _crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    if !is_enabled() {
        if !scene.pending_material_atlas.is_empty() {
            scene.pending_material_atlas.clear();
        }
        return true;
    }

    let pending: Vec<SceneEntityId> = scene
        .pending_material_atlas
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
        scene.pending_material_atlas.remove(entity);
    }

    scene.pending_material_atlas.is_empty()
}

fn is_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().material_atlas_enabled)
        .unwrap_or(false)
}

fn process_entity(scene: &mut Scene, entity: &SceneEntityId) {
    if !scene.material_atlas.classified.insert(*entity) {
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
        scene.material_atlas.stats.record(&classification);
        metrics::record_global(&classification);

        if let Classification::Mergeable {
            albedo_texture,
            params,
            transparency: _,
            cull_mode: _,
        } = classification
        {
            let alloc = with_global(|g| g.atlas.allocate_layer(albedo_texture, params));
            let layer = match alloc.flatten() {
                Some(l) => l,
                None => {
                    metrics::record_atlas_full();
                    continue;
                }
            };
            metrics::record_layer_alloc();

            // The Texture2DArray RID never changes after `MaterialAtlas::new()` —
            // per-layer updates happen via `RenderingServer::texture_2d_update`.
            // We only need to bump the `layer_count` clamp uniform.
            let shared_material = with_global(|g| {
                g.shader_material.set_shader_parameter(
                    "layer_count",
                    &(g.atlas.layer_count as i32).to_variant(),
                );
                g.shader_material.clone()
            });

            let Some(source_mesh) = mi.get_mesh() else {
                continue;
            };
            let source_rid = source_mesh.get_rid().to_u64() as i64;

            // Cache hit: another MI already baked this (mesh, layer) pair —
            // reuse the baked clone so all consumers share one mesh_rid.
            // Cache miss: bake once and stash.
            let baked = with_global(|g| {
                if let Some(existing) = g.baked_cache.get(&(source_rid, layer)) {
                    return Some(existing.clone());
                }
                let baked = vertex_baker::bake_layer_into_custom0(&source_mesh, layer)?;
                g.baked_cache.insert((source_rid, layer), baked.clone());
                Some(baked)
            })
            .flatten();

            let Some(baked_mesh) = baked else {
                continue;
            };

            if let Some(shared_material) = shared_material {
                mi.set_mesh(&baked_mesh.upcast::<godot::classes::Mesh>());
                mi.set_surface_override_material(
                    0,
                    &shared_material.upcast::<godot::classes::Material>(),
                );
                metrics::record_mi_replace();
                scene.material_atlas.mis_replaced =
                    scene.material_atlas.mis_replaced.saturating_add(1);
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
