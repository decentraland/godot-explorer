use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use godot::{
    engine::{AudioStream, ImageTexture, Material, Mesh},
    prelude::*,
};
use tokio::sync::Semaphore;

use crate::{
    auth::wallet::AsH160,
    avatars::{dcl_user_profile::DclUserProfile, item::DclItemEntityDefinition},
    content::content_mapping::DclContentMappingAndUrl,
    dcl::common::string::FindNthChar,
    godot_classes::promise::Promise,
    http_request::http_queue_requester::HttpQueueRequester,
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::{
    audio::load_audio,
    content_notificator::ContentNotificator,
    gltf::{
        apply_update_set_mask_colliders, load_gltf_emote, load_gltf_scene_content,
        load_gltf_wearable, DclEmoteGltf,
    },
    profile::{prepare_request_requirements, request_lambda_profile},
    texture::{load_image_texture, TextureEntry},
    thread_safety::{set_thread_safety_checks_enabled, then_promise, GodotSingleThreadSafety},
    video::download_video,
    wearable_entities::{request_wearables, WearableManyResolved},
};
pub struct ContentEntry {
    promise: Gd<Promise>,
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct ContentProvider {
    content_folder: Arc<String>,
    http_queue_requester: Arc<HttpQueueRequester>,
    content_notificator: Arc<ContentNotificator>,
    cached: HashMap<String, ContentEntry>,
    godot_single_thread: Arc<Semaphore>,
}

#[derive(Clone)]
pub struct ContentProviderContext {
    pub content_folder: Arc<String>,
    pub http_queue_requester: Arc<HttpQueueRequester>,
    pub content_notificator: Arc<ContentNotificator>,
    pub godot_single_thread: Arc<Semaphore>,
}

unsafe impl Send for ContentProviderContext {}

#[godot_api]
impl INode for ContentProvider {
    fn init(_base: Base<Node>) -> Self {
        let content_folder = Arc::new(format!(
            "{}/content/",
            godot::engine::Os::singleton().get_user_data_dir()
        ));
        Self {
            content_folder,
            http_queue_requester: Arc::new(HttpQueueRequester::new(6)),
            cached: HashMap::new(),
            content_notificator: Arc::new(ContentNotificator::new()),
            godot_single_thread: Arc::new(Semaphore::new(1)),
        }
    }
    fn ready(&mut self) {}
    fn exit_tree(&mut self) {
        self.cached.clear();
        tracing::info!("ContentProvider::exit_tree");
    }
}

#[godot_api]
impl ContentProvider {
    // content_type 1: wearable, 2: emote, default: scene
    #[func]
    pub fn fetch_gltf(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
        content_type: i32,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping.get_hash(file_path.to_string().as_str()) else {
            return Promise::from_rejected(format!("File not found: {}", file_path));
        };

        if let Some(entry) = self.cached.get(file_hash) {
            return entry.promise.clone();
        }

        let file_hash = file_hash.clone();
        let (promise, get_promise) = Promise::make_to_async();
        let gltf_file_path = file_path.to_string();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            let result = match content_type {
                1 => {
                    load_gltf_wearable(gltf_file_path, content_mapping, content_provider_context)
                        .await
                }
                2 => {
                    load_gltf_emote(gltf_file_path, content_mapping, content_provider_context).await
                }
                _ => {
                    load_gltf_scene_content(
                        gltf_file_path,
                        content_mapping,
                        content_provider_context,
                    )
                    .await
                }
            };

            then_promise(get_promise, result);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn instance_gltf_colliders(
        &mut self,
        gltf_node: Gd<Node>,
        dcl_visible_cmask: i32,
        dcl_invisible_cmask: i32,
        dcl_scene_id: i32,
        dcl_entity_id: i32,
    ) -> Gd<Promise> {
        let (promise, get_promise) = Promise::make_to_async();
        let gltf_node_instance_id = gltf_node.instance_id();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            let result = apply_update_set_mask_colliders(
                gltf_node_instance_id,
                dcl_visible_cmask,
                dcl_invisible_cmask,
                dcl_scene_id,
                dcl_entity_id,
                content_provider_context,
            )
            .await;
            then_promise(get_promise, result);
        });

        promise
    }

    #[func]
    pub fn fetch_audio(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping.get_hash(file_path.to_string().as_str()) else {
            return Promise::from_rejected(format!("File not found: {}", file_path));
        };

        if let Some(entry) = self.cached.get(file_hash) {
            return entry.promise.clone();
        }

        let file_hash = file_hash.clone();
        let (promise, get_promise) = Promise::make_to_async();
        let audio_file_path = file_path.to_string();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            let result =
                load_audio(audio_file_path, content_mapping, content_provider_context).await;
            then_promise(get_promise, result);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn fetch_texture(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = content_mapping.bind().get_hash(file_path);
        if file_hash.is_empty() {
            return Promise::from_rejected("Texture not found in the mappings.".to_string());
        };

        self.fetch_texture_by_hash(file_hash, content_mapping)
    }

    #[func]
    pub fn fetch_texture_by_hash(
        &mut self,
        file_hash_godot: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = file_hash_godot.to_string();
        if let Some(entry) = self.cached.get(&file_hash) {
            return entry.promise.clone();
        }

        // TODO: In the future, this would be handled by each component handler
        //  and check if the hostname is allowed (set up in the scene.json)
        //  https://github.com/decentraland/godot-explorer/issues/363
        if file_hash.starts_with("http") {
            // get file_hash from url
            let new_file_hash = format!("hashed_{:x}", file_hash_godot.hash());
            let promise = self.fetch_texture_by_url(GString::from(new_file_hash), file_hash_godot);
            self.cached.insert(
                file_hash,
                ContentEntry {
                    promise: promise.clone(),
                },
            );
            return promise;
        }

        let url = format!(
            "{}{}",
            content_mapping.bind().get_base_url(),
            file_hash.clone()
        );
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();
        let sent_file_hash = file_hash.clone();
        TokioRuntime::spawn(async move {
            let result = load_image_texture(url, sent_file_hash, content_provider_context).await;
            then_promise(get_promise, result);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn fetch_texture_by_url(&mut self, file_hash: GString, url: GString) -> Gd<Promise> {
        let file_hash = file_hash.to_string();
        if let Some(entry) = self.cached.get(&file_hash) {
            return entry.promise.clone();
        }
        let url = url.to_string();
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();
        let sent_file_hash = file_hash.clone();
        TokioRuntime::spawn(async move {
            let result = load_image_texture(url, sent_file_hash, content_provider_context).await;
            then_promise(get_promise, result);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn get_texture_from_hash(&self, file_hash: GString) -> Option<Gd<ImageTexture>> {
        let promise_data = self
            .cached
            .get(&file_hash.to_string())?
            .promise
            .bind()
            .get_data();
        let texture_entry = promise_data.try_to::<Gd<TextureEntry>>().ok()?;
        let texture = texture_entry.bind().texture.clone();
        Some(texture)
    }

    #[func]
    pub fn get_gltf_from_hash(&self, file_hash: GString) -> Option<Gd<Node3D>> {
        self.cached
            .get(&file_hash.to_string())?
            .promise
            .bind()
            .get_data()
            .try_to::<Gd<Node3D>>()
            .ok()
    }

    #[func]
    pub fn get_emote_gltf_from_hash(&self, file_hash: GString) -> Option<Gd<DclEmoteGltf>> {
        self.cached
            .get(&file_hash.to_string())?
            .promise
            .bind()
            .get_data()
            .try_to::<Gd<DclEmoteGltf>>()
            .ok()
    }

    #[func]
    pub fn get_audio_from_hash(&self, file_hash: GString) -> Option<Gd<AudioStream>> {
        self.cached
            .get(&file_hash.to_string())?
            .promise
            .bind()
            .get_data()
            .try_to::<Gd<AudioStream>>()
            .ok()
    }

    #[func]
    pub fn is_resource_from_hash_loaded(&self, file_hash: GString) -> bool {
        if let Some(entry) = self.cached.get(&file_hash.to_string()) {
            return entry.promise.bind().is_resolved();
        }
        false
    }

    #[func]
    pub fn fetch_video(
        &mut self,
        file_hash: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let (promise, get_promise) = Promise::make_to_async();
        let file_hash = file_hash.to_string();
        let video_file_hash = file_hash.clone();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            let result =
                download_video(video_file_hash, content_mapping, content_provider_context).await;
            then_promise(get_promise, result);
        });

        self.cached.insert(
            file_hash,
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn duplicate_materials(&mut self, target_meshes: VariantArray) -> Gd<Promise> {
        let data = target_meshes
            .iter_shared()
            .map(|dict| {
                let dict = dict.try_to::<Dictionary>().ok()?;
                let mesh = dict.get("mesh")?.try_to::<Gd<Mesh>>().ok()?;
                let n = dict.get("n")?.try_to::<i32>().ok()?;

                Some((mesh.instance_id(), n))
            })
            .filter(|v| v.is_some())
            .flatten()
            .collect::<Vec<_>>();

        let (promise, get_promise) = Promise::make_to_async();
        TokioRuntime::spawn(async move {
            set_thread_safety_checks_enabled(false);

            for (mesh_instance_id, n) in data {
                let mut mesh = Gd::<Mesh>::from_instance_id(mesh_instance_id);
                for i in 0..n {
                    let Some(new_material) = mesh.surface_get_material(i) else {
                        continue;
                    };
                    let Some(new_material) = new_material.duplicate() else {
                        continue;
                    };

                    mesh.surface_set_material(i, new_material.cast::<Material>());
                }
            }

            set_thread_safety_checks_enabled(true);

            then_promise(get_promise, Ok(None));
        });

        promise
    }

    #[func]
    pub fn fetch_wearables(
        &mut self,
        wearables: VariantArray,
        content_base_url: GString,
    ) -> Array<Gd<Promise>> {
        let mut promise_ids = HashSet::new();
        let mut new_promise = None;
        let mut wearable_to_fetch = HashSet::new();

        for wearable in wearables.iter_shared() {
            let wearable_id = wearable.to_string();
            let token_id_pos = wearable_id
                .find_nth_char(6, ':')
                .unwrap_or(wearable_id.len());
            let wearable_id = wearable_id[0..token_id_pos].to_lowercase();

            if let Some(entry) = self.cached.get(&wearable_id) {
                promise_ids.insert(entry.promise.instance_id());
            } else {
                wearable_to_fetch.insert(wearable_id.clone());
                if new_promise.is_none() {
                    let (promise, get_promise) = Promise::make_to_async();
                    promise_ids.insert(promise.instance_id());
                    new_promise = Some((promise, get_promise));
                }

                self.cached.insert(
                    wearable_id,
                    ContentEntry {
                        promise: new_promise.as_ref().unwrap().0.clone(),
                    },
                );
            }
        }

        if !wearable_to_fetch.is_empty() {
            let (promise, get_promise) = new_promise.unwrap();
            let content_provider_context = self.get_context();
            let content_base_url = content_base_url.to_string();
            let extra_slash = if content_base_url.ends_with('/') {
                ""
            } else {
                "/"
            };
            let content_base_url = format!("{}{extra_slash}", content_base_url);
            let ipfs_content_base_url = format!("{content_base_url}contents/");
            TokioRuntime::spawn(async move {
                let result = request_wearables(
                    content_base_url,
                    ipfs_content_base_url,
                    wearable_to_fetch.into_iter().collect(),
                    content_provider_context,
                )
                .await;
                then_promise(get_promise, result);
            });
            self.cached.insert(
                "wearables".to_string(),
                ContentEntry {
                    promise: promise.clone(),
                },
            );
        }

        Array::from_iter(promise_ids.into_iter().map(Gd::from_instance_id))
    }

    #[func]
    pub fn get_wearable(&mut self, id: GString) -> Option<Gd<DclItemEntityDefinition>> {
        let id = id.to_string();
        let token_id_pos = id.find_nth_char(6, ':').unwrap_or(id.len());
        let id = id[0..token_id_pos].to_lowercase();

        if let Some(entry) = self.cached.get(&id) {
            if let Ok(results) = entry
                .promise
                .bind()
                .get_data()
                .try_to::<Gd<WearableManyResolved>>()
            {
                if let Some(wearable) = results.bind().wearable_map.get(&id) {
                    return Some(DclItemEntityDefinition::from_gd(wearable.clone()));
                }
            }
        }
        None
    }

    #[func]
    pub fn get_pending_promises(&self) -> Array<Gd<Promise>> {
        Array::from_iter(
            self.cached
                .iter()
                .filter(|(_, entry)| !entry.promise.bind().is_resolved())
                .map(|(_, entry)| entry.promise.clone()),
        )
    }

    #[func]
    pub fn get_profile(&self, user_id: GString) -> Option<Gd<DclUserProfile>> {
        let user_id = user_id.to_string().as_str().as_h160()?;
        let hash = format!("profile_{:x}", user_id);
        let promise_data = self.cached.get(&hash)?.promise.bind().get_data();
        promise_data.try_to::<Gd<DclUserProfile>>().ok()
    }

    #[func]
    pub fn fetch_profile(&mut self, user_id: GString) -> Gd<Promise> {
        let Some(user_id) = user_id.to_string().as_str().as_h160() else {
            return Promise::from_rejected("Invalid user id".to_string());
        };

        let hash = format!("profile_{:x}", user_id);
        if let Some(entry) = self.cached.get(&hash) {
            return entry.promise.clone();
        }

        let (lamda_server_base_url, profile_base_url, http_requester) =
            prepare_request_requirements();
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();

        TokioRuntime::spawn(async move {
            let result = request_lambda_profile(
                user_id,
                lamda_server_base_url.as_str(),
                profile_base_url.as_str(),
                http_requester,
            )
            .await;

            let Some(_thread_safe_check) =
                GodotSingleThreadSafety::acquire_owned(&content_provider_context).await
            else {
                tracing::error!("Failed to acquire semaphore");
                return;
            };
            let result = result.map(|value| Some(DclUserProfile::from_gd(value).to_variant()));

            then_promise(get_promise, result)
        });

        self.cached.insert(
            hash,
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }
}

impl ContentProvider {
    fn get_context(&self) -> ContentProviderContext {
        ContentProviderContext {
            content_folder: self.content_folder.clone(),
            http_queue_requester: self.http_queue_requester.clone(),
            content_notificator: self.content_notificator.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
        }
    }
}
