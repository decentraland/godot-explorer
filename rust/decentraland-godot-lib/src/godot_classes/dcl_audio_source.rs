use godot::engine::AudioStreamPlayer3D;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer3D)]
pub struct DclAudioSource {
    #[var]
    dcl_enable: bool,

    /// whether the clip is currently playing.
    #[var]
    dcl_playing: bool,

    /// the audio volume (default: 1.0).
    #[var]
    dcl_volume: f32,

    /// whether the clip should restart when finished.
    #[var]
    dcl_loop_activated: bool,

    /// the audio pitch (default: 1.0).
    #[var]
    dcl_pitch: f32,

    /// the clip path as given in the `files` array of the scene's manifest.
    #[var]
    dcl_audio_clip_url: GodotString,

    #[var]
    dcl_scene_id: i32,

    #[base]
    _base: Base<AudioStreamPlayer3D>,
}

#[godot_api]
impl DclAudioSource {}
