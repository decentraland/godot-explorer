use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct DclCamera3D {
    #[export]
    camera_mode: i32,

    #[base]
    _base: Base<Camera3D>,
}

#[godot_api]
impl DclCamera3D {}
