/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Content Provider V2 - New GLTF loading architecture
//!
//! This module provides a new approach to loading GLTF assets where:
//! - Background thread loads, processes, and saves scenes to disk
//! - Main thread loads saved scenes using Godot's ResourceLoader
//! - No Node3D objects are passed between threads
//! - Each caller gets their own Node3D instance via instantiate()

pub mod content_provider;
pub mod gltf_loader;
pub mod scene_saver;

pub use content_provider::ContentProvider2;
