use godot::prelude::*;

/// Scene emote data with GLB hash and optional audio hash.
/// Audio is found by searching content mapping for files with same base name and audio extension.
#[derive(Debug, Clone)]
pub struct SceneEmoteHash {
    pub glb_hash: String,
    pub audio_hash: Option<String>,
}

impl SceneEmoteHash {
    /// Create from GLB hash and optional audio hash
    pub fn new(glb_hash: String, audio_hash: Option<String>) -> Self {
        Self {
            glb_hash,
            audio_hash,
        }
    }

    /// Create DclSceneEmoteData for passing to GDScript
    pub fn to_godot_data(&self, looping: bool) -> Gd<DclSceneEmoteData> {
        DclSceneEmoteData::create(
            self.glb_hash.as_str().into(),
            self.audio_hash.as_deref().unwrap_or_default().into(),
            looping,
        )
    }
}

/// Scene emote data passed from Rust to GDScript.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclSceneEmoteData {
    base: Base<RefCounted>,
    #[var]
    pub glb_hash: GString,
    #[var]
    pub audio_hash: GString,
    #[var]
    pub looping: bool,
}

#[godot_api]
impl DclSceneEmoteData {
    #[func]
    pub fn create(glb_hash: GString, audio_hash: GString, looping: bool) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            glb_hash,
            audio_hash,
            looping,
        })
    }
}
