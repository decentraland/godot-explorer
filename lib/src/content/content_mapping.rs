use std::{collections::HashMap, sync::Arc};

use godot::prelude::*;

use crate::avatars::scene_emote::SceneEmoteHash;
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

    /// Get scene emote data for an emote file.
    /// Returns GLB hash and searches for associated audio file by extension.
    pub fn get_scene_emote_hash(&self, emote_file: &str) -> Option<SceneEmoteHash> {
        // Get the GLB hash
        let glb_hash = self.get_hash(emote_file)?.clone();

        // Find audio file with same base name but audio extension
        let emote_file_lower = emote_file.to_lowercase();
        let base_name = emote_file_lower
            .strip_suffix(".glb")
            .or_else(|| emote_file_lower.strip_suffix(".gltf"))
            .unwrap_or(&emote_file_lower);

        let audio_hash = self.find_audio_for_base_name(base_name);

        tracing::debug!(
            "get_scene_emote_hash: file={}, glb_hash={}, base_name={}, audio_hash={:?}",
            emote_file,
            glb_hash,
            base_name,
            audio_hash
        );

        Some(SceneEmoteHash::new(glb_hash, audio_hash))
    }

    /// Find audio file hash for a given base name (without extension).
    /// Searches for .mp3 or .ogg files with the same base name.
    fn find_audio_for_base_name(&self, base_name: &str) -> Option<String> {
        // Try common audio extensions
        for ext in &[".mp3", ".ogg"] {
            let audio_file = format!("{}{}", base_name, ext);
            if let Some(hash) = self.content.get(&audio_file) {
                tracing::debug!(
                    "find_audio_for_base_name: found audio file={}, hash={}",
                    audio_file,
                    hash
                );
                return Some(hash.clone());
            }
        }
        tracing::debug!(
            "find_audio_for_base_name: no audio found for base_name={}",
            base_name
        );
        None
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
    fn from_values(base_url: GString, dict: VarDictionary) -> Gd<DclContentMappingAndUrl> {
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
