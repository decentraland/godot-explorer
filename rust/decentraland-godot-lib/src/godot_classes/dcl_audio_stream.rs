use godot::engine::AudioStreamPlayer;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclAudioStream {
    // Used to mute and restore the volume
    #[export]
    dcl_volume: f32,

    #[export]
    dcl_url: GodotString,

    #[base]
    _base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl DclAudioStream {}
