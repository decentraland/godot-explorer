use godot::engine::Camera3D;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct DCLCamera3D {
    #[var]
    camera_mode: i32,

    #[base]
    _base: Base<Camera3D>,
}

#[godot_api]
impl DCLCamera3D {}
