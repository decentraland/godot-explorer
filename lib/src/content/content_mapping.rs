use std::{collections::HashMap, sync::Arc};

use godot::prelude::*;

use crate::dcl::common::content_entity::TypedIpfsRef;

#[derive(Debug, Default)]
pub struct ContentMappingAndUrl {
    pub base_url: String,

    // This field is private because in the constructor
    //  all the `keys` are converted to lowercase
    // So the only way to access it is through the `get_hash` method
    //  which converts the input to lowercase
    content: HashMap<String, String>,
}

impl ContentMappingAndUrl {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_base_url_and_content(base_url: String, content: Vec<TypedIpfsRef>) -> Self {
        ContentMappingAndUrl {
            base_url,
            content: content
                .into_iter()
                .map(|v| (v.file.to_lowercase(), v.hash))
                .collect(),
        }
    }

    pub fn get_hash(&self, file: &str) -> Option<&String> {
        let file = file.to_lowercase();
        self.content.get(&file)
    }

    pub fn files(&self) -> &HashMap<String, String> {
        &self.content
    }
}

pub type ContentMappingAndUrlRef = Arc<ContentMappingAndUrl>;
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclContentMappingAndUrl {
    inner: ContentMappingAndUrlRef,
}

#[godot_api]
impl IRefCounted for DclContentMappingAndUrl {
    fn init(_base: Base<RefCounted>) -> Self {
        DclContentMappingAndUrl {
            inner: Arc::new(ContentMappingAndUrl {
                base_url: "".into(),
                content: HashMap::new(),
            }),
        }
    }
}

#[godot_api]
impl DclContentMappingAndUrl {
    #[func]
    fn from_values(base_url: GString, dict: Dictionary) -> Gd<DclContentMappingAndUrl> {
        let mut value = ContentMappingAndUrl::new();

        value.base_url = base_url.to_string();
        value.content = HashMap::from_iter(
            dict.iter_shared()
                .map(|(k, v)| (k.to_string().to_lowercase(), v.to_string())),
        );

        Gd::from_init_fn(|_base| DclContentMappingAndUrl {
            inner: Arc::new(value),
        })
    }

    #[func]
    pub fn get_base_url(&self) -> GString {
        self.inner.base_url.to_godot()
    }

    #[func]
    pub fn get_hash(&self, file: GString) -> GString {
        let file = file.to_string().to_lowercase();
        self.inner
            .content
            .get(&file)
            .unwrap_or(&"".to_string())
            .into()
    }

    #[func]
    pub fn get_files(&self) -> PackedStringArray {
        PackedStringArray::from_iter(self.inner.content.keys().map(|k| k.into()))
    }
}

impl DclContentMappingAndUrl {
    pub fn get_content_mapping(&self) -> ContentMappingAndUrlRef {
        self.inner.clone()
    }

    pub fn from_ref(ref_: ContentMappingAndUrlRef) -> Gd<DclContentMappingAndUrl> {
        Gd::from_init_fn(move |_base| DclContentMappingAndUrl { inner: ref_ })
    }

    pub fn empty() -> Gd<DclContentMappingAndUrl> {
        Gd::from_init_fn(move |_base| DclContentMappingAndUrl {
            inner: Arc::new(ContentMappingAndUrl::new()),
        })
    }
}
