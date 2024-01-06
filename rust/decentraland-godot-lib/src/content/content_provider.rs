use std::{collections::HashMap, sync::Arc};

use godot::{engine::ImageTexture, prelude::*};

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
        _file_hash: GString,
        _content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn duplicate_materials(&mut self, _target_meshes: VariantArray) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn fetch_wearables(
        &mut self,
        _wearables: VariantArray,
        _content_base_url: GString,
    ) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn get_wearable(&mut self, _id: GString) -> Variant {
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
