//! Promise-based loading of wearable and emote scenes through the content provider.
//! Loaded scene paths are `res://` for optimized assets and `user://` for runtime-processed ones.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use godot::classes::resource_loader::ThreadLoadStatus;
use godot::classes::{FileAccess, PackedScene, ResourceLoader};
use godot::global::Error;
use godot::prelude::*;

use crate::avatars::item::DclItemEntityDefinition;
use crate::avatars::wearables;
use crate::content::content_mapping::DclContentMappingAndUrl;
use crate::content::gltf::emote::DclEmoteGltf;
use crate::engine::local_task::{await_all, await_promise, next_frame};
use crate::fetcher::globals;
use crate::godot_classes::promise::Promise;

/// Threaded `PackedScene` load, polled each frame so the main thread never blocks.
async fn load_packed_scene(who: &str, scene_path: &str) -> Option<Gd<PackedScene>> {
    let exists = if scene_path.starts_with("res://") {
        ResourceLoader::singleton().exists(scene_path)
    } else {
        FileAccess::file_exists(scene_path)
    };
    if !exists {
        godot_error!("{who}: scene not found: {scene_path}");
        return None;
    }
    if ResourceLoader::singleton().load_threaded_request(scene_path) != Error::OK {
        godot_error!("{who}: failed to request threaded load for {scene_path}");
        return None;
    }
    let mut status = ResourceLoader::singleton().load_threaded_get_status(scene_path);
    while status == ThreadLoadStatus::IN_PROGRESS {
        next_frame().await;
        status = ResourceLoader::singleton().load_threaded_get_status(scene_path);
    }
    if status != ThreadLoadStatus::LOADED {
        godot_error!("{who}: threaded load of {scene_path} ended as {status:?}");
        return None;
    }
    let loaded = ResourceLoader::singleton().load_threaded_get(scene_path);
    match loaded.and_then(|resource| resource.try_cast::<PackedScene>().ok()) {
        Some(scene) => Some(scene),
        None => {
            godot_error!("{who}: {scene_path} is not a PackedScene");
            None
        }
    }
}

