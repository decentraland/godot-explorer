/*
 * GLTF/GLB to .scn Conversion Handler
 *
 * Accepts GLB/GLTF file uploads and converts them to Godot .scn files
 * with mobile optimizations (ETC2 compression).
 *
 * Uses the full ContentProvider infrastructure with promises, threading, and caching.
 */

use godot::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::content::content_mapping::{ContentMappingAndUrl, DclContentMappingAndUrl};
use crate::content::thread_safety::set_thread_safety_checks_enabled;
use crate::converter_server::server::{
    wait_for_promise, AssetType, CachedAsset, ConverterState, PromiseResult,
};
use crate::dcl::common::content_entity::TypedIpfsRef;

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

    // Check if already converted in our local cache
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

    // Save the uploaded file to cache folder (so ContentProvider can access it via file:// URL)
    let glb_path = state.cache_folder.join(&hash);
    if let Err(e) = std::fs::write(&glb_path, body) {
        return json_error_response(500, &format!("Failed to save file: {}", e));
    }

    // Create content mapping for the uploaded file
    // The base_url points to the cache folder, and the mapping maps original_name -> hash
    let content_mapping = Arc::new(ContentMappingAndUrl::from_base_url_and_content(
        format!("file://{}/", state.cache_folder.to_string_lossy()),
        vec![TypedIpfsRef {
            file: original_name.clone(),
            hash: hash.clone(),
        }],
    ));

    // Acquire semaphore for Godot thread safety
    let _guard = match state.godot_single_thread.clone().acquire_owned().await {
        Ok(guard) => guard,
        Err(_) => {
            std::fs::remove_file(&glb_path).ok();
            return json_error_response(500, "Failed to acquire Godot thread lock");
        }
    };

    // Disable thread safety checks while we use Godot APIs
    set_thread_safety_checks_enabled(false);

    // Get ContentProvider and call load_scene_gltf, extracting InstanceId immediately
    let promise_id = {
        let content_provider = match state.get_content_provider() {
            Some(cp) => cp,
            None => {
                set_thread_safety_checks_enabled(true);
                std::fs::remove_file(&glb_path).ok();
                return json_error_response(500, "ContentProvider not available");
            }
        };

        // Create DclContentMappingAndUrl from our Rust struct
        let dcl_content_mapping = DclContentMappingAndUrl::from_ref(content_mapping);

        // Call the ContentProvider's load_scene_gltf method
        let mut cp = content_provider.clone();
        let promise_opt = cp
            .bind_mut()
            .load_scene_gltf(GString::from(&original_name), dcl_content_mapping);

        // Extract InstanceId before the Gd<Promise> goes out of scope
        match promise_opt {
            Some(p) => p.instance_id(),
            None => {
                set_thread_safety_checks_enabled(true);
                std::fs::remove_file(&glb_path).ok();
                return json_error_response(
                    500,
                    &format!("Failed to start GLTF conversion for {}", original_name),
                );
            }
        }
    };

    // Re-enable thread safety checks after getting the promise
    set_thread_safety_checks_enabled(true);

    // Release guard before waiting
    drop(_guard);

    // Wait for the promise to resolve using our bridge function
    let result = wait_for_promise(promise_id, state.godot_single_thread.clone()).await;

    match result {
        Ok(PromiseResult::Resolved(scene_path)) => {
            // Register the asset in our local cache
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
        Ok(PromiseResult::Rejected(e)) => {
            // Clean up the saved GLB file on error
            std::fs::remove_file(&glb_path).ok();
            json_error_response(500, &format!("GLTF conversion failed: {}", e))
        }
        Err(e) => {
            // Clean up the saved GLB file on error
            std::fs::remove_file(&glb_path).ok();
            json_error_response(500, &format!("GLTF conversion error: {}", e))
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
