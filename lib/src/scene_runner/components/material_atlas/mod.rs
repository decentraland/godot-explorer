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
    /// Cache: `(source_mesh_rid, layer_vec)` → combined-surface mesh. Multi-
    /// surface MIs collapse their N surfaces into a single surface, with each
    /// source surface's atlas layer baked into CUSTOM0 per-vertex. The key
    /// includes the layer vector because two MIs with identical source mesh
    /// but distinct material→layer fan-outs would produce different bakes.
    combined_cache: std::collections::HashMap<(i64, Vec<u32>), Gd<godot::classes::ArrayMesh>>,
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
                combined_cache: std::collections::HashMap::new(),
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
        let per_surface = classifier::classify_per_surface(&mi, scene, *entity);
        // Stats/metrics: walk every classification. Single-element vecs from
        // mi_level_skip get counted once; multi-element vecs (one per surface)
        // count per surface.
        for c in &per_surface {
            scene.material_atlas.stats.record(c);
            metrics::record_global(c);
        }

        // Combine path: every surface must be mergeable to collapse them into
        // one draw call. If any surface isn't, keep the MI intact (legacy
        // path also bailed on multi-surface meshes via SkipReason::MultiSurface,
        // so this is no worse for those cases and strictly better for the
        // all-mergeable case which was previously rejected outright).
        let mut layers: Vec<u32> = Vec::with_capacity(per_surface.len());
        let mut all_mergeable = !per_surface.is_empty();
        for c in &per_surface {
            match c {
                Classification::Mergeable {
                    albedo_texture,
                    params,
                    ..
                } => {
                    let alloc =
                        with_global(|g| g.atlas.allocate_layer(albedo_texture.clone(), *params));
                    match alloc.flatten() {
                        Some(l) => {
                            metrics::record_layer_alloc();
                            layers.push(l);
                        }
                        None => {
                            metrics::record_atlas_full();
                            all_mergeable = false;
                            break;
                        }
                    }
                }
                Classification::Skip(_) => {
                    all_mergeable = false;
                    break;
                }
            }
        }
        if !all_mergeable {
            continue;
        }

        // Bump layer_count uniform now that we've allocated more layers.
        let shared_material = with_global(|g| {
            g.shader_material
                .set_shader_parameter("layer_count", &(g.atlas.layer_count as i32).to_variant());
            g.shader_material.clone()
        });

        let Some(source_mesh) = mi.get_mesh() else {
            continue;
        };
        let source_rid = source_mesh.get_rid().to_u64() as i64;
        // Cache key includes the layer assignment so two MIs with the same
        // source mesh but distinct layer fan-outs don't collide.
        let cache_key: Vec<u32> = layers.clone();
        let baked = with_global(|g| {
            if let Some(existing) = g.combined_cache.get(&(source_rid, cache_key.clone())) {
                return Some(existing.clone());
            }
            let combined = vertex_baker::bake_combined_surfaces(&source_mesh, &cache_key)?;
            g.combined_cache
                .insert((source_rid, cache_key), combined.clone());
            Some(combined)
        })
        .flatten();
        let Some(combined_mesh) = baked else {
            continue;
        };

        if let Some(shared_material) = shared_material {
            mi.set_mesh(&combined_mesh.upcast::<godot::classes::Mesh>());
            mi.set_surface_override_material(
                0,
                &shared_material.upcast::<godot::classes::Material>(),
            );
            metrics::record_mi_replace();
            scene.material_atlas.mis_replaced = scene.material_atlas.mis_replaced.saturating_add(1);
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
