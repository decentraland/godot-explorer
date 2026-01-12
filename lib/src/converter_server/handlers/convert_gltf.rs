/*
 * GLTF/GLB to .scn Conversion Handler
 *
 * Accepts GLB/GLTF file uploads and converts them to Godot .scn files
 * with mobile optimizations (ETC2 compression).
 *
 * Uses the existing content pipeline from content_provider.
 */

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::content::content_mapping::ContentMappingAndUrl;
use crate::content::gltf::load_and_save_scene_gltf;
use crate::converter_server::server::{AssetType, CachedAsset, ConverterState};

use super::json_success_response;

/// Handle GLTF conversion request
pub async fn handle(
    state: &Arc<ConverterState>,
    headers: &HashMap<String, String>,
    body: &[u8],
) -> String {
    if body.is_empty() {
        return json_error_response(400, "Empty request body");
    }

    // Compute hash of the content
    let hash = compute_hash(body);

    // Check if already converted
    if let Some(existing) = state.get_asset(&hash) {
        return json_success_response(serde_json::json!({
            "hash": hash,
            "scene_path": format!("res://glbs/{}.scn", hash),
            "cached": true,
            "file_path": existing.file_path.to_string_lossy(),
        }));
    }

    // Get original filename from headers if available
    let original_name = headers
        .get("x-filename")
        .cloned()
        .unwrap_or_else(|| format!("{}.glb", &hash[..8]));

    // Save the uploaded file to cache
    let glb_path = state.cache_folder.join(&hash);
    if let Err(e) = std::fs::write(&glb_path, body) {
        return json_error_response(500, &format!("Failed to save file: {}", e));
    }

    // Create content mapping for the uploaded file
    // For a single GLB file with embedded textures, we only need the main file mapping
    let mut content_mapping = ContentMappingAndUrl::new();
    content_mapping.base_url = format!("file://{}/", state.cache_folder.to_string_lossy());

    // The file path is just the filename (hash), and the hash is the same
    // This creates a mapping: original_name -> hash
    let content_mapping = Arc::new(ContentMappingAndUrl::from_base_url_and_content(
        format!("file://{}/", state.cache_folder.to_string_lossy()),
        vec![crate::dcl::common::content_entity::TypedIpfsRef {
            file: original_name.clone(),
            hash: hash.clone(),
        }],
    ));

    // Create the GLTF context from state
    let ctx = state.create_gltf_context();

    // Call the existing GLTF pipeline
    let result = load_and_save_scene_gltf(
        original_name.clone(),
        hash.clone(),
        content_mapping,
        ctx,
    )
    .await;

    match result {
        Ok(scene_path) => {
            // Register the asset
            let asset = CachedAsset {
                hash: hash.clone(),
                asset_type: AssetType::Scene,
                file_path: std::path::PathBuf::from(&scene_path),
                original_name,
            };
            state.add_asset(asset);

            json_success_response(serde_json::json!({
                "hash": hash,
                "scene_path": format!("res://glbs/{}.scn", hash),
                "file_path": scene_path,
                "cached": false,
                "status": "converted",
            }))
        }
        Err(e) => {
            // Clean up the saved GLB file on error
            std::fs::remove_file(&glb_path).ok();
            json_error_response(500, &format!("GLTF conversion failed: {}", e))
        }
    }
}

/// Compute SHA256 hash of data
fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn json_error_response(status_code: u16, message: &str) -> String {
    let status_text = match status_code {
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Error",
    };

    let response_body = serde_json::json!({
        "error": message,
    });

    format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\r\n{}",
        status_code,
        status_text,
        response_body.to_string().len(),
        response_body
    )
}
