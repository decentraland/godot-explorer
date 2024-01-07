use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use godot::{
    engine::{ImageTexture, Material, Mesh},
    prelude::*,
};

use crate::{
    content::content_mapping::DclContentMappingAndUrl, godot_classes::promise::Promise,
    http_request::http_queue_requester::HttpQueueRequester,
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::{
    audio::load_audio,
    content_notificator::ContentNotificator,
    gltf::{apply_update_set_mask_colliders, load_gltf},
    texture::load_png_texture,
    thread_safety::{resolve_promise, set_thread_safety_checks_enabled},
    video::download_video,
    wearable_entities::request_wearables,
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
}

#[derive(Clone)]
pub struct ContentProviderContext {
    pub content_folder: Arc<String>,
    pub http_queue_requester: Arc<HttpQueueRequester>,
    pub content_notificator: Arc<ContentNotificator>,
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
            http_queue_requester: Arc::new(HttpQueueRequester::new(4)),
            cached: HashMap::new(),
            content_notificator: Arc::new(ContentNotificator::new()),
        }
    }
    fn ready(&mut self) {}
}

#[godot_api]
impl ContentProvider {
    #[func]
    pub fn fetch_gltf(
        &mut self,
        file_path: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let content_mapping = content_mapping.bind().get_content_mapping();
        let Some(file_hash) = content_mapping
            .content
            .get(&file_path.to_string().to_lowercase())
        else {
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
            load_gltf(
                gltf_file_path,
                content_mapping,
                get_promise,
                content_provider_context,
            )
            .await;
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
        TokioRuntime::spawn(async move {
            apply_update_set_mask_colliders(
                gltf_node_instance_id,
                dcl_visible_cmask,
                dcl_invisible_cmask,
                dcl_scene_id,
                dcl_entity_id,
                get_promise,
            );
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
        let Some(file_hash) = content_mapping
            .content
            .get(&file_path.to_string().to_lowercase())
        else {
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
            load_audio(
                audio_file_path,
                content_mapping,
                get_promise,
                content_provider_context,
            )
            .await;
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
        file_hash: GString,
        content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        let file_hash = file_hash.to_string();
        if let Some(entry) = self.cached.get(&file_hash) {
            return entry.promise.clone();
        }

        let absolute_file_path = format!("{}{}", self.content_folder, file_hash);
        let url = format!("{}{}", content_mapping.bind().get_base_url(), file_hash);
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            load_png_texture(
                url,
                absolute_file_path,
                get_promise,
                content_provider_context,
            )
            .await;
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
        let absolute_file_path = format!("{}{}", self.content_folder, file_hash);
        let (promise, get_promise) = Promise::make_to_async();
        let content_provider_context = self.get_context();
        TokioRuntime::spawn(async move {
            load_png_texture(
                url,
                absolute_file_path,
                get_promise,
                content_provider_context,
            )
            .await;
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
        self.cached
            .get(&file_hash.to_string())?
            .promise
            .bind()
            .get_data()
            .try_to::<Dictionary>()
            .ok()?
            .get("texture")?
            .try_to::<Gd<ImageTexture>>()
            .ok()
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
            download_video(
                video_file_hash,
                content_mapping,
                get_promise,
                content_provider_context,
            )
            .await;
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
    pub fn duplicate_materials(&mut self, target_meshes: Array<Dictionary>) -> Gd<Promise> {
        let data = target_meshes
            .iter_shared()
            .map(|dict| {
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

            resolve_promise(get_promise, None);
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
            let wearable_id = wearable.to_string().to_lowercase();
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
                request_wearables(
                    content_base_url,
                    ipfs_content_base_url,
                    wearable_to_fetch.into_iter().collect(),
                    get_promise,
                    content_provider_context,
                )
                .await;
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
    pub fn get_wearable(&mut self, id: GString) -> Variant {
        let id = id.to_string().to_lowercase();
        if let Some(entry) = self.cached.get(&id) {
            if let Ok(results) = entry.promise.bind().get_data().try_to::<Dictionary>() {
                if let Some(wearable) = results.get(id) {
                    return wearable;
                }
            }
        }
        Variant::nil()
    }
}

impl ContentProvider {
    fn get_context(&self) -> ContentProviderContext {
        ContentProviderContext {
            content_folder: self.content_folder.clone(),
            http_queue_requester: self.http_queue_requester.clone(),
            content_notificator: self.content_notificator.clone(),
        }
    }
}
