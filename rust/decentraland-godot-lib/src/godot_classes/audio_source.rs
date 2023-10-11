use godot::engine::AudioStreamPlayer3D;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer3D)]
pub struct DCLAudioSource {
    /// whether the clip is currently playing.
    #[export]
    dcl_playing: bool,

    /// the audio volume (default: 1.0).
    #[export]
    dcl_volume: f32,

    /// whether the clip should restart when finished.
    #[export]
    dcl_loop_activated: bool,

    /// the audio pitch (default: 1.0).
    #[export]
    dcl_pitch: f32,

    /// the clip path as given in the `files` array of the scene's manifest.
    #[export]
    dcl_audio_clip_url: GodotString,

    #[export]
    dcl_scene_id: u32,

    #[base]
    _base: Base<AudioStreamPlayer3D>,
}

#[godot_api]
impl DCLAudioSource {}
