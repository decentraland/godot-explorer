/*
 * HTTP Request Handlers for Converter Server
 */

mod convert_gltf;
mod convert_texture;
mod package_scene;

use std::collections::HashMap;
use std::sync::Arc;

use super::server::ConverterState;

/// Health check endpoint
pub async fn health_handler(state: &Arc<ConverterState>) -> String {
    let asset_count = state.assets.read().map(|a| a.len()).unwrap_or(0);

    let response_body = serde_json::json!({
        "status": "ok",
        "port": state.port,
        "cache_folder": state.cache_folder.to_string_lossy(),
        "cached_assets": asset_count,
    });

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\r\n{}",
        response_body.to_string().len(),
        response_body
    )
}

/// Get asset by hash
pub async fn get_asset_handler(state: &Arc<ConverterState>, hash: &str) -> String {
    match state.get_asset(hash) {
        Some(asset) => {
            if let Ok(data) = std::fs::read(&asset.file_path) {
                let content_type = match asset.asset_type {
                    super::server::AssetType::Scene => "application/octet-stream",
                    super::server::AssetType::Texture => "application/octet-stream",
                };

                format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: {}\r\n\
                     Content-Disposition: attachment; filename=\"{}\"\r\n\
                     Access-Control-Allow-Origin: *\r\n\
                     Content-Length: {}\r\n\r\n",
                    content_type,
                    asset
                        .file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    data.len()
                ) + &String::from_utf8_lossy(&data)
            } else {
                json_error_response(500, "Failed to read asset file")
            }
        }
        None => json_error_response(404, "Asset not found"),
    }
}

/// Convert GLTF/GLB to .scn
pub async fn convert_gltf_handler(
    state: &Arc<ConverterState>,
    headers: &HashMap<String, String>,
    body: &[u8],
) -> String {
    convert_gltf::handle(state, headers, body).await
}

/// Convert texture to .res
pub async fn convert_texture_handler(
    state: &Arc<ConverterState>,
    headers: &HashMap<String, String>,
    body: &[u8],
) -> String {
    convert_texture::handle(state, headers, body).await
}

/// Package scene assets into ZIP
pub async fn package_scene_handler(
    state: &Arc<ConverterState>,
    headers: &HashMap<String, String>,
    body: &[u8],
) -> String {
    package_scene::handle(state, headers, body).await
}

/// Clear cache
pub async fn clear_cache_handler(state: &Arc<ConverterState>) -> String {
    let mut cleared_count = 0;
    let mut freed_bytes: u64 = 0;

    if let Ok(mut assets) = state.assets.write() {
        for (_, asset) in assets.drain() {
            if let Ok(metadata) = std::fs::metadata(&asset.file_path) {
                freed_bytes += metadata.len();
            }
            std::fs::remove_file(&asset.file_path).ok();
            cleared_count += 1;
        }
    }

    let response_body = serde_json::json!({
        "cleared": true,
        "assets_removed": cleared_count,
        "freed_bytes": freed_bytes,
    });

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\r\n{}",
        response_body.to_string().len(),
        response_body
    )
}

/// 404 Not Found
pub async fn not_found_handler() -> String {
    json_error_response(404, "Not found")
}

/// Helper to create JSON error responses
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

/// Helper to create JSON success responses
pub fn json_success_response(data: serde_json::Value) -> String {
    let response_body = data.to_string();

    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    )
}

/// Helper to create binary responses
pub fn binary_response(data: &[u8], content_type: &str, filename: &str) -> String {
    // Note: This is a simplified implementation that works for smaller files
    // For large files, we'd want to stream the response
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {}\r\n\
         Content-Disposition: attachment; filename=\"{}\"\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\r\n",
        content_type,
        filename,
        data.len()
    );

    // For binary data, we need to handle this differently
    // This is a placeholder - actual binary handling would need raw bytes
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error_response_400() {
        let response = json_error_response(400, "Invalid request");
        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("Content-Type: application/json"));
        assert!(response.contains("Access-Control-Allow-Origin: *"));
        assert!(response.contains("\"error\":\"Invalid request\""));
    }

    #[test]
    fn test_json_error_response_404() {
        let response = json_error_response(404, "Not found");
        assert!(response.starts_with("HTTP/1.1 404 Not Found"));
        assert!(response.contains("\"error\":\"Not found\""));
    }

    #[test]
    fn test_json_error_response_500() {
        let response = json_error_response(500, "Server error");
        assert!(response.starts_with("HTTP/1.1 500 Internal Server Error"));
        assert!(response.contains("\"error\":\"Server error\""));
    }

    #[test]
    fn test_json_success_response() {
        let data = serde_json::json!({
            "hash": "abc123",
            "status": "converted"
        });
        let response = json_success_response(data);

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/json"));
        assert!(response.contains("Access-Control-Allow-Origin: *"));
        assert!(response.contains("abc123"));
        assert!(response.contains("converted"));
    }

    #[test]
    fn test_json_success_response_content_length() {
        let data = serde_json::json!({"key": "value"});
        let response = json_success_response(data.clone());

        let body = data.to_string();
        let expected_length = format!("Content-Length: {}", body.len());
        assert!(response.contains(&expected_length));
    }

    #[test]
    fn test_binary_response_headers() {
        let data = b"test binary data";
        let response = binary_response(data, "application/octet-stream", "test.bin");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/octet-stream"));
        assert!(response.contains("Content-Disposition: attachment; filename=\"test.bin\""));
        assert!(response.contains("Content-Length: 16"));
    }
}
