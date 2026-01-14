use godot::prelude::*;

/// Scene emote data passed from Rust to GDScript.
/// This replaces the fragile URN string encoding for scene emotes.
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

/// Parse compound hash from server: "{audio_hash}-{glb_hash}" or just "{glb_hash}"
///
/// The server returns compound hashes where:
/// - First part (bafkrei* raw multicodec) = audio file
/// - Second part (bafybei* dag-pb) = GLB/GLTF file
pub fn parse_compound_hash(file_hash: &str) -> (String, Option<String>) {
    if let Some(dash_pos) = file_hash.find('-') {
        // Two hashes: first = audio, second = glb
        let audio = &file_hash[..dash_pos];
        let glb = &file_hash[dash_pos + 1..];
        (glb.to_string(), Some(audio.to_string()))
    } else {
        // Single hash = GLB only
        (file_hash.to_string(), None)
    }
}
