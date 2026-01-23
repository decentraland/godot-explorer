//! ZIP packing using Godot's ZIPPacker.
//!
//! Packs processed assets into a ZIP file that can be loaded via
//! `ProjectSettings.load_resource_pack()`.

use std::collections::HashSet;

use godot::classes::file_access::ModeFlags;
use godot::classes::zip_packer::ZipAppend;
use godot::classes::{FileAccess, ZipPacker};
use godot::prelude::*;

use super::types::{AssetType, SceneOptimizationMetadata};

/// Pack processed assets into a ZIP file.
///
/// Creates a ZIP file at `{output_folder}{output_hash}-mobile.zip` containing
/// all processed assets. The paths inside the ZIP are structured as:
/// - `glbs/{hash}.scn` for GLTF assets (scene/wearable/emote)
/// - `content/{hash}.res` for texture files
///
/// After `load_resource_pack()`, files become accessible at `res://glbs/...` and `res://content/...`
///
/// # Arguments
/// * `output_hash` - The hash to use for the ZIP filename
/// * `asset_paths` - List of (hash, optimized_path, asset_type) for each completed job
/// * `output_folder` - The output folder path for ZIP files (e.g., `./output/`)
///
/// # Returns
/// * `Ok(String)` - The path to the created ZIP file
/// * `Err(anyhow::Error)` - If packing fails
pub fn pack_assets_to_zip(
    output_hash: &str,
    asset_paths: Vec<(String, String, AssetType)>,
    output_folder: &str,
) -> Result<String, anyhow::Error> {
    let zip_path = format!("{}{}-mobile.zip", output_folder, output_hash);

    tracing::info!("Packing {} assets to ZIP: {}", asset_paths.len(), zip_path);

    let mut packer = ZipPacker::new_gd();

    // Open the ZIP file for writing (CREATE mode)
    let err = packer
        .open_ex(&GString::from(&zip_path))
        .append(ZipAppend::CREATE)
        .done();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!(
            "Failed to open ZIP file for writing: {:?}",
            err
        ));
    }

    for (hash, path, asset_type) in asset_paths {
        // Read the file contents
        let file_access = FileAccess::open(&GString::from(&path), ModeFlags::READ);
        let Some(mut file) = file_access else {
            tracing::warn!("Failed to open file for packing: {}", path);
            continue;
        };

        let data = file.get_buffer(file.get_length() as i64);
        file.close();

        // Determine the path inside the ZIP
        // No res:// prefix - Godot adds it when loading the resource pack
        // GLTFs go to glbs/, textures go to content/
        let zip_internal_path = match asset_type {
            AssetType::Texture => format!("content/{}.res", hash),
            _ => format!("glbs/{}.scn", hash), // Scene, Wearable, Emote
        };

        tracing::debug!("Adding to ZIP: {} -> {}", path, zip_internal_path);

        // Start file entry in ZIP
        let err = packer.start_file(&GString::from(&zip_internal_path));
        if err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to start file entry in ZIP for {}: {:?}",
                zip_internal_path,
                err
            );
            continue;
        }

        // Write file data
        let err = packer.write_file(&data);
        if err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to write file data to ZIP for {}: {:?}",
                zip_internal_path,
                err
            );
            // Try to close the file entry anyway
            let _ = packer.close_file();
            continue;
        }

        // Close file entry
        let err = packer.close_file();
        if err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to close file entry in ZIP for {}: {:?}",
                zip_internal_path,
                err
            );
        }
    }

    // Close the ZIP file
    let err = packer.close();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!("Failed to close ZIP file: {:?}", err));
    }

    tracing::info!("ZIP file created successfully: {}", zip_path);

    Ok(zip_path)
}

