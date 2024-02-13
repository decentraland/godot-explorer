use std::sync::Arc;

use godot::prelude::*;

use super::scene_definition::SceneEntityDefinition;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclSceneEntityDefinition {
    inner: Arc<SceneEntityDefinition>,
}

#[godot_api]
impl IRefCounted for DclSceneEntityDefinition {
    fn init(_base: Base<RefCounted>) -> Self {
        DclSceneEntityDefinition {
            inner: Arc::new(SceneEntityDefinition::default()),
        }
    }
}

#[godot_api]
impl DclSceneEntityDefinition {
    #[func]
    fn get_parcels(&self) -> godot::prelude::Array<Vector2i> {
        let mut ret = godot::prelude::Array::new();
        let parcels_str = &self.inner.scene_meta_scene.scene.parcels;
        ret.resize(parcels_str.len());
        parcels_str.iter().for_each(|parcel| ret.push(*parcel));

        ret
    }

    #[func]
    fn get_base_parcel(&self) -> Vector2i {
        self.inner.scene_meta_scene.scene.base
    }

    #[func]
    fn get_title(&self) -> GString {
        let Some(scene_display) = self
            .inner
            .scene_meta_scene
            .display
            .as_ref()
            .and_then(|d| d.title.as_ref())
        else {
            return GString::from("No title");
        };
        scene_display.to_string().into()
    }

    #[func]
    fn is_global(&self) -> bool {
        self.inner.is_global
    }

    #[func]
    fn is_sdk7(&self) -> bool {
        if let Some(runtime_version) = self.inner.scene_meta_scene.runtime_version.as_ref() {
            runtime_version == "7"
        } else {
            false
        }
    }

    #[func]
    fn get_main_js_hash(&self) -> GString {
        self.inner
            .content_mapping
            .content
            .get(&self.inner.scene_meta_scene.main)
            .unwrap_or(&"".to_string())
            .to_string()
            .into()
    }

    #[func]
    fn get_main_crdt_hash(&self) -> GString {
        self.inner
            .content_mapping
            .content
            .get("main.crdt")
            .unwrap_or(&"".to_string())
            .to_string()
            .into()
    }

    #[func]
    fn get_base_url(&self) -> GString {
        self.inner.content_mapping.base_url.to_string().into()
    }

    #[func]
    fn get_global_spawn_position(&self) -> Vector3 {
        self.inner.get_global_spawn_position()
    }
}

impl DclSceneEntityDefinition {
    pub fn from_ref(ref_: &Arc<SceneEntityDefinition>) -> Gd<DclSceneEntityDefinition> {
        Gd::from_init_fn(move |_base| DclSceneEntityDefinition {
            inner: ref_.clone(),
        })
    }

    pub fn get_ref(&self) -> Arc<SceneEntityDefinition> {
        self.inner.clone()
    }
}
