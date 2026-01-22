//! Common utilities and pipeline for GLTF loading.

use std::sync::Arc;

use godot::{
    builtin::GString,
    classes::{
        base_material_3d::TextureParam, BaseMaterial3D, GltfDocument, GltfState, ImageTexture,
        MeshInstance3D, Node, Node3D,
    },
    global::Error,
    meta::ToGodot,
    obj::Gd,
    prelude::*,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Semaphore;

use crate::content::texture::resize_image;

use super::super::{
    content_mapping::ContentMappingAndUrlRef, content_provider::SceneGltfContext,
    file_string::get_base_dir, texture::create_compressed_texture,
};

#[cfg(feature = "use_resource_tracking")]
use crate::godot_classes::dcl_resource_tracker::{
    report_resource_error, report_resource_loaded, report_resource_start,
};

/// Post-import texture processing for all GLTF types.
/// Resizes images according to max_size limits.
pub fn post_import_process(node_to_inspect: Gd<Node>, max_size: i32) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            if let Some(mesh) = mesh_instance_3d.get_mesh() {
                for surface_index in 0..mesh.get_surface_count() {
                    if let Some(material) = mesh.surface_get_material(surface_index) {
                        if let Ok(mut base_material) = material.try_cast::<BaseMaterial3D>() {
                            // Resize images
                            for ord in 0..TextureParam::MAX.ord() {
                                let texture_param = TextureParam::from_ord(ord);
                                if let Some(texture) = base_material.get_texture(texture_param) {
                                    if let Ok(mut texture_image) =
                                        texture.try_cast::<ImageTexture>()
                                    {
                                        if let Some(mut image) = texture_image.get_image() {
                                            if std::env::consts::OS == "ios"
                                                || std::env::consts::OS == "android"
                                            {
                                                let texture =
                                                    create_compressed_texture(&mut image, max_size);
                                                base_material.set_texture(texture_param, &texture);
                                            } else if resize_image(&mut image, max_size) {
                                                texture_image.set_image(&image);
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

/// Recursively clear the owner of a node and all its children
pub(super) fn clear_owner_recursive(node: &mut Gd<Node>) {
    node.set_owner(Gd::<Node>::null_arg());
    for mut child in node.get_children().iter_shared() {
        clear_owner_recursive(&mut child);
    }
}

/// Recursively set the owner of a node and all its children
pub(super) fn set_owner_recursive(node: &mut Gd<Node>, owner: &Gd<Node>) {
    node.set_owner(owner);
    for mut child in node.get_children().iter_shared() {
        set_owner_recursive(&mut child, owner);
    }
}

/// Parse GLTF/GLB file to extract dependencies (images and buffers).
/// Returns file paths as referenced in the GLTF (relative paths like "textures/image.png").
pub async fn get_dependencies(file_path: &str) -> Result<Vec<String>, anyhow::Error> {
    let mut dependencies = Vec::new();
    let mut file = tokio::fs::File::open(file_path).await?;

    let magic = file.read_i32_le().await?;
    let json: serde_json::Value = if magic == 0x46546C67 {
        let _version = file.read_i32_le().await?;
        let _length = file.read_i32_le().await?;
        let chunk_length = file.read_i32_le().await?;
        let _chunk_type = file.read_i32_le().await?;

        let mut json_data = vec![0u8; chunk_length as usize];
        let _ = file.read_exact(&mut json_data).await?;
        serde_json::de::from_slice(json_data.as_slice())
    } else {
        let mut json_data = Vec::new();
        let _ = file.seek(std::io::SeekFrom::Start(0)).await?;
        let _ = file.read_to_end(&mut json_data).await?;
        serde_json::de::from_slice(json_data.as_slice())
    }?;

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

    if let Some(images) = json.get("buffers") {
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

    Ok(dependencies)
}

/// Thread safety guard for Godot API access
pub struct GodotThreadSafetyGuard {
    _guard: tokio::sync::OwnedSemaphorePermit,
}

impl GodotThreadSafetyGuard {
    pub async fn acquire(godot_single_thread: &Arc<Semaphore>) -> Option<Self> {
        let guard = godot_single_thread.clone().acquire_owned().await.ok()?;
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
        godot::tools::load::<godot::classes::Script>("res://src/logic/thread_safety.gd");
    temp_script.call("set_thread_safety_checks_enabled", &[enabled.to_variant()]);
}

/// Count the number of nodes in a tree
pub(super) fn count_nodes(node: Gd<Node>) -> i32 {
    let mut count = 1;
    for child in node.get_children().iter_shared() {
        count += count_nodes(child);
    }
    count
}

/// Common GLTF loading pipeline.
///
/// This handles the shared logic for loading scenes, wearables, and emotes:
/// 1. Download main GLTF file
/// 2. Parse and download dependencies
/// 3. Acquire Godot thread safety guard
/// 4. Load GltfDocument
/// 5. Post-process textures
/// 6. Rotate node 180° Y
/// 7. Call processor function for type-specific processing
/// 8. Cleanup source file
///
/// The processor function receives the loaded Node3D and should return
/// a tuple of (result, file_size). The caller is responsible for cache registration.
pub async fn load_gltf_pipeline<F, R>(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
    processor: F,
) -> Result<(R, i64), anyhow::Error>
where
    F: FnOnce(Gd<Node3D>, &str, &SceneGltfContext) -> Result<(R, i64), anyhow::Error>,
{
    // Download the main GLTF file
    let base_path = Arc::new(get_base_dir(&file_path));
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    #[cfg(feature = "use_resource_tracking")]
    report_resource_start(&file_hash, "gltf");

    let gltf_result = ctx
        .resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await;

    #[cfg(feature = "use_resource_tracking")]
    if let Err(ref e) = gltf_result {
        report_resource_error(&file_hash, &e.to_string());
    }

    gltf_result.map_err(anyhow::Error::msg)?;

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
        let dep_hash = dependency_file_hash.clone();
        async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&dep_hash, "gltf_dep");

            let url = format!("{}{}", content_mapping.base_url, dep_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dep_hash);
            let result = ctx
                .resource_provider
                .fetch_resource(url, dep_hash.clone(), absolute_file_path)
                .await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(ref e) = result {
                report_resource_error(&dep_hash, &format!("{:?}", e));
            } else {
                report_resource_loaded(&dep_hash);
            }

            result.map_err(|e| format!("Dependency {} failed: {:?}", dep_hash, e))
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
    let _thread_guard = GodotThreadSafetyGuard::acquire(&ctx.godot_single_thread)
        .await
        .ok_or(anyhow::Error::msg("Failed to acquire thread safety guard"))?;

    // Process GLTF using Godot (all Godot objects are scoped here to drop before await)
    let (result, file_size) = {
        // Load the GLTF using Godot
        let mut new_gltf = GltfDocument::new_gd();
        let mut new_gltf_state = GltfState::new_gd();

        let mappings = VarDictionary::from_iter(
            dependencies_hash
                .iter()
                .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
        );

        new_gltf_state.set_additional_data("base_path", &"some".to_variant());
        new_gltf_state.set_additional_data("mappings", &mappings.to_variant());

        let file_path_gstr = GString::from(absolute_file_path.as_str());
        let base_path_gstr = GString::from(ctx.content_folder.as_str());
        let err = new_gltf
            .append_from_file_ex(&file_path_gstr, &new_gltf_state.clone())
            .base_path(&base_path_gstr)
            .flags(0)
            .done();

        if err != Error::OK {
            return Err(anyhow::Error::msg(format!("Error loading gltf: {:?}", err)));
        }

        let node = new_gltf
            .generate_scene(&new_gltf_state)
            .ok_or(anyhow::Error::msg("Error generating scene from gltf"))?;

        // Post-process textures
        let max_size = ctx.texture_quality.to_max_size();
        post_import_process(node.clone(), max_size);

        // Cast to Node3D and rotate
        let mut node = node
            .try_cast::<Node3D>()
            .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;
        node.rotate_y(std::f32::consts::PI);

        // Call the type-specific processor
        processor(node, &file_hash, &ctx)?
    };
    // All Godot objects are now dropped, safe to await

    // Cleanup source GLTF file after successful save
    // NOTE: We only delete the main GLTF file, NOT dependencies (textures/buffers).
    // Dependencies may be shared by multiple GLTFs loading in parallel.
    // They will be cleaned up by LRU eviction when the cache exceeds its limit.
    ctx.resource_provider
        .try_delete_file_by_hash(&file_hash)
        .await;

    #[cfg(feature = "use_resource_tracking")]
    report_resource_loaded(&file_hash);

    Ok((result, file_size))
}
