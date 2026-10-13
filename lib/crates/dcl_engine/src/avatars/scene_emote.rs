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
}

/// Scene emote data passed from Rust to GDScript.
/// Now includes base_url and scene_id for content resolution.
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
    #[var]
    pub base_url: GString,
    #[var]
    pub scene_id: GString,
}

#[godot_api]
impl DclSceneEmoteData {
    #[func]
    pub fn create(
        glb_hash: GString,
        audio_hash: GString,
        looping: bool,
        base_url: GString,
        scene_id: GString,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            glb_hash,
            audio_hash,
            looping,
            base_url,
            scene_id,
        })
    }
}
