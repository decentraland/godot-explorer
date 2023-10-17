use godot::engine::{AudioStreamPlayer, ImageTexture};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclVideoPlayer {
    // Used to mute and restore the volume
    #[export]
    dcl_volume: f32,

    #[export]
    dcl_source: GodotString,

    #[export]
    dcl_texture: Option<Gd<ImageTexture>>,

    #[base]
    _base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl DclVideoPlayer {}
