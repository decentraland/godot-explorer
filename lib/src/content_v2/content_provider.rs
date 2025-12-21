/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! ContentProvider2 - New content loading architecture
//!
//! This provider uses a different approach:
//! - Background thread loads, processes, and saves scenes to disk
//! - Main thread loads saved scenes using Godot's ResourceLoader
//! - No Node3D objects are passed between threads

use std::{collections::HashSet, sync::Arc};

use godot::prelude::*;
use tokio::sync::Semaphore;

use crate::{
    content::{content_mapping::DclContentMappingAndUrl, resource_provider::ResourceProvider},
    godot_classes::dcl_config::{DclConfig, TextureQuality},
    scene_runner::tokio_runtime::TokioRuntime,
};

#[cfg(feature = "use_resource_tracking")]
use crate::content::resource_download_tracking::ResourceDownloadTracking;

use super::gltf_loader::load_and_save_gltf;
use super::scene_saver::get_scene_path_for_hash;

/// Context passed to async operations
#[derive(Clone)]
pub struct ContentProvider2Context {
    pub content_folder: Arc<String>,
    pub resource_provider: Arc<ResourceProvider>,
    pub godot_single_thread: Arc<Semaphore>,
    pub texture_quality: TextureQuality,
}

unsafe impl Send for ContentProvider2Context {}

/// ContentProvider2 - New GLTF loading system
///
/// Uses signals instead of promises for async communication.
/// Saves processed scenes to disk and loads them via ResourceLoader.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct ContentProvider2 {
    base: Base<Node>,

    content_folder: Arc<String>,
    resource_provider: Arc<ResourceProvider>,
    #[cfg(feature = "use_resource_tracking")]
    #[allow(dead_code)]
    resource_download_tracking: Arc<ResourceDownloadTracking>,
    godot_single_thread: Arc<Semaphore>,
    texture_quality: TextureQuality,

    /// Set of hashes currently being loaded (to prevent duplicate loads)
    loading_hashes: HashSet<String>,
}

#[godot_api]
impl INode for ContentProvider2 {
    fn init(base: Base<Node>) -> Self {
        let content_folder = Arc::new(format!(
            "{}/content/",
            godot::engine::Os::singleton().get_user_data_dir()
        ));

        #[cfg(feature = "use_resource_tracking")]
        let resource_download_tracking =
            Arc::new(crate::content::resource_download_tracking::ResourceDownloadTracking::new());

        Self {
            base,
            resource_provider: Arc::new(ResourceProvider::new(
                content_folder.clone().as_str(),
                2048 * 1000 * 1000, // 2GB cache
                32,                 // max concurrent downloads
                #[cfg(feature = "use_resource_tracking")]
                resource_download_tracking.clone(),
            )),
            #[cfg(feature = "use_resource_tracking")]
            resource_download_tracking,
            content_folder,
            godot_single_thread: Arc::new(Semaphore::new(1)),
            texture_quality: DclConfig::static_get_texture_quality(),
            loading_hashes: HashSet::new(),
        }
    }

    fn ready(&mut self) {
        tracing::info!("ContentProvider2 ready");
    }

    fn exit_tree(&mut self) {
        self.loading_hashes.clear();
        tracing::info!("ContentProvider2::exit_tree");
    }
}

#[godot_api]
impl ContentProvider2 {
    /// Signal emitted when a GLTF is ready to be loaded from disk
    ///
    /// Parameters:
    /// - file_hash: The hash of the GLTF file
    /// - scene_path: The path to the saved .tscn file (user://content/glbs/<hash>.tscn)
    #[signal]
    fn gltf_ready(file_hash: GString, scene_path: GString);

    /// Signal emitted when a GLTF fails to load
    ///
    /// Parameters:
    /// - file_hash: The hash of the GLTF file
    /// - error: Error message
    #[signal]
    fn gltf_error(file_hash: GString, error: GString);

