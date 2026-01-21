//! ZIP packing using Godot's ZIPPacker.
//!
//! Packs processed assets into a ZIP file that can be loaded via
//! `ProjectSettings.load_resource_pack()`.

use godot::classes::file_access::ModeFlags;
use godot::classes::zip_packer::ZipAppend;
use godot::classes::{FileAccess, ZipPacker};
use godot::prelude::*;

use super::types::AssetType;

/// Pack processed assets into a ZIP file.
///
/// Creates a ZIP file at `{content_folder}{output_hash}-mobile.zip` containing
/// all processed assets. The paths inside the ZIP are structured as:
/// - `optimized-content/{hash}.scn` for GLTF assets (scene/wearable/emote)
/// - `optimized-content/{hash}` for texture files (no extension)
///
/// After `load_resource_pack()`, files become accessible at `res://optimized-content/...`
///
/// # Arguments
/// * `output_hash` - The hash to use for the ZIP filename
/// * `asset_paths` - List of (hash, optimized_path, asset_type) for each completed job
/// * `content_folder` - The content folder path (e.g., `~/.local/share/godot/.../content/`)
///
/// # Returns
/// * `Ok(String)` - The path to the created ZIP file
/// * `Err(anyhow::Error)` - If packing fails
pub fn pack_assets_to_zip(
    output_hash: &str,
    asset_paths: Vec<(String, String, AssetType)>,
    content_folder: &str,
) -> Result<String, anyhow::Error> {
    let zip_path = format!("{}{}-mobile.zip", content_folder, output_hash);

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
        let zip_internal_path = match asset_type {
            AssetType::Texture => format!("optimized-content/{}", hash),
            _ => format!("optimized-content/{}.scn", hash), // Scene, Wearable, Emote
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
