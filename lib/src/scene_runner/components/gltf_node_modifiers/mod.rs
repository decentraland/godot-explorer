//! GLTF Node Modifiers component implementation.
//!
//! This module allows SDK scenes to dynamically modify materials and shadow casting
//! settings on specific nodes within loaded GLTF models at runtime.
//!
//! # Module Structure
//!
//! - `state`: State tracking structs for modifier state
//! - `path_matching`: Path matching logic for finding nodes
//! - `mesh_utils`: Mesh collection and traversal utilities
//! - `material`: Material creation and application

mod material;
mod mesh_utils;
mod path_matching;
mod state;

use std::collections::HashSet;
use std::time::Instant;

use godot::global::weakref;
use godot::prelude::*;

use crate::dcl::components::{SceneComponentId, SceneEntityId};
use crate::dcl::crdt::{
    last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState, SceneCrdtStateProtoComponents,
};
use crate::scene_runner::scene::Scene;

// Re-export public types
pub use material::{update_modifier_textures, update_modifier_video_textures};
pub use state::{GltfNodeModifierState, ModifierMaterialItem};

use material::{
    apply_material_to_mesh, apply_shadow_to_mesh, capture_original_materials,
    restore_original_materials,
};
use mesh_utils::{
    collect_all_mesh_instances, collect_paths_with_meshes, find_node_by_path, get_gltf_container,
};
use path_matching::{resolve_modifiers, ModifierKey};
use state::ModifierMaterialItem as MaterialItem;

