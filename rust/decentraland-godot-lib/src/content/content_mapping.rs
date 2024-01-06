use std::{collections::HashMap, sync::Arc};

use godot::prelude::*;

#[derive(Default)]
pub struct ContentMappingAndUrl {
    pub base_url: String,
    pub content: HashMap<String, String>,
}

impl ContentMappingAndUrl {
    pub fn new() -> Self {
        Default::default()
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
    fn initialize(&mut self, base_url: GString, dict: Dictionary) {
        if !self.inner.base_url.is_empty() {
            tracing::error!("Trying to modify an already initialized ContentMapping");
            return;
        }

        let Some(inner_mut) = Arc::get_mut(&mut self.inner) else {
            tracing::error!("Trying to modify an already initialized ContentMapping");
            return;
        };

        inner_mut.base_url = base_url.to_string();
        inner_mut.content = HashMap::from_iter(
            dict.iter_shared()
                .map(|(k, v)| (k.to_string().to_lowercase(), v.to_string())),
        );
    }

    #[func]
    pub fn get_base_url(&self) -> GString {
        self.inner.base_url.to_string().into()
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
