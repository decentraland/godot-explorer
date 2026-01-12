/*
 * Scene Package Handler
 *
 * Creates ZIP packages containing converted scenes and textures
 * using Godot's ZIPPacker for mobile-optimized bundles.
 *
 * Uses the full ContentProvider infrastructure with thread safety.
 */

use godot::classes::zip_packer::ZipAppend;
use godot::classes::{FileAccess, ZipPacker};
use godot::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::content::thread_safety::set_thread_safety_checks_enabled;
use crate::converter_server::server::{AssetType, CachedAsset, ConverterState};

use super::json_success_response;

#[derive(serde::Deserialize)]
pub struct PackageRequest {
    pub scene_id: String,
    pub assets: Vec<String>,
}

/// Handle scene packaging request
pub async fn handle(
    state: &Arc<ConverterState>,
    _headers: &HashMap<String, String>,
    body: &[u8],
) -> String {
    let request: PackageRequest = match serde_json::from_slice(body) {
        Ok(req) => req,
        Err(e) => return json_error_response(400, &format!("Invalid JSON: {}", e)),
    };

    if request.scene_id.is_empty() {
        return json_error_response(400, "scene_id is required");
    }

    if request.assets.is_empty() {
        return json_error_response(400, "assets array cannot be empty");
    }

    // Verify all requested assets exist
    let mut missing_assets = Vec::new();
    let mut found_assets = Vec::new();

    for hash in &request.assets {
        if let Some(asset) = state.get_asset(hash) {
            found_assets.push(asset);
        } else {
            missing_assets.push(hash.clone());
        }
    }

    if !missing_assets.is_empty() {
        return json_error_response(
            404,
            &format!("Assets not found: {}", missing_assets.join(", ")),
        );
    }

    let zip_filename = format!("{}-mobile.zip", request.scene_id);
    let zip_path = state.cache_folder.join(&zip_filename);
    let zip_path_str = zip_path.to_string_lossy().to_string();

    // Acquire semaphore for Godot thread safety
    let _guard = match state.godot_single_thread.clone().acquire_owned().await {
        Ok(guard) => guard,
        Err(_) => return json_error_response(500, "Failed to acquire Godot thread lock"),
    };

    // Disable thread safety checks while we use Godot APIs
    set_thread_safety_checks_enabled(false);

    // Create the ZIP using Godot's ZIPPacker
    let result = create_zip_package(&zip_path_str, &request.scene_id, &found_assets);

    set_thread_safety_checks_enabled(true);
    drop(_guard);

    match result {
        Ok(manifest) => {
            // Get final ZIP file size
            let zip_size = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);

            json_success_response(serde_json::json!({
                "scene_id": request.scene_id,
                "zip_path": zip_path_str,
                "zip_filename": zip_filename,
                "zip_size": zip_size,
                "assets_count": found_assets.len(),
                "manifest": manifest,
                "status": "packaged",
            }))
        }
        Err(e) => json_error_response(500, &format!("Failed to create ZIP package: {}", e)),
    }
}

/// Create a ZIP package using Godot's ZIPPacker
/// Must be called with thread safety disabled
fn create_zip_package(
    zip_path: &str,
    scene_id: &str,
    assets: &[CachedAsset],
) -> Result<serde_json::Value, String> {
    let mut zip = ZipPacker::new_gd();

    // Open ZIP file for writing
    let err = zip
        .open_ex(zip_path)
        .append(ZipAppend::CREATE)
        .done();
    if err != godot::global::Error::OK {
        return Err(format!("Failed to create ZIP: {:?}", err));
    }

    // Build manifest as we add files
    let mut manifest_assets = Vec::new();

    // Add each asset file to the ZIP
    for asset in assets {
        let src_path = asset.file_path.to_string_lossy().to_string();

        // Determine ZIP internal path based on asset type
        let zip_internal_path = match asset.asset_type {
            AssetType::Scene => format!("glbs/{}.scn", asset.hash),
            AssetType::Texture => format!("textures/{}.res", asset.hash),
        };

        // Read the source file
        let file_bytes = FileAccess::get_file_as_bytes(&GString::from(&src_path));
        if file_bytes.is_empty() {
            // Close ZIP before returning error
            zip.close();
            return Err(format!("Failed to read asset file: {}", src_path));
        }

        // Start file in ZIP
        let err = zip.start_file(&GString::from(&zip_internal_path));
        if err != godot::global::Error::OK {
            zip.close();
            return Err(format!(
                "Failed to start file {} in ZIP: {:?}",
                zip_internal_path, err
            ));
        }

        // Write file data
        let err = zip.write_file(&file_bytes);
        if err != godot::global::Error::OK {
            zip.close();
            return Err(format!(
                "Failed to write file {} to ZIP: {:?}",
                zip_internal_path, err
            ));
        }

        // Add to manifest
        let res_path = match asset.asset_type {
            AssetType::Scene => format!("res://glbs/{}.scn", asset.hash),
            AssetType::Texture => format!("res://textures/{}.res", asset.hash),
        };

        manifest_assets.push(serde_json::json!({
            "hash": asset.hash,
            "type": format!("{:?}", asset.asset_type).to_lowercase(),
            "path": res_path,
            "original_name": asset.original_name,
        }));
    }

    // Create and add manifest.json
    let manifest = serde_json::json!({
        "scene_id": scene_id,
        "version": "1.0",
        "platform": "mobile",
        "compression": "etc2",
        "assets": manifest_assets,
    });

    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap_or_default();

    let err = zip.start_file(&GString::from("manifest.json"));
    if err != godot::global::Error::OK {
        zip.close();
        return Err(format!("Failed to start manifest.json in ZIP: {:?}", err));
    }

    let manifest_bytes = PackedByteArray::from(manifest_json.as_bytes());
    let err = zip.write_file(&manifest_bytes);
    if err != godot::global::Error::OK {
        zip.close();
        return Err(format!("Failed to write manifest.json to ZIP: {:?}", err));
    }

    // Close ZIP
    let err = zip.close();
    if err != godot::global::Error::OK {
        return Err(format!("Failed to close ZIP: {:?}", err));
    }

    Ok(manifest)
}

fn json_error_response(status_code: u16, message: &str) -> String {
    let status_text = match status_code {
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Error",
    };

    let response_body = serde_json::json!({ "error": message });

    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
        status_code, status_text, response_body.to_string().len(), response_body
    )
}
