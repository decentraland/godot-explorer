/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Scene saving utilities for content_v2
//!
//! Provides functions to save Node3D as PackedScene to disk.
//! Scenes are stored in the same cache folder as other content,
//! using the hash as filename with .scn extension.

use godot::classes::{PackedScene, ResourceSaver};
use godot::prelude::*;

/// Saves a Node3D as a PackedScene to the specified file path
///
/// # Arguments
/// * `node` - The Node3D to save
/// * `file_path` - The full path to save to (e.g., "/path/to/cache/hash.scn")
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message on failure
pub fn save_node_as_scene(node: Gd<Node3D>, file_path: &str) -> Result<(), String> {
    let mut packed = PackedScene::new_gd();

    let err = packed.pack(node.clone().upcast());
    if err != godot::global::Error::OK {
        return Err(format!("Failed to pack scene: {:?}", err));
    }

    let err = ResourceSaver::singleton()
        .save_ex(packed.upcast())
        .path(file_path.into())
        .done();
    if err != godot::global::Error::OK {
        return Err(format!("Failed to save scene to {}: {:?}", file_path, err));
    }

    Ok(())
}

/// Gets the absolute path for a cached GLTF scene by its hash
///
/// # Arguments
/// * `content_folder` - The cache folder path (e.g., "/path/to/cache/")
/// * `hash` - The content hash
pub fn get_scene_path_for_hash(content_folder: &str, hash: &str) -> String {
    format!("{}{}.scn", content_folder, hash)
}