/// Pack scene assets into a ZIP file with metadata and optional selective packing.
///
/// Creates a ZIP file at `{output_folder}{output_hash}-mobile.zip` containing:
/// - `metadata.json` with optimization results
/// - `glbs/{hash}.scn` for GLTF assets
/// - `content/{hash}.res` for texture assets
///
/// # Arguments
/// * `output_hash` - The hash to use for the ZIP filename
/// * `asset_paths` - List of (hash, optimized_path, asset_type) for each completed job
/// * `pack_filter` - Optional set of hashes to include (None = include all)
/// * `metadata` - Scene optimization metadata to include in the ZIP
/// * `output_folder` - The output folder path for ZIP files (e.g., `./output/`)
///
/// # Returns
/// * `Ok(String)` - The path to the created ZIP file
/// * `Err(anyhow::Error)` - If packing fails
pub fn pack_scene_assets_to_zip(
    output_hash: &str,
    asset_paths: Vec<(String, String, AssetType)>,
    pack_filter: Option<&HashSet<String>>,
    metadata: SceneOptimizationMetadata,
    output_folder: &str,
) -> Result<String, anyhow::Error> {
    let zip_path = format!("{}{}-mobile.zip", output_folder, output_hash);

    // Filter assets if pack_filter is provided
    let assets_to_pack: Vec<_> = if let Some(filter) = pack_filter {
        asset_paths
            .into_iter()
            .filter(|(hash, _, _)| filter.contains(hash))
            .collect()
    } else {
        asset_paths
    };

    tracing::info!(
        "Packing {} assets to ZIP with metadata: {}",
        assets_to_pack.len(),
        zip_path
    );

    let mut packer = ZipPacker::new_gd();

    // Open the ZIP file for writing (CREATE mode)
    let err = packer
        .open_ex(&GString::from(&zip_path))
        .append(ZipAppend::CREATE)
        .done();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!(
            "Failed to open ZIP file for writing: {:?}",
            err
        ));
    }

    // Add {scene_id}-optimized.json metadata file
    let metadata_filename = format!("{}-optimized.json", output_hash);
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;

    let err = packer.start_file(&GString::from(&metadata_filename));
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!(
            "Failed to start {} entry: {:?}",
            metadata_filename,
            err
        ));
    }

    let metadata_bytes = PackedByteArray::from(metadata_json.as_bytes());
    let err = packer.write_file(&metadata_bytes);
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!(
            "Failed to write {}: {:?}",
            metadata_filename,
            err
        ));
    }

    let err = packer.close_file();
    if err != godot::global::Error::OK {
        tracing::warn!("Failed to close {} entry: {:?}", metadata_filename, err);
    }

    // Add asset files
    for (hash, path, asset_type) in assets_to_pack {
        // Read the file contents
        let file_access = FileAccess::open(&GString::from(&path), ModeFlags::READ);
        let Some(mut file) = file_access else {
            tracing::warn!("Failed to open file for packing: {}", path);
            continue;
        };

        let data = file.get_buffer(file.get_length() as i64);
        file.close();

        // Determine the path inside the ZIP
        // GLTFs go to glbs/, textures go to content/
        let zip_internal_path = match asset_type {
            AssetType::Texture => format!("content/{}.res", hash),
            _ => format!("glbs/{}.scn", hash),
        };

        tracing::debug!("Adding to ZIP: {} -> {}", path, zip_internal_path);

        // Start file entry in ZIP
        let err = packer.start_file(&GString::from(&zip_internal_path));
        if err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to start file entry in ZIP for {}: {:?}",
                zip_internal_path,
                err
            );
            continue;
        }

        // Write file data
        let err = packer.write_file(&data);
        if err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to write file data to ZIP for {}: {:?}",
                zip_internal_path,
                err
            );
            let _ = packer.close_file();
            continue;
        }

        // Close file entry
        let err = packer.close_file();
        if err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to close file entry in ZIP for {}: {:?}",
                zip_internal_path,
                err
            );
        }
    }

    // Close the ZIP file
    let err = packer.close();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!("Failed to close ZIP file: {:?}", err));
    }

    tracing::info!("Scene ZIP file created successfully: {}", zip_path);

    Ok(zip_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a Godot runtime to actually run.
    // They're here to document the expected behavior.

    #[test]
    #[ignore = "requires Godot runtime"]
    fn test_pack_assets_to_zip() {
        // Would test creating a ZIP with mock assets
    }
}
