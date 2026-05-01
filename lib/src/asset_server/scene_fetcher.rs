//! Scene entity fetching and asset discovery.
//!
//! This module fetches scene entities from Decentraland content servers
//! and discovers all assets (GLTFs and textures) that need to be processed.

use std::collections::HashMap;

use reqwest::Client;

use crate::dcl::common::content_entity::EntityDefinitionJson;

use super::types::AssetType;

/// A discovered asset from a scene entity.
#[derive(Debug, Clone)]
pub struct DiscoveredAsset {
    /// Content hash of the asset
    pub hash: String,
    /// File path in the content mapping
    pub file_path: String,
    /// Type of asset
    pub asset_type: AssetType,
    /// URL to fetch the asset from
    pub url: String,
}

/// Result of fetching and parsing a scene entity.
#[derive(Debug)]
pub struct SceneEntityAssets {
    /// The scene entity hash
    pub scene_hash: String,
    /// Base URL for content fetching
    pub content_base_url: String,
    /// All discovered GLTF assets
    pub gltfs: Vec<DiscoveredAsset>,
    /// All discovered texture assets
    pub textures: Vec<DiscoveredAsset>,
    /// Full content mapping (file_path -> hash)
    pub content_mapping: HashMap<String, String>,
}

impl SceneEntityAssets {
    /// Get all discovered assets (GLTFs + textures).
    pub fn all_assets(&self) -> Vec<&DiscoveredAsset> {
        self.gltfs.iter().chain(self.textures.iter()).collect()
    }

    /// Get the total number of assets.
    pub fn total_count(&self) -> usize {
        self.gltfs.len() + self.textures.len()
    }
}

/// Image file extensions that should be processed as textures.
const IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tga", ".ktx", ".ktx2",
];

/// GLTF file extensions.
const GLTF_EXTENSIONS: &[&str] = &[".glb", ".gltf"];

/// Check if a file path is an image file.
fn is_image_file(file_path: &str) -> bool {
    let lower = file_path.to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Check if a file path is a GLTF file.
fn is_gltf_file(file_path: &str) -> bool {
    let lower = file_path.to_lowercase();
    GLTF_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Fetch a scene entity from a content server and discover all assets.
///
/// # Arguments
/// * `content_base_url` - Base URL for the content server (e.g., "https://peer.decentraland.org/content/")
///                        Can also include "contents/" suffix
/// * `scene_hash` - The hash of the scene entity to fetch
///
/// # Returns
/// * `Ok(SceneEntityAssets)` - Discovered assets and content mapping
/// * `Err(anyhow::Error)` - If fetching or parsing fails
pub async fn fetch_scene_entity(
    content_base_url: &str,
    scene_hash: &str,
) -> Result<SceneEntityAssets, anyhow::Error> {
    let client = Client::builder()
        .user_agent("decentraland-godot-explorer")
        .build()?;

    // Normalize base URL to the content server root (e.g., https://peer.decentraland.org/content/)
    let content_root = content_base_url
        .trim_end_matches('/')
        .trim_end_matches("contents")
        .trim_end_matches('/');
    let content_root = format!("{}/", content_root);

    // The base_url used for ContentMappingAndUrl should include "contents/" since
    // the GLTF loader constructs URLs as {base_url}{hash}
    let base_url = format!("{}contents/", content_root);

    // Fetch the entity definition
    let entity_url = format!("{}{}", base_url, scene_hash);
    tracing::info!("Fetching scene entity from: {}", entity_url);

    let response = client.get(&entity_url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch scene entity: HTTP {}",
            response.status()
        ));
    }

    let entity_bytes = response.bytes().await?;
    let entity: EntityDefinitionJson = serde_json::from_slice(&entity_bytes)?;

    // Build content mapping and discover assets
    let mut content_mapping = HashMap::new();
    let mut gltfs = Vec::new();
    let mut textures = Vec::new();

    for content_item in &entity.content {
        let file_path = content_item.file.clone();
        let hash = content_item.hash.clone();
        let url = format!("{}{}", base_url, hash);

        content_mapping.insert(file_path.to_lowercase(), hash.clone());

        if is_gltf_file(&file_path) {
            gltfs.push(DiscoveredAsset {
                hash,
                file_path,
                asset_type: AssetType::Scene,
                url,
            });
        } else if is_image_file(&file_path) {
            textures.push(DiscoveredAsset {
                hash,
                file_path,
                asset_type: AssetType::Texture,
                url,
            });
        }
        // Skip other file types (audio, JS, etc.)
    }

    tracing::info!(
        "Discovered {} GLTFs and {} textures in scene {}",
        gltfs.len(),
        textures.len(),
        scene_hash
    );

    Ok(SceneEntityAssets {
        scene_hash: scene_hash.to_string(),
        content_base_url: base_url,
        gltfs,
        textures,
        content_mapping,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("texture.png"));
        assert!(is_image_file("TEXTURE.PNG"));
        assert!(is_image_file("path/to/image.jpg"));
        assert!(is_image_file("file.webp"));
        assert!(!is_image_file("model.glb"));
        assert!(!is_image_file("script.js"));
    }

    #[test]
    fn test_is_gltf_file() {
        assert!(is_gltf_file("model.glb"));
        assert!(is_gltf_file("MODEL.GLB"));
        assert!(is_gltf_file("path/to/scene.gltf"));
        assert!(!is_gltf_file("texture.png"));
        assert!(!is_gltf_file("script.js"));
    }
}
