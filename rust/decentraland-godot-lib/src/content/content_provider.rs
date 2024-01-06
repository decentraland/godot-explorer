use std::{collections::HashMap, sync::Arc};

use godot::{
    engine::{AudioStream, Image, ImageTexture},
    prelude::*,
};

use crate::{
    content::content_mapping::DclContentMappingAndUrl, godot_classes::promise::Promise,
    http_request::http_queue_requester::HttpQueueRequester,
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::{
    content_notificator::ContentNotificator,
    gltf::{apply_update_set_mask_colliders, load_gltf},
};
pub enum ContentEntryData {
    Texture(Gd<ImageTexture>),
    Gltf(Gd<Node3D>),
    WearableEmote(Dictionary),
    Audio(Gd<AudioStream>),
    Video(()),
}

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
        if let Some(entry) = self.cached.get(&file_path.to_string()) {
            return entry.promise.clone();
        }

        let (promise, get_promise) = Promise::make_to_async();
        let content_mapping = content_mapping.bind().get_content_mapping();
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
            file_path.to_string(),
            ContentEntry {
                promise: promise.clone(),
            },
        );

        promise
    }

    #[func]
    pub fn duplicate_materials(&mut self, _target_meshes: VariantArray) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
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
    pub fn fetch_wearables(
        &mut self,
        _wearables: VariantArray,
        _content_base_url: GString,
    ) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn fetch_texture(
        &mut self,
        _file_path: GString,
        _content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn fetch_texture_by_hash(
        &mut self,
        _file_hash: GString,
        _content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn fetch_texture_by_url(&mut self, _file_hash: GString, _url: GString) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
    }

    #[func]
    pub fn get_image_from_texture_or_nil(
        &mut self,
        _file_path: GString,
        _content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Image> {
        Image::new()
    }

    #[func]
    pub fn get_image_from_texture_by_hash_or_nil(&mut self, _file_hash: GString) -> Gd<Image> {
        Image::new()
    }

    #[func]
    pub fn fetch_audio(
        &mut self,
        _file_path: GString,
        _content_mapping: Gd<DclContentMappingAndUrl>,
    ) -> Gd<Promise> {
        Promise::from_resolved(Variant::nil())
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
    pub fn get_wearable(&mut self, _id: GString) -> Variant {
        Variant::nil()
    }

    #[func]
    pub fn get_texture_from_hash(&self, _file_hash: GString) -> Option<Gd<ImageTexture>> {
        None
    }

    #[func]
    pub fn get_gltf_from_hash(&self, _file_hash: GString) -> Option<Gd<Node3D>> {
        None
    }

    #[func]
    pub fn is_resource_from_hash_loaded(&self, _file_hash: GString) -> bool {
        true
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
