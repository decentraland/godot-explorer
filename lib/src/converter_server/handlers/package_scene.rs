/*
 * Scene Package Handler
 *
 * Creates ZIP packages containing converted scenes and textures
 * using Godot's ZIPPacker for mobile-optimized bundles.
 */

use std::collections::HashMap;
use std::sync::Arc;

use crate::converter_server::server::ConverterState;

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

    // TODO: Create ZIP using Godot's ZIPPacker on main thread
    // The actual ZIP creation needs to happen on the Godot main thread using:
    // - ZIPPacker.open(path, ZIPPacker.APPEND_CREATE)
    // - ZIPPacker.start_file() / ZIPPacker.write_file() for each asset
    // - ZIPPacker.close()

    let manifest = serde_json::json!({
        "scene_id": request.scene_id,
        "version": "1.0",
        "platform": "mobile",
        "compression": "etc2",
        "assets": found_assets.iter().map(|asset| {
            let asset_path = match asset.asset_type {
                crate::converter_server::server::AssetType::Scene => format!("res://glbs/{}.scn", asset.hash),
                crate::converter_server::server::AssetType::Texture => format!("res://textures/{}.res", asset.hash),
            };
            serde_json::json!({
                "hash": asset.hash,
                "type": format!("{:?}", asset.asset_type).to_lowercase(),
                "path": asset_path,
                "original_name": asset.original_name,
            })
        }).collect::<Vec<_>>(),
    });

    json_success_response(serde_json::json!({
        "scene_id": request.scene_id,
        "zip_path": zip_path.to_string_lossy(),
        "zip_filename": zip_filename,
        "assets_count": found_assets.len(),
        "manifest": manifest,
        "status": "pending_packaging",
        "note": "ZIP packaging requires Godot main thread (ZIPPacker) - packaging will be queued"
    }))
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
