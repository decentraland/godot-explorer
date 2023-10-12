use godot::engine::Area3D;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct DCLCameraModeArea3D {
    #[export]
    forced_camera_mode: i32,

    #[export]
    area: Vector3,

    #[base]
    _base: Base<Area3D>,
}

#[godot_api]
impl DCLCameraModeArea3D {}
