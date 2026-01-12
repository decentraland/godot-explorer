/*
 * ZIP Builder using Godot's ZIPPacker
 *
 * Creates ZIP files that can be loaded via ProjectSettings.load_resource_pack()
 *
 * NOTE: ZIPPacker operations must run on the Godot main thread.
 * This module provides the interface; actual implementation will be called
 * from the main thread via deferred calls.
 */

use super::server::{AssetType, CachedAsset};

/// Create manifest.json content for the ZIP package
pub fn create_manifest_json(scene_id: &str, assets: &[CachedAsset]) -> String {
    let asset_entries: Vec<serde_json::Value> = assets
        .iter()
        .map(|asset| {
            let (asset_type_str, path) = match asset.asset_type {
                AssetType::Scene => ("scene", format!("res://glbs/{}.scn", asset.hash)),
                AssetType::Texture => ("texture", format!("res://textures/{}.res", asset.hash)),
            };
            serde_json::json!({
                "hash": asset.hash,
                "type": asset_type_str,
                "path": path,
                "original_name": asset.original_name,
            })
        })
        .collect();

    let manifest = serde_json::json!({
        "scene_id": scene_id,
        "version": "1.0",
        "platform": "mobile",
        "compression": "etc2",
        "assets": asset_entries,
    });

    serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string())
}

/// Get the expected ZIP path structure for an asset
pub fn get_zip_path_for_asset(asset: &CachedAsset) -> String {
    match asset.asset_type {
        AssetType::Scene => format!("glbs/{}.scn", asset.hash),
        AssetType::Texture => format!("textures/{}.res", asset.hash),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_create_manifest_json() {
        let assets = vec![
            CachedAsset {
                hash: "abc123".to_string(),
                asset_type: AssetType::Scene,
                file_path: PathBuf::from("/tmp/abc123.scn"),
                original_name: "model.glb".to_string(),
            },
            CachedAsset {
                hash: "def456".to_string(),
                asset_type: AssetType::Texture,
                file_path: PathBuf::from("/tmp/def456.res"),
                original_name: "texture.png".to_string(),
            },
        ];

        let manifest = create_manifest_json("test-scene", &assets);
        assert!(manifest.contains("\"scene_id\": \"test-scene\""));
        assert!(manifest.contains("abc123"));
        assert!(manifest.contains("def456"));
    }

    #[test]
    fn test_get_zip_path_for_asset() {
        let scene_asset = CachedAsset {
            hash: "abc123".to_string(),
            asset_type: AssetType::Scene,
            file_path: PathBuf::from("/tmp/abc123.scn"),
            original_name: "model.glb".to_string(),
        };

        let texture_asset = CachedAsset {
            hash: "def456".to_string(),
            asset_type: AssetType::Texture,
            file_path: PathBuf::from("/tmp/def456.res"),
            original_name: "texture.png".to_string(),
        };

        assert_eq!(get_zip_path_for_asset(&scene_asset), "glbs/abc123.scn");
        assert_eq!(
            get_zip_path_for_asset(&texture_asset),
            "textures/def456.res"
        );
    }
}