fn path_of(promise: &Gd<Promise>) -> Option<String> {
    let promise = promise.bind();
    if !promise.is_resolved() || promise.is_rejected() {
        return None;
    }
    promise
        .get_data()
        .try_to::<GString>()
        .ok()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

/// `file_hash -> scene_path` of what the last load finished.
type Completed = Rc<RefCell<HashMap<String, String>>>;

#[derive(Clone, Default)]
pub struct WearableLoader {
    completed: Completed,
}

impl WearableLoader {
    /// Starts every wearable's download and waits for them; the scenes are read back with
    /// `get_wearable_node`.
    pub async fn load_wearables(&self, wearable_ids: &[String], body_shape_id: &str) {
        self.completed.borrow_mut().clear();
        let mut gltf_promises: Vec<Gd<Promise>> = Vec::new();
        let mut gltf_hashes: Vec<String> = Vec::new();
        let mut texture_promises: Vec<Gd<Promise>> = Vec::new();
        let mut provider = globals::content_provider();

        for id in wearable_ids {
            let Some(item) = provider.bind_mut().get_wearable(id.as_str().into()) else {
                godot_error!("WearableLoader: wearable {id} is null");
                continue;
            };
            if !DclItemEntityDefinition::is_valid_wearable(
                Some(item.clone()),
                body_shape_id.to_string(),
                true,
            ) {
                continue;
            }
            let category = item.bind().get_category().to_string();
            let mapping = item.bind().get_content_mapping();

            if wearables::is_texture(&category) {
                let hashes = wearables::get_wearable_facial_hashes(Some(&item), body_shape_id);
                let files = mapping.bind().get_files();
                for file in files.as_slice() {
                    let hash = mapping.bind().get_hash(file.clone()).to_string();
                    if hashes.contains(&hash) {
                        let promise = provider
                            .bind_mut()
                            .fetch_texture(file.clone(), mapping.clone());
                        texture_promises.push(promise);
                    }
                }
                continue;
            }

            let hash = wearables::get_item_main_file_hash(&item, body_shape_id);
            if hash.is_empty() {
                godot_error!("WearableLoader: empty file_hash for {id}");
                continue;
            }
            let main_file = item
                .bind()
                .get_representation_main_file(body_shape_id.to_string());
            if let Some(promise) = provider.bind_mut().load_wearable_gltf(main_file, mapping) {
                gltf_promises.push(promise);
                gltf_hashes.push(hash);
            }
        }

        await_all(gltf_promises.clone()).await;
        for (promise, hash) in gltf_promises.iter().zip(gltf_hashes) {
            if let Some(path) = path_of(promise) {
                self.completed.borrow_mut().insert(hash, path);
            }
        }
        await_all(texture_promises).await;
    }

    /// A fresh instance of the loaded wearable scene.
    pub async fn get_wearable_node(&self, file_hash: &str) -> Option<Gd<Node3D>> {
        let known = self.completed.borrow().get(file_hash).cloned();
        let path = match known {
            Some(path) => path,
            None => globals::content_provider()
                .bind()
                .get_wearable_cache_path(file_hash.into())
                .to_string(),
        };
        if path.is_empty() {
            godot_error!("WearableLoader: no scene_path for hash {file_hash}");
            return None;
        }
        let scene = load_packed_scene("WearableLoader", &path).await?;
        scene
            .instantiate()
            .and_then(|n| n.try_cast::<Node3D>().ok())
    }
}

#[derive(Clone, Default)]
pub struct EmoteLoader {
    completed: Completed,
}

fn first_audio_file(mapping: &Gd<DclContentMappingAndUrl>) -> Option<GString> {
    mapping
        .bind()
        .get_files()
        .as_slice()
        .iter()
        .find(|f| {
            let name = f.to_string();
            name.ends_with(".mp3") || name.ends_with(".ogg")
        })
        .cloned()
}

impl EmoteLoader {
    /// Loads one emote of an entity; the scene path, empty when it failed.
    pub async fn load_emote(&self, emote_urn: &str, body_shape_id: &str) -> String {
        let mut provider = globals::content_provider();
        let Some(emote) = provider.bind_mut().get_wearable(emote_urn.into()) else {
            godot_error!("EmoteLoader: emote {emote_urn} is null");
            return String::new();
        };
        let file_name = emote
            .bind()
            .get_representation_main_file(body_shape_id.to_string());
        if file_name.is_empty() {
            godot_error!(
                "EmoteLoader: no representation for {emote_urn} with body shape {body_shape_id}"
            );
            return String::new();
        }
        let mapping = emote.bind().get_content_mapping();
        self.load_from_mapping(emote_urn, &file_name.to_string(), &mapping, false)
            .await
    }

    /// Loads an emote from a content mapping (scene emotes pass `runtime_only` to skip the
    /// optimized assets); the scene path, empty when it failed.
    pub async fn load_from_mapping(
        &self,
        emote_key: &str,
        file_name: &str,
        mapping: &Gd<DclContentMappingAndUrl>,
        runtime_only: bool,
    ) -> String {
        let hash = mapping.bind().get_hash(file_name.into()).to_string();
        if hash.is_empty() {
            godot_error!("EmoteLoader: empty file_hash for {emote_key} file {file_name}");
            return String::new();
        }
        let mut provider = globals::content_provider();
        let audio = first_audio_file(mapping)
            .map(|file| provider.bind_mut().fetch_audio(file, mapping.clone()));
        let gltf = if runtime_only {
            provider
                .bind_mut()
                .load_emote_gltf_runtime_only(file_name.into(), mapping.clone())
        } else {
            provider
                .bind_mut()
                .load_emote_gltf(file_name.into(), mapping.clone())
        };
        let Some(gltf) = gltf else {
            godot_error!("EmoteLoader: failed to start loading emote {emote_key}");
            return String::new();
        };
        await_promise(gltf.clone()).await;
        let path = path_of(&gltf).unwrap_or_default();
        if !path.is_empty() {
            self.completed.borrow_mut().insert(hash, path.clone());
        }
        if let Some(audio) = audio {
            await_promise(audio).await;
        }
        path
    }

    /// The emote's animations out of its cached scene. With `file_name` and `mapping`, an
    /// optimized bake that kept the prop animation but lost the prop geometry is re-processed
    /// from the original GLB at runtime.
    pub async fn get_emote_gltf(
        &self,
        file_hash: &str,
        file_name: &str,
        mapping: Option<&Gd<DclContentMappingAndUrl>>,
    ) -> Option<Gd<DclEmoteGltf>> {
        let known = self.completed.borrow().get(file_hash).cloned();
        let path = match known {
            Some(path) => path,
            None => globals::content_provider()
                .bind()
                .get_emote_cache_path(file_hash.into())
                .to_string(),
        };
        if path.is_empty() {
            godot_error!("EmoteLoader: no scene_path for hash {file_hash}");
            return None;
        }
        let emote = self.extract(&path, file_hash).await;

        let lost_prop = emote.as_ref().is_some_and(|e| {
            let e = e.bind();
            e.get_prop_animation().is_some() && e.get_armature_prop().is_none()
        });
        if let (true, true, Some(mapping), false) = (
            lost_prop,
            path.starts_with("res://"),
            mapping,
            file_name.is_empty(),
        ) {
            godot_print!(
                "EmoteLoader: optimized bake for {file_hash} has a prop animation but no prop node, re-processing the GLB at runtime"
            );
            let runtime_path = self
                .load_from_mapping(file_hash, file_name, mapping, true)
                .await;
            if !runtime_path.is_empty() && !runtime_path.starts_with("res://") {
                if let Some(healed) = self.extract(&runtime_path, file_hash).await {
                    return Some(healed);
                }
            }
        }
        emote
    }

    async fn extract(&self, scene_path: &str, file_hash: &str) -> Option<Gd<DclEmoteGltf>> {
        let scene = load_packed_scene("EmoteLoader", scene_path).await?;
        globals::content_provider()
            .bind()
            .extract_emote_from_scene(scene, file_hash.into())
    }
}

/// Downloads the entity of an emote urn (the definition, not the glTF).
pub async fn fetch_emote_definition(emote_urn: &str) {
    let base_url = globals::realm().bind().get_profile_content_url();
    let promises = globals::content_provider()
        .bind_mut()
        .fetch_wearables(varray![emote_urn], base_url);
    await_all(promises.iter_shared().collect()).await;
}
