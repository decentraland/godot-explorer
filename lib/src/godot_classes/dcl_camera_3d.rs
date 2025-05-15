use godot::prelude::*;
#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct DclCamera3D {
    #[export]
    camera_mode: i32,

    #[export]
    smoothing_speed: f32,

    #[export]
    target_fov: f32,

    _base: Base<Camera3D>,
}

#[godot_api]
impl INode3D for DclCamera3D {
    fn process(&mut self, delta: f64) {
        let speed = self.get_smoothing_speed();
        let current_fov = self.base().get_fov();
        let new_fov = current_fov + (self.get_target_fov() - current_fov) * speed * (delta as f32);
        self.base_mut().set_fov(new_fov);
    }
}
