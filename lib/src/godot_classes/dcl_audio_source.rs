use godot::classes::AudioStreamPlayer3D;
use godot::prelude::*;

/// Clip loading state reported by GDScript. Combined with the node's playback to
/// derive the `MediaState` of `PBAudioEvent`.
pub const CLIP_STATE_NONE: i32 = 0;
pub const CLIP_STATE_LOADING: i32 = 1;
pub const CLIP_STATE_READY: i32 = 2;
pub const CLIP_STATE_ERROR: i32 = 3;


#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer3D)]
pub struct DclAudioSource {
    #[var]
    dcl_audio_clip_url: GString,

    #[var]
    dcl_playing: bool,

    #[var]
    dcl_volume: f32,

    #[var]
    dcl_loop_activated: bool,

    #[var]
    dcl_pitch: f32,

    #[var]
    dcl_global: bool,

    #[var]
    dcl_current_time: f32,

    #[var]
    dcl_enable: bool,

    #[var]
    dcl_scene_id: i32,

    /// See `CLIP_STATE_*`.
    #[var]
    dcl_clip_state: i32,

    /// Last `MediaState` appended to `PBAudioEvent`, to avoid re-emitting it.
    pub last_media_state: i32,


    base: Base<AudioStreamPlayer3D>,
}

#[godot_api]
impl DclAudioSource {}

impl DclAudioSource {
    pub fn is_clip_playing(&self) -> bool {
        self.base().is_playing()
    }
}
