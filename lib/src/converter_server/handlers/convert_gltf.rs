/*
 * GLTF/GLB to .scn Conversion Handler
 *
 * Accepts GLB/GLTF file uploads and converts them to Godot .scn files
 * with mobile optimizations (ETC2 compression).
 */

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::converter_server::server::{AssetType, CachedAsset, ConverterState};

use super::json_success_response;

/// Handle GLTF conversion request
pub async fn handle(
    state: &Arc<ConverterState>,
    headers: &HashMap<String, String>,
    body: &[u8],
) -> String {
    // For now, we'll implement a simple version that saves the file
    // and returns a hash. Full GLTF conversion will require Godot main thread access.

    if body.is_empty() {
        return super::json_error_response(400, "Empty request body");
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

    // Save the uploaded file
    let glb_path = state.cache_folder.join(format!("{}.glb", hash));
    if let Err(e) = std::fs::write(&glb_path, body) {
        return super::json_error_response(500, &format!("Failed to save file: {}", e));
    }

    // TODO: Queue conversion on Godot main thread
    // For now, we'll just save the GLB and return success
    // The actual conversion needs to happen on the Godot main thread using:
    // - GltfDocument for loading
    // - create_scene_colliders for colliders
    // - save_node_as_scene for saving

    // Create a placeholder .scn path (actual conversion pending)
    let scn_path = state.cache_folder.join(format!("{}.scn", hash));

    // Register the asset (for now, pointing to the GLB file)
    let asset = CachedAsset {
        hash: hash.clone(),
        asset_type: AssetType::Scene,
        file_path: glb_path.clone(),
        original_name,
    };
    state.add_asset(asset);

    json_success_response(serde_json::json!({
        "hash": hash,
        "scene_path": format!("res://glbs/{}.scn", hash),
        "cached": false,
        "glb_saved": glb_path.to_string_lossy(),
        "scn_path": scn_path.to_string_lossy(),
        "status": "pending_conversion",
        "note": "GLTF conversion requires Godot main thread - conversion will be queued"
    }))
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