    /// Request to load a scene GLTF
    ///
    /// This will either:
    /// - Emit gltf_ready immediately if the scene is already cached
    /// - Start async loading and emit gltf_ready/gltf_error when done
    ///
    /// Note: Colliders are created with mask=0. Caller should set masks after instantiating.
    ///
    /// Returns true if the load was started, false if already loading
    #[func]
    pub fn load_scene_gltf(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> bool {
        let file_path_str = file_path.to_string().to_lowercase();
        let content_mapping_ref = content_mapping.bind().get_content_mapping();

        // Resolve file path to hash
        let file_hash = match content_mapping_ref.get_hash(&file_path_str) {
            Some(hash) => hash.clone(),
            None => {
                // Emit error signal
                self.base_mut().emit_signal(
                    "gltf_error".into(),
                    &[
                        GString::from("").to_variant(),
                        GString::from(format!(
                            "File not found in content mapping: {}",
                            file_path_str
                        ))
                        .to_variant(),
                    ],
                );
                return false;
            }
        };

        // Check if already loading this hash - caller should wait for signal
        if self.loading_hashes.contains(&file_hash) {
            // Return true because the load is in progress and signal will be emitted
            // The caller should wait for gltf_ready/gltf_error signal
            return true;
        }

        // Mark as loading
        self.loading_hashes.insert(file_hash.clone());

        // Create context for async operation
        let ctx = ContentProvider2Context {
            content_folder: self.content_folder.clone(),
            resource_provider: self.resource_provider.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
            texture_quality: self.texture_quality.clone(),
        };

        // Get instance ID for callback
        let instance_id = self.base().instance_id();
        let file_hash_clone = file_hash.clone();

        // Spawn async task - cache check and loading all happens here
        TokioRuntime::spawn(async move {
            // Check if scene is already cached on disk
            let scene_path = get_scene_path_for_hash(&ctx.content_folder, &file_hash_clone);

            let result = if ctx.resource_provider.file_exists_by_path(&scene_path).await {
                // Cache HIT - just touch and return the path
                tracing::debug!("GLTF cache HIT: {} -> {}", file_hash_clone, scene_path);
                ctx.resource_provider.touch_file_async(&scene_path).await;
                Ok(scene_path)
            } else {
                // Cache MISS - load, process, and save
                tracing::debug!("GLTF cache MISS: {} - loading", file_hash_clone);
                load_and_save_gltf(
                    file_path_str,
                    file_hash_clone.clone(),
                    content_mapping_ref,
                    ctx,
                )
                .await
            };

            // Callback to main thread
            let file_hash_gd = GString::from(&file_hash_clone);
            match result {
                Ok(scene_path) => {
                    let scene_path_gd = GString::from(&scene_path);
                    if let Ok(mut provider) =
                        Gd::<ContentProvider2>::try_from_instance_id(instance_id)
                    {
                        provider.call_deferred(
                            "on_gltf_load_complete".into(),
                            &[
                                file_hash_gd.to_variant(),
                                scene_path_gd.to_variant(),
                                GString::from("").to_variant(),
                            ],
                        );
                    }
                }
                Err(e) => {
                    let error_msg = GString::from(e.to_string());
                    if let Ok(mut provider) =
                        Gd::<ContentProvider2>::try_from_instance_id(instance_id)
                    {
                        provider.call_deferred(
                            "on_gltf_load_complete".into(),
                            &[
                                file_hash_gd.to_variant(),
                                GString::from("").to_variant(),
                                error_msg.to_variant(),
                            ],
                        );
                    }
                }
            }
        });

        true
    }

    /// Internal callback when GLTF load completes (called via call_deferred)
    #[func]
    fn on_gltf_load_complete(&mut self, file_hash: GString, scene_path: GString, error: GString) {
        let hash_str = file_hash.to_string();
        self.loading_hashes.remove(&hash_str);

        if error.is_empty() {
            self.base_mut().emit_signal(
                "gltf_ready".into(),
                &[file_hash.to_variant(), scene_path.to_variant()],
            );
        } else {
            self.base_mut().emit_signal(
                "gltf_error".into(),
                &[file_hash.to_variant(), error.to_variant()],
            );
        }
    }

    /// Helper to emit gltf_ready signal (called via call_deferred)
    #[func]
    fn emit_gltf_ready(&mut self, file_hash: GString, scene_path: GString) {
        self.base_mut().emit_signal(
            "gltf_ready".into(),
            &[file_hash.to_variant(), scene_path.to_variant()],
        );
    }

    /// Check if a scene is cached on disk
    #[func]
    pub fn is_scene_cached(&self, file_hash: GString) -> bool {
        let scene_path = get_scene_path_for_hash(&self.content_folder, &file_hash.to_string());
        std::path::Path::new(&scene_path).exists()
    }

    /// Get the path where a scene would be cached
    #[func]
    pub fn get_scene_cache_path(&self, file_hash: GString) -> GString {
        get_scene_path_for_hash(&self.content_folder, &file_hash.to_string()).into()
    }

    /// Check if a hash is currently being loaded
    #[func]
    pub fn is_loading(&self, file_hash: GString) -> bool {
        self.loading_hashes.contains(&file_hash.to_string())
    }

    /// Set the shared resource provider (to share with ContentProvider)
    #[func]
    pub fn set_resource_provider(
        &mut self,
        content_provider: Gd<crate::content::content_provider::ContentProvider>,
    ) {
        // Get the resource provider from the existing ContentProvider
        // This allows sharing the same download cache
        let provider_ref = content_provider.bind().get_resource_provider();
        self.resource_provider = provider_ref;
    }
}

impl ContentProvider2 {
    /// Get the resource provider (for sharing with other systems)
    pub fn get_resource_provider(&self) -> Arc<ResourceProvider> {
        self.resource_provider.clone()
    }
}
