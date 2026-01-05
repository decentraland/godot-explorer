//! Wearable GLTF loading (for ContentProvider wearable loading).

use super::super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::SceneGltfContext,
    scene_saver::{get_wearable_path_for_hash, save_node_as_scene},
};
use super::common::{count_nodes, load_gltf_pipeline};

/// Load and save a wearable GLTF to disk.
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Saves the processed scene to disk (NO colliders - wearables don't need them)
///
/// Returns the path to the saved scene file on success.
pub async fn load_and_save_wearable_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
) -> Result<String, anyhow::Error> {
    let ctx_clone = ctx.clone();

    let (scene_path, file_size) = load_gltf_pipeline(
        file_path,
        file_hash.clone(),
        content_mapping,
        ctx,
        |node, hash, ctx| {
            // NOTE: No colliders for wearables - they don't need collision shapes

            // Save the processed scene to disk
            let scene_path = get_wearable_path_for_hash(&ctx.content_folder, hash);
            save_node_as_scene(node.clone(), &scene_path).map_err(anyhow::Error::msg)?;

            // Get file size synchronously
            let file_size = std::fs::metadata(&scene_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            // Count nodes before freeing
            let node_count = count_nodes(node.clone().upcast());
            tracing::info!(
                "Wearable GLTF processed: {} with {} nodes, saved to {} ({} bytes)",
                hash,
                node_count,
                scene_path,
                file_size
            );

            // Free the node since we've saved it to disk
            node.free();

            Ok((scene_path, file_size))
        },
    )
    .await?;

    // Register the saved scene in resource_provider for cache management
    ctx_clone
        .resource_provider
        .register_local_file(&scene_path, file_size)
        .await;

    Ok(scene_path)
}
