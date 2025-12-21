/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! GLTF loading and processing for content_v2
//!
//! This module handles:
//! - Downloading GLTF files and their dependencies
//! - Loading GLTF into Godot Node3D
//! - Post-processing textures
//! - Creating and configuring colliders
//! - Saving the processed scene to disk

use std::sync::Arc;

use godot::{
    builtin::{meta::ToGodot, Dictionary, GString},
    engine::{
        base_material_3d::TextureParam, node::ProcessMode, AnimatableBody3D, BaseMaterial3D,
        CollisionShape3D, ConcavePolygonShape3D, GltfDocument, GltfState, ImageTexture,
        MeshInstance3D, Node, Node3D,
    },
    obj::{EngineEnum, Gd, NewAlloc, NewGd},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::content::{
    content_mapping::ContentMappingAndUrlRef, file_string::get_base_dir,
    texture::create_compressed_texture, texture::resize_image,
};

use super::content_provider::ContentProvider2Context;
use super::scene_saver::{ensure_glbs_directory, get_scene_path_for_hash, save_node_as_scene};

/// Thread safety guard for Godot API access
pub struct GodotThreadSafetyGuard {
    _guard: tokio::sync::OwnedSemaphorePermit,
}

impl GodotThreadSafetyGuard {
    pub async fn acquire(ctx: &ContentProvider2Context) -> Option<Self> {
        let guard = ctx.godot_single_thread.clone().acquire_owned().await.ok()?;
        set_thread_safety_checks_enabled(false);
        Some(Self { _guard: guard })
    }
}

impl Drop for GodotThreadSafetyGuard {
    fn drop(&mut self) {
        set_thread_safety_checks_enabled(true);
    }
}

fn set_thread_safety_checks_enabled(enabled: bool) {
    let mut temp_script =
        godot::engine::load::<godot::engine::Script>("res://src/logic/thread_safety.gd");
    temp_script.call(
        "set_thread_safety_checks_enabled".into(),
        &[enabled.to_variant()],
    );
}

/// Main entry point for loading and processing a GLTF file
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Creates colliders (with mask=0 - caller sets masks after instantiating)
/// 5. Saves the processed scene to disk
///
/// Returns the path to the saved scene file on success
pub async fn load_and_save_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProvider2Context,
) -> Result<String, anyhow::Error> {
    // Ensure the output directory exists
    let _dir = ensure_glbs_directory().map_err(anyhow::Error::msg)?;

    // Download the main GLTF file
    let base_path = Arc::new(get_base_dir(&file_path));
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    ctx.resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    // Get dependencies from the GLTF file
    let dependencies = get_dependencies(&absolute_file_path)
        .await?
        .into_iter()
        .map(|dep| {
            let full_path = if base_path.is_empty() {
                dep.clone()
            } else {
                format!("{}/{}", base_path, dep)
            };
            let item = content_mapping.get_hash(full_path.as_str()).cloned();
            (dep, item)
        })
        .collect::<Vec<(String, Option<String>)>>();

    // Check all dependencies are available
    if dependencies.iter().any(|(_, hash)| hash.is_none()) {
        return Err(anyhow::Error::msg(
            "There are some missing dependencies in the gltf",
        ));
    }

    let dependencies_hash: Vec<(String, String)> = dependencies
        .into_iter()
        .map(|(file_path, hash)| (file_path, hash.unwrap()))
        .collect();

    // Download all dependencies in parallel
    let futures = dependencies_hash.iter().map(|(_, dependency_file_hash)| {
        let ctx = ctx.clone();
        let content_mapping = content_mapping.clone();
        async move {
            let url = format!("{}{}", content_mapping.base_url, dependency_file_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dependency_file_hash);
            ctx.resource_provider
                .fetch_resource(url, dependency_file_hash.clone(), absolute_file_path)
                .await
                .map_err(|e| format!("Dependency {} failed: {:?}", dependency_file_hash, e))
        }
    });

    let result = futures_util::future::join_all(futures).await;
    if result.iter().any(|res| res.is_err()) {
        let errors: Vec<String> = result.into_iter().filter_map(|res| res.err()).collect();
        return Err(anyhow::Error::msg(format!(
            "Error downloading gltf dependencies: {}",
            errors.join("\n")
        )));
    }

    // Acquire thread safety guard for Godot API access
    let _thread_guard = GodotThreadSafetyGuard::acquire(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed to acquire thread safety guard"))?;

    // Load the GLTF using Godot
    let mut new_gltf = GltfDocument::new_gd();
    let mut new_gltf_state = GltfState::new_gd();

    let mappings = Dictionary::from_iter(
        dependencies_hash
            .iter()
            .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
    );

    new_gltf_state.set_additional_data("base_path".into(), "some".to_variant());
    new_gltf_state.set_additional_data("mappings".into(), mappings.to_variant());

    let err = new_gltf
        .append_from_file_ex(
            GString::from(absolute_file_path.as_str()),
            new_gltf_state.clone(),
        )
        .base_path(GString::from(ctx.content_folder.as_str()))
        .flags(0)
        .done();

    if err != godot::global::Error::OK {
        return Err(anyhow::Error::msg(format!("Error loading gltf: {:?}", err)));
    }

    let node = new_gltf
        .generate_scene(new_gltf_state)
        .ok_or(anyhow::Error::msg("Error generating scene from gltf"))?;

    // Post-process textures
    let max_size = ctx.texture_quality.to_max_size();
    post_import_process(node.clone(), max_size);

    // Cast to Node3D and rotate
    let mut node = node
        .try_cast::<Node3D>()
        .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;
    node.rotate_y(std::f32::consts::PI);

    // Create colliders (with mask=0 initially - will be set by gltf_container.gd after loading)
    let root_node = node.clone();
    create_colliders(node.clone().upcast(), root_node.clone());

    // Save the processed scene to disk
    let scene_path = get_scene_path_for_hash(&file_hash);
    save_node_as_scene(node.clone(), &scene_path).map_err(anyhow::Error::msg)?;

    // Free the node since we've saved it to disk
    node.queue_free();

    Ok(scene_path)
}

/// Post-process textures in the node tree
fn post_import_process(node_to_inspect: Gd<Node>, max_size: i32) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            if let Some(mesh) = mesh_instance_3d.get_mesh() {
                for surface_index in 0..mesh.get_surface_count() {
                    if let Some(material) = mesh.surface_get_material(surface_index) {
                        if let Ok(mut base_material) = material.try_cast::<BaseMaterial3D>() {
                            for ord in 0..TextureParam::MAX.ord() {
                                let texture_param = TextureParam::from_ord(ord);
                                if let Some(texture) = base_material.get_texture(texture_param) {
                                    if let Ok(mut texture_image) =
                                        texture.try_cast::<ImageTexture>()
                                    {
                                        if let Some(mut image) = texture_image.get_image() {
                                            if std::env::consts::OS == "ios" {
                                                let texture =
                                                    create_compressed_texture(&mut image, max_size);
                                                base_material.set_texture(texture_param, texture);
                                            } else if resize_image(&mut image, max_size) {
                                                texture_image.set_image(image);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        post_import_process(child, max_size);
    }
}

/// Get dependencies from a GLTF file (images and buffers)
async fn get_dependencies(file_path: &String) -> Result<Vec<String>, anyhow::Error> {
    let mut dependencies = Vec::new();
    let mut file = tokio::fs::File::open(file_path).await?;

    let magic = file.read_i32_le().await?;
    let json: serde_json::Value = if magic == 0x46546C67 {
        // Binary GLTF
        let _version = file.read_i32_le().await?;
        let _length = file.read_i32_le().await?;
        let chunk_length = file.read_i32_le().await?;
        let _chunk_type = file.read_i32_le().await?;

        let mut json_data = vec![0u8; chunk_length as usize];
        let _ = file.read_exact(&mut json_data).await?;
        serde_json::de::from_slice(json_data.as_slice())
    } else {
        // Text GLTF
        let mut json_data = Vec::new();
        let _ = file.seek(std::io::SeekFrom::Start(0)).await?;
        let _ = file.read_to_end(&mut json_data).await?;
        serde_json::de::from_slice(json_data.as_slice())
    }?;

    // Extract image URIs
    if let Some(images) = json.get("images") {
        if let Some(images) = images.as_array() {
            for image in images {
                if let Some(uri) = image.get("uri") {
                    if let Some(uri) = uri.as_str() {
                        if !uri.is_empty() && !uri.starts_with("data:") {
                            dependencies.push(uri.to_string());
                        }
                    }
                }
            }
        }
    }

    // Extract buffer URIs
    if let Some(buffers) = json.get("buffers") {
        if let Some(buffers) = buffers.as_array() {
            for buffer in buffers {
                if let Some(uri) = buffer.get("uri") {
                    if let Some(uri) = uri.as_str() {
                        if !uri.is_empty() && !uri.starts_with("data:") {
                            dependencies.push(uri.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(dependencies)
}

/// Get the StaticBody3D collider from a MeshInstance3D (created by create_trimesh_collision)
fn get_static_body_collider(
    mesh_instance: &Gd<MeshInstance3D>,
) -> Option<Gd<godot::engine::StaticBody3D>> {
    for maybe_static_body in mesh_instance.get_children().iter_shared() {
        if let Ok(static_body_3d) = maybe_static_body.try_cast::<godot::engine::StaticBody3D>() {
            return Some(static_body_3d);
        }
    }
    None
}

/// Create colliders for all mesh instances
/// Note: Colliders are created with mask=0 (disabled) and no scene_id/entity_id.
/// The masks and metadata should be set by the caller after instantiating the scene.
/// Uses AnimatableBody3D in STATIC mode for performance. When the entity moves,
/// gltf_container.gd will switch it to KINEMATIC mode.
fn create_colliders(node_to_inspect: Gd<Node>, root_node: Gd<Node3D>) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mut mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            let invisible_mesh = mesh_instance_3d
                .get_name()
                .to_string()
                .to_lowercase()
                .contains("collider");

            if invisible_mesh {
                mesh_instance_3d.set_visible(false);
            }

            // First check if there's already a StaticBody3D (created by create_trimesh_collision)
            let mut static_body_3d = get_static_body_collider(&mesh_instance_3d);
            if static_body_3d.is_none() {
                mesh_instance_3d.create_trimesh_collision();
                static_body_3d = get_static_body_collider(&mesh_instance_3d);
            }

            if let Some(mut static_body_3d) = static_body_3d {
                // Create AnimatableBody3D to replace StaticBody3D
                let mut animatable_body = AnimatableBody3D::new_alloc();
                animatable_body.set_sync_to_physics(false);
                animatable_body.set_process_mode(ProcessMode::DISABLED);
                animatable_body.set_meta("dcl_col".into(), 0.to_variant());
                animatable_body.set_meta("invisible_mesh".into(), invisible_mesh.to_variant());
                animatable_body.set_collision_layer(0);
                animatable_body.set_collision_mask(0);
                animatable_body.set_name(GString::from(format!(
                    "{}_colgen",
                    mesh_instance_3d.get_name()
                )));

                // Get the parent to add the new body
                if let Some(mut parent) = static_body_3d.get_parent() {
                    parent.add_child(animatable_body.clone().upcast());

                    // Move collision shapes from StaticBody3D to AnimatableBody3D
                    for mut body_child in static_body_3d
                        .get_children_ex()
                        .include_internal(true)
                        .done()
                        .iter_shared()
                    {
                        static_body_3d.remove_child(body_child.clone());
                        body_child.call("set_owner".into(), &[godot::builtin::Variant::nil()]);
                        animatable_body.add_child(body_child.clone());

                        // Enable backface collision for concave shapes
                        if let Ok(collision_shape_3d) =
                            body_child.clone().try_cast::<CollisionShape3D>()
                        {
                            if let Some(shape) = collision_shape_3d.get_shape() {
                                if let Ok(mut concave_polygon_shape_3d) =
                                    shape.try_cast::<ConcavePolygonShape3D>()
                                {
                                    concave_polygon_shape_3d.set_backface_collision_enabled(true);
                                }
                            }
                        }

                        // Set owner to root so it gets saved with PackedScene
                        body_child.set_owner(root_node.clone().upcast());
                    }

                    // Remove the old StaticBody3D
                    parent.remove_child(static_body_3d.clone().upcast());
                    static_body_3d.queue_free();

                    // Set owner for AnimatableBody3D
                    animatable_body.set_owner(root_node.clone().upcast());
                }
            }
        }

        create_colliders(child, root_node.clone());
    }
}
