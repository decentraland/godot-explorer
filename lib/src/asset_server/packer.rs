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

/// Compute the path of a file inside the ZIP.
/// No res:// prefix - Godot adds it when loading the resource pack.
/// GLTFs go to glbs/, textures go to content/ keeping their on-disk file name
/// (`{hash}_q{N}.res` for quality variants).
fn zip_internal_path(hash: &str, path: &str, asset_type: AssetType) -> String {
    match asset_type {
        AssetType::Texture => {
            let basename = path.rsplit('/').next().unwrap_or(path);
            format!("content/{}", basename)
        }
        _ => format!("glbs/{}.scn", hash), // Scene, Wearable, Emote
    }
}

/// Add a single file to an open ZIP, logging and skipping on failure.
fn add_file_to_zip(packer: &mut Gd<ZipPacker>, hash: &str, path: &str, asset_type: AssetType) {
    // Read the file contents
    let file_access = FileAccess::open(&GString::from(path), ModeFlags::READ);
    let Some(mut file) = file_access else {
        tracing::warn!("Failed to open file for packing: {}", path);
        return;
    };

    let data = file.get_buffer(file.get_length() as i64);
    file.close();

    let zip_internal_path = zip_internal_path(hash, path, asset_type);

    tracing::debug!("Adding to ZIP: {} -> {}", path, zip_internal_path);

    // Start file entry in ZIP
    let err = packer.start_file(&GString::from(&zip_internal_path));
    if err != godot::global::Error::OK {
        tracing::warn!(
            "Failed to start file entry in ZIP for {}: {:?}",
            zip_internal_path,
            err
        );
        return;
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
        return;
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

/// Pack processed assets into a ZIP file.
///
/// Creates a ZIP file at `{output_folder}{output_hash}-mobile.zip` containing
/// all processed assets. The paths inside the ZIP are structured as:
/// - `glbs/{hash}.scn` for GLTF assets (scene/wearable/emote)
/// - `content/{hash}_q{N}.res` for texture quality variants
///
/// After `load_resource_pack()`, files become accessible at `res://glbs/...` and `res://content/...`
///
/// # Arguments
/// * `output_hash` - The hash to use for the ZIP filename
/// * `asset_paths` - List of (hash, file_paths, asset_type) for each completed job
/// * `output_folder` - The output folder path for ZIP files (e.g., `./output/`)
///
/// # Returns
/// * `Ok(String)` - The path to the created ZIP file
/// * `Err(anyhow::Error)` - If packing fails
pub fn pack_assets_to_zip(
    output_hash: &str,
    asset_paths: Vec<(String, Vec<String>, AssetType)>,
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

    for (hash, paths, asset_type) in asset_paths {
        for path in &paths {
            add_file_to_zip(&mut packer, &hash, path, asset_type);
        }
    }

    // Close the ZIP file
    let err = packer.close();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!("Failed to close ZIP file: {:?}", err));
    }

    Ok(zip_path)
}

/// Pack a single asset into its own ZIP file.
///
/// Creates `{output_folder}{hash}-mobile.zip` containing the asset's files:
/// a single `.scn` for GLTFs, or one `.res` per quality variant for textures.
pub fn pack_single_asset_to_zip(
    hash: &str,
    file_paths: &[String],
    asset_type: AssetType,
    output_folder: &str,
) -> Result<String, anyhow::Error> {
    let zip_path = format!("{}{}-mobile.zip", output_folder, hash);

    tracing::debug!("Packing single asset {} to ZIP: {}", hash, zip_path);

    let mut packer = ZipPacker::new_gd();

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

    for path in file_paths {
        let file_access = FileAccess::open(&GString::from(path), ModeFlags::READ);
        let Some(mut file) = file_access else {
            packer.close();
            return Err(anyhow::anyhow!("Failed to open file for packing: {}", path));
        };

        let data = file.get_buffer(file.get_length() as i64);
        file.close();

        let internal_path = zip_internal_path(hash, path, asset_type);

        let err = packer.start_file(&GString::from(&internal_path));
        if err != godot::global::Error::OK {
            packer.close();
            return Err(anyhow::anyhow!(
                "Failed to start file entry in ZIP: {:?}",
                err
            ));
        }

        let err = packer.write_file(&data);
        if err != godot::global::Error::OK {
            let _ = packer.close_file();
            packer.close();
            return Err(anyhow::anyhow!(
                "Failed to write file data to ZIP: {:?}",
                err
            ));
        }

        let _ = packer.close_file();
    }

    let err = packer.close();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!("Failed to close ZIP file: {:?}", err));
    }

    Ok(zip_path)
}

/// Pack scene metadata and optionally preloaded assets into a ZIP file.
///
/// Creates a ZIP file at `{output_folder}{output_hash}-mobile.zip` containing:
/// - `{output_hash}-optimized.json` with optimization metadata (always)
/// - Assets whose hashes are in `preloaded_hashes` (if any)
///
/// # Arguments
/// * `output_hash` - The hash to use for the ZIP filename
/// * `asset_paths` - List of (hash, file_paths, asset_type) for each completed job
/// * `preloaded_hashes` - Optional set of hashes to include alongside metadata
/// * `metadata` - Scene optimization metadata to include in the ZIP
/// * `output_folder` - The output folder path for ZIP files (e.g., `./output/`)
pub fn pack_scene_assets_to_zip(
    output_hash: &str,
    asset_paths: Vec<(String, Vec<String>, AssetType)>,
    preloaded_hashes: Option<&HashSet<String>>,
    metadata: SceneOptimizationMetadata,
    output_folder: &str,
) -> Result<String, anyhow::Error> {
    let zip_path = format!("{}{}-mobile.zip", output_folder, output_hash);

    // Filter assets to only those in preloaded_hashes (if specified)
    let assets_to_pack: Vec<_> = match preloaded_hashes {
        Some(hashes) if !hashes.is_empty() => asset_paths
            .into_iter()
            .filter(|(hash, _, _)| hashes.contains(hash))
            .collect(),
        _ => Vec::new(), // No preloaded hashes or empty set = metadata only
    };

    tracing::info!(
        "Packing scene ZIP with metadata + {} preloaded assets: {}",
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

    // Always add metadata JSON
    {
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
    }

    // Add asset files
    for (hash, paths, asset_type) in assets_to_pack {
        for path in &paths {
            add_file_to_zip(&mut packer, &hash, path, asset_type);
        }
    }

    // Close the ZIP file
    let err = packer.close();
    if err != godot::global::Error::OK {
        return Err(anyhow::anyhow!("Failed to close ZIP file: {:?}", err));
    }

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
