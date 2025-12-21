/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Scene saving utilities for content_v2
//!
//! Provides functions to save Node3D as PackedScene to disk
//! and manage the glbs cache directory.

use godot::classes::{DirAccess, PackedScene, ResourceSaver};
use godot::prelude::*;

/// The base path for cached GLTF scenes
pub const GLBS_CACHE_PATH: &str = "user://content/glbs/";

/// Ensures the glbs cache directory exists
///
/// Creates `user://content/glbs/` if it doesn't exist.
/// Returns the path to the directory.
pub fn ensure_glbs_directory() -> Result<String, String> {
    let mut dir =
        DirAccess::open("user://".into()).ok_or("Cannot access user:// directory".to_string())?;

    let err = dir.make_dir_recursive("content/glbs".into());
    if err != godot::global::Error::OK && err != godot::global::Error::ERR_ALREADY_EXISTS {
        return Err(format!("Failed to create glbs directory: {:?}", err));
    }

    Ok(GLBS_CACHE_PATH.to_string())
}

/// Saves a Node3D as a PackedScene to the specified file path
///
/// # Arguments
/// * `node` - The Node3D to save
/// * `file_path` - The full path to save to (e.g., "user://content/glbs/hash.tscn")
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

/// Gets the full path for a cached GLTF scene by its hash
pub fn get_scene_path_for_hash(hash: &str) -> String {
    format!("{}{}.scn", GLBS_CACHE_PATH, hash)
}

/// Checks if a cached scene exists for the given hash
pub fn cached_scene_exists(hash: &str) -> bool {
    let path = get_scene_path_for_hash(hash);
    godot::classes::FileAccess::file_exists(path.into())
}