/// Main update function for GLTF node modifiers.
/// Processes dirty entities and applies/removes modifiers.
pub fn update_gltf_node_modifiers(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    // Remove and consume dirty entities from CRDT (important: must use remove, not get/clone)
    let gltf_node_modifiers_dirty = scene
        .current_dirty
        .lww_components
        .remove(&SceneComponentId::GLTF_NODE_MODIFIERS);

    // Check if there are pending entities (without draining yet)
    let has_pending = !scene.gltf_node_modifiers_pending.is_empty();
    let has_dirty = gltf_node_modifiers_dirty
        .as_ref()
        .is_some_and(|d| !d.is_empty());

    // Early exit if nothing to do
    if !has_dirty && !has_pending {
        return true;
    }

    let mut updated_count = 0;
    let mut current_time_us;

    // Drain pending entities
    let pending_entities: Vec<SceneEntityId> = scene.gltf_node_modifiers_pending.drain().collect();

    let gltf_node_modifiers_component =
        SceneCrdtStateProtoComponents::get_gltf_node_modifiers(crdt_state);

    // Clone content_mapping reference before mutable borrows
    let content_mapping = scene.content_mapping.clone();

    // Combine both sources of entities to process
    let entities_to_process: Vec<SceneEntityId> = {
        let mut entities = pending_entities;
        if let Some(dirty) = &gltf_node_modifiers_dirty {
            for entity in dirty {
                if !entities.contains(entity) {
                    entities.push(*entity);
                }
            }
        }
        entities
    };

    if !entities_to_process.is_empty() {
        tracing::info!(
            "[GltfNodeModifier] Processing {} entities",
            entities_to_process.len()
        );
        for entity in entities_to_process.iter() {
            let new_value = gltf_node_modifiers_component.get(entity);

            // Skip entities that have no GltfNodeModifiers component AND no existing state
            // This avoids processing entities that never had the component
            let has_component = new_value
                .and_then(|v| v.value.as_ref())
                .is_some_and(|v| !v.modifiers.is_empty());
            let has_existing_state = scene.gltf_node_modifier_states.contains_key(entity);

            if !has_component && !has_existing_state {
                updated_count += 1;
                continue;
            }

            let godot_dcl_scene = &mut scene.godot_dcl_scene;
            let (_, node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            // Get the GltfContainer to access the loaded GLTF
            let Some(gltf_container) = get_gltf_container(&node_3d) else {
                // No GltfContainer on this entity - will be processed when GLTF finishes loading
                updated_count += 1;
                continue;
            };

            let Some(gltf_root) = gltf_container.bind().get_gltf_resource() else {
                // GLTF not loaded yet - will be processed when loading completes
                updated_count += 1;
                continue;
            };

            // Get or create state for this entity
            let state = scene.gltf_node_modifier_states.entry(*entity).or_default();

            // Handle component removal or empty modifiers
            let modifiers = new_value
                .and_then(|v| v.value.as_ref())
                .map(|v| v.modifiers.as_slice())
                .unwrap_or(&[]);

            if modifiers.is_empty() {
                // Component was removed or has no modifiers - restore all original states
                for (_path, mesh_instances) in collect_paths_with_meshes(&gltf_root) {
                    for (full_path, mut mesh) in mesh_instances {
                        if let Some(original_materials) = state.original_materials.get(&full_path) {
                            restore_original_materials(&mut mesh, original_materials);
                        }
                        if let Some(original_shadow) = state.original_shadows.get(&full_path) {
                            mesh.set_cast_shadows_setting(*original_shadow);
                        }
                    }
                }

                // Clear state
                state.original_materials.clear();
                state.original_shadows.clear();
                state.applied_paths.clear();

                updated_count += 1;
                current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
                if current_time_us > end_time_us {
                    break;
                }
                continue;
            }

            // Collect all mesh instances in the GLTF
            let all_meshes = collect_all_mesh_instances(&gltf_root);

            // Resolve which modifier applies to each mesh (and optionally which surface)
            let resolved = resolve_modifiers(modifiers, &all_meshes);

            // Track which paths are now being modified (using node_path for simplicity)
            let new_applied_paths: HashSet<String> =
                resolved.keys().map(|k| k.node_path.clone()).collect();

            // Find paths that were previously modified but no longer have modifiers
            let paths_to_restore: Vec<String> = state
                .applied_paths
                .difference(&new_applied_paths)
                .cloned()
                .collect();

            // Restore paths that no longer have modifiers
            for path in paths_to_restore {
                if let Some(node) = find_node_by_path(&gltf_root, &path) {
                    if let Ok(mut mesh) = node.try_cast::<godot::classes::MeshInstance3D>() {
                        if let Some(original_materials) = state.original_materials.get(&path) {
                            restore_original_materials(&mut mesh, original_materials);
                        }
                        if let Some(original_shadow) = state.original_shadows.get(&path) {
                            mesh.set_cast_shadows_setting(*original_shadow);
                        }
                    }
                }
            }

            // Apply modifiers to each mesh
            tracing::info!(
                "[GltfNodeModifier] Entity {:?}: applying modifiers to {} meshes",
                entity,
                all_meshes.len()
            );
            for info in all_meshes {
                let path = &info.node_path;
                let mut mesh = info.mesh_instance;

                // Capture original state if not already captured (before applying any modifiers)
                if !state.original_materials.contains_key(path) {
                    state
                        .original_materials
                        .insert(path.clone(), capture_original_materials(&mesh));
                    state
                        .original_shadows
                        .insert(path.clone(), mesh.get_cast_shadows_setting());
                }

                // Check for "all surfaces" modifier first
                let all_surfaces_key = ModifierKey::all_surfaces(path);
                if let Some(modifier_match) = resolved.get(&all_surfaces_key) {
                    // Apply shadow casting if specified
                    if let Some(cast_shadows) = modifier_match.modifier.cast_shadows {
                        apply_shadow_to_mesh(&mut mesh, cast_shadows, None);
                    }

                    // Apply material if specified (to all surfaces)
                    if let Some(material) = &modifier_match.modifier.material {
                        if let Some((dcl_material, godot_material)) =
                            apply_material_to_mesh(&mut mesh, material, &content_mapping, None)
                        {
                            // Track material for texture loading
                            state.pending_materials.insert(
                                path.clone(),
                                MaterialItem {
                                    dcl_material,
                                    weak_ref: weakref(&godot_material.to_variant()),
                                    waiting_textures: true,
                                },
                            );
                        }
                    }
                }

                // Check for per-surface modifiers (can override "all surfaces" for specific surfaces)
                for surface_idx in 0..info.surface_count {
                    let surface_key = ModifierKey::new(path, Some(surface_idx));
                    if let Some(modifier_match) = resolved.get(&surface_key) {
                        // Per-surface shadow is applied to whole mesh (shadows are mesh-level)
                        if let Some(cast_shadows) = modifier_match.modifier.cast_shadows {
                            apply_shadow_to_mesh(&mut mesh, cast_shadows, Some(surface_idx));
                        }

                        // Apply material to this specific surface
                        if let Some(material) = &modifier_match.modifier.material {
                            if let Some((dcl_material, godot_material)) = apply_material_to_mesh(
                                &mut mesh,
                                material,
                                &content_mapping,
                                Some(surface_idx),
                            ) {
                                // Track material for texture loading with surface-specific key
                                let material_key = format!("{}:{}", path, surface_idx);
                                state.pending_materials.insert(
                                    material_key,
                                    MaterialItem {
                                        dcl_material,
                                        weak_ref: weakref(&godot_material.to_variant()),
                                        waiting_textures: true,
                                    },
                                );
                            }
                        }
                    }
                }
            }

            // Update applied paths
            state.applied_paths = new_applied_paths;

            updated_count += 1;
            current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
            if current_time_us > end_time_us {
                break;
            }
        }

        // If we didn't process all entities, re-add remaining to pending set
        if updated_count < entities_to_process.len() {
            let remaining: Vec<_> = entities_to_process.iter().skip(updated_count).collect();
            tracing::debug!(
                "Time budget exceeded: processed {}/{} entities, re-adding {:?} to pending",
                updated_count,
                entities_to_process.len(),
                remaining
            );
            for entity in remaining {
                scene.gltf_node_modifiers_pending.insert(*entity);
            }
            return false;
        } else {
            tracing::debug!(
                "Finished processing all {} entities",
                entities_to_process.len()
            );
        }
    }

    true
}
