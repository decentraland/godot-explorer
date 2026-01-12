/*
 * Texture to .res Conversion Handler
 *
 * Accepts image uploads and converts them to Godot .res files
 * with mobile optimizations (ETC2 compression).
 *
 * Uses the full ContentProvider infrastructure with promises, threading, and caching.
 * The wait_for_texture_promise function handles waiting and saving in a single flow.
 */

use godot::classes::resource_saver::SaverFlags;
use godot::classes::{Resource, ResourceSaver};
use godot::obj::InstanceId;
use godot::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::content::content_mapping::{ContentMappingAndUrl, DclContentMappingAndUrl};
use crate::content::texture::TextureEntry;
use crate::content::thread_safety::set_thread_safety_checks_enabled;
use crate::converter_server::server::{AssetType, CachedAsset, ConverterState};
use crate::dcl::common::content_entity::TypedIpfsRef;
use crate::godot_classes::promise::{Promise, PromiseError};

use super::json_success_response;

/// Result of texture conversion
struct TextureConversionResult {
    res_path: String,
    original_size: (i32, i32),
    file_size: u64,
}

/// Wait for texture promise and save the result - handles all Godot operations in polling loop
async fn wait_for_texture_promise(
    promise_id: InstanceId,
    semaphore: Arc<Semaphore>,
    res_path: String,
) -> Result<TextureConversionResult, String> {
    loop {
        // Acquire semaphore to safely access Godot API
        let _guard = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Semaphore error: {}", e))?;

        set_thread_safety_checks_enabled(false);

        // Access Godot types in a block that ends before await
        let poll_result: Result<Option<TextureConversionResult>, String> = (|| {
            // Recreate the Gd<Promise> from InstanceId each iteration
            let promise = match Gd::<Promise>::try_from_instance_id(promise_id) {
                Ok(p) => p,
                Err(_) => {
                    return Err("Promise no longer valid".to_string());
                }
            };

            let promise_ref = promise.bind();
            let is_resolved = promise_ref.is_resolved();
            let is_rejected = promise_ref.is_rejected();

            if is_resolved {
                if is_rejected {
                    let data = promise_ref.get_data();
                    let error_str = match data.try_to::<Gd<PromiseError>>() {
                        Ok(err) => err.bind().get_error().to_string(),
                        Err(_) => "Unknown error".to_string(),
                    };
                    return Err(error_str);
                }

                // Get the TextureEntry from the promise data
                let data = promise_ref.get_data();
                drop(promise_ref); // Drop the borrow before we work with the data

                let texture_entry: Gd<TextureEntry> = match data.try_to() {
                    Ok(te) => te,
                    Err(_) => {
                        return Err(
                            "Invalid texture entry returned from ContentProvider".to_string()
                        );
                    }
                };

                let te_ref = texture_entry.bind();
                let original_size = (te_ref.original_size.x, te_ref.original_size.y);
                let texture = te_ref.texture.clone();
                drop(te_ref);

                // Save as .res file while we still have thread safety disabled
                let err = ResourceSaver::singleton()
                    .save_ex(&texture.upcast::<Resource>())
                    .path(&res_path)
                    .flags(SaverFlags::COMPRESS)
                    .done();

                if err != godot::global::Error::OK {
                    return Err(format!("Failed to save texture: {:?}", err));
                }

                // Get file size
                let file_size = std::fs::metadata(&res_path).map(|m| m.len()).unwrap_or(0);

                return Ok(Some(TextureConversionResult {
                    res_path: res_path.clone(),
                    original_size,
                    file_size,
                }));
            }

            Ok(None) // Not yet resolved
        })();

        set_thread_safety_checks_enabled(true);
        drop(_guard);

        // Now check the result without holding any Godot types
        match poll_result {
            Err(e) => return Err(e),
            Ok(Some(result)) => return Ok(result),
            Ok(None) => {
                // Not resolved yet, continue polling
            }
        }

        // Small delay before checking again
        tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
    }
}

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

    // Check if already converted in our local cache
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

    // Save the uploaded file to cache folder (so ContentProvider can access it via file:// URL)
    let texture_path = state.cache_folder.join(&hash);
    if let Err(e) = std::fs::write(&texture_path, body) {
        return json_error_response(500, &format!("Failed to save file: {}", e));
    }

    // Create content mapping for the uploaded file
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
            std::fs::remove_file(&texture_path).ok();
            return json_error_response(500, "Failed to acquire Godot thread lock");
        }
    };

    // Disable thread safety checks while we use Godot APIs
    set_thread_safety_checks_enabled(false);

    // Get ContentProvider, call fetch_texture_by_hash, and extract InstanceId
    let promise_id = {
        let content_provider = match state.get_content_provider() {
            Some(cp) => cp,
            None => {
                set_thread_safety_checks_enabled(true);
                std::fs::remove_file(&texture_path).ok();
                return json_error_response(500, "ContentProvider not available");
            }
        };

        // Create DclContentMappingAndUrl from our Rust struct
        let dcl_content_mapping = DclContentMappingAndUrl::from_ref(content_mapping);

        // Call the ContentProvider's fetch_texture_by_hash method
        let mut cp = content_provider.clone();
        let promise = cp
            .bind_mut()
            .fetch_texture_by_hash(GString::from(&hash), dcl_content_mapping);

        // Extract InstanceId before the Gd<Promise> goes out of scope
        promise.instance_id()
    };

    // Re-enable thread safety checks after getting the promise
    set_thread_safety_checks_enabled(true);

    // Release guard before waiting
    drop(_guard);

    // Compute the res path for saving
    let res_path = state
        .cache_folder
        .join(format!("{}.res", hash))
        .to_string_lossy()
        .to_string();

    // Wait for the promise to resolve and save the texture
    let result = wait_for_texture_promise(
        promise_id,
        state.godot_single_thread.clone(),
        res_path.clone(),
    )
    .await;

    match result {
        Ok(conversion_result) => {
            // Register the asset in our local cache
            let asset = CachedAsset {
                hash: hash.clone(),
                asset_type: AssetType::Texture,
                file_path: std::path::PathBuf::from(&conversion_result.res_path),
                original_name,
            };
            state.add_asset(asset);

            json_success_response(serde_json::json!({
                "hash": hash,
                "resource_path": format!("res://textures/{}.res", hash),
                "file_path": conversion_result.res_path,
                "cached": false,
                "status": "converted",
                "original_size": [conversion_result.original_size.0, conversion_result.original_size.1],
                "compressed_size": conversion_result.file_size,
            }))
        }
        Err(e) => {
            // Clean up the saved texture file on error
            std::fs::remove_file(&texture_path).ok();
            json_error_response(500, &format!("Texture conversion failed: {}", e))
        }
    }
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
