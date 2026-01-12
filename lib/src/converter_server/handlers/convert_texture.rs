/*
 * Texture to .res Conversion Handler
 *
 * Accepts image uploads and converts them to Godot .res files
 * with mobile optimizations (ETC2 compression).
 *
 * Uses the existing texture processing from content/texture.rs.
 */

use godot::classes::resource_saver::SaverFlags;
use godot::classes::{Image, Resource, ResourceSaver};
use godot::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use crate::content::texture::create_compressed_texture;
use crate::content::thread_safety::set_thread_safety_checks_enabled;
use crate::converter_server::server::{AssetType, CachedAsset, ConverterState};
use crate::utils::infer_mime;

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

    // Get original filename from headers
    let original_name = headers
        .get("x-filename")
        .cloned()
        .unwrap_or_else(|| format!("{}.png", &hash[..8]));

    // Acquire Godot thread safety
    let _guard = match state.godot_single_thread.clone().acquire_owned().await {
        Ok(guard) => guard,
        Err(_) => return json_error_response(500, "Failed to acquire Godot thread lock"),
    };

    // Disable thread safety checks while we use Godot APIs
    set_thread_safety_checks_enabled(false);

    // Load image from bytes
    let bytes = PackedByteArray::from_iter(body.iter().copied());
    let mut image = Image::new_gd();

    let load_result = if infer_mime::is_png(body) {
        image.load_png_from_buffer(&bytes)
    } else if infer_mime::is_jpeg(body) || infer_mime::is_jpeg2000(body) {
        image.load_jpg_from_buffer(&bytes)
    } else if infer_mime::is_webp(body) {
        image.load_webp_from_buffer(&bytes)
    } else if infer_mime::is_tga(body) {
        image.load_tga_from_buffer(&bytes)
    } else if infer_mime::is_bmp(body) {
        image.load_bmp_from_buffer(&bytes)
    } else {
        set_thread_safety_checks_enabled(true);
        return json_error_response(400, "Unsupported image format");
    };

    if load_result != godot::global::Error::OK {
        set_thread_safety_checks_enabled(true);
        return json_error_response(400, "Failed to load image");
    }

    let original_size = image.get_size();
    let max_size = state.texture_quality.to_max_size();

    // Create compressed texture for mobile
    let texture = create_compressed_texture(&mut image, max_size);

    // Save as .res file
    let res_path = state
        .cache_folder
        .join(format!("{}.res", hash))
        .to_string_lossy()
        .to_string();

    let err = ResourceSaver::singleton()
        .save_ex(&texture.upcast::<Resource>())
        .path(&res_path)
        .flags(SaverFlags::COMPRESS)
        .done();

    // Re-enable thread safety checks
    set_thread_safety_checks_enabled(true);

    if err != godot::global::Error::OK {
        return json_error_response(500, &format!("Failed to save texture: {:?}", err));
    }

    // Get file size
    let file_size = std::fs::metadata(&res_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // Register the asset
    let asset = CachedAsset {
        hash: hash.clone(),
        asset_type: AssetType::Texture,
        file_path: std::path::PathBuf::from(&res_path),
        original_name,
    };
    state.add_asset(asset);

    json_success_response(serde_json::json!({
        "hash": hash,
        "resource_path": format!("res://textures/{}.res", hash),
        "file_path": res_path,
        "cached": false,
        "status": "converted",
        "original_size": [original_size.x, original_size.y],
        "compressed_size": file_size,
    }))
}

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

    let response_body = serde_json::json!({ "error": message });

    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
        status_code, status_text, response_body.to_string().len(), response_body
    )
}
