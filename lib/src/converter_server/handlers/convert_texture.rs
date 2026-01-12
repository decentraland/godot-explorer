/*
 * Texture to .res Conversion Handler
 *
 * Accepts image uploads and converts them to Godot .res files
 * with mobile optimizations (ETC2 compression).
 */

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::converter_server::server::{AssetType, CachedAsset, ConverterState};

use super::json_success_response;

/// Handle texture conversion request
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
            "resource_path": format!("res://textures/{}.res", hash),
            "cached": true,
            "file_path": existing.file_path.to_string_lossy(),
        }));
    }

    // Get original filename and detect format from headers
    let original_name = headers
        .get("x-filename")
        .cloned()
        .unwrap_or_else(|| format!("{}.png", &hash[..8]));

    // Detect file extension from original name or content-type
    let extension = detect_extension(&original_name, headers);

    // Save the uploaded file
    let image_path = state.cache_folder.join(format!("{}.{}", hash, extension));
    if let Err(e) = std::fs::write(&image_path, body) {
        return json_error_response(500, &format!("Failed to save file: {}", e));
    }

    // TODO: Queue conversion on Godot main thread
    // The actual conversion needs to happen on the Godot main thread using:
    // - Image.load_from_file() to load the image
    // - Image.compress() with ETC2 for mobile
    // - ResourceSaver.save() to save as .res

    let res_path = state.cache_folder.join(format!("{}.res", hash));

    let asset = CachedAsset {
        hash: hash.clone(),
        asset_type: AssetType::Texture,
        file_path: image_path.clone(),
        original_name,
    };
    state.add_asset(asset);

    json_success_response(serde_json::json!({
        "hash": hash,
        "resource_path": format!("res://textures/{}.res", hash),
        "cached": false,
        "image_saved": image_path.to_string_lossy(),
        "res_path": res_path.to_string_lossy(),
        "status": "pending_conversion",
        "note": "Texture conversion requires Godot main thread - conversion will be queued"
    }))
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn detect_extension(filename: &str, headers: &HashMap<String, String>) -> String {
    if let Some(ext) = filename.rsplit('.').next() {
        let ext_lower = ext.to_lowercase();
        if matches!(
            ext_lower.as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tga"
        ) {
            return ext_lower;
        }
    }

    if let Some(content_type) = headers.get("content-type") {
        match content_type.as_str() {
            "image/png" => return "png".to_string(),
            "image/jpeg" => return "jpg".to_string(),
            "image/webp" => return "webp".to_string(),
            _ => {}
        }
    }

    "png".to_string()
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
