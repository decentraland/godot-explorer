use godot::classes::INode3D;
use godot::prelude::*;

/// Keeps `target` at a global scale of one: an area node under a scaled entity would otherwise
/// scale its collision box with it.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct ForceGlobalScaleComponent {
    #[export]
    target: Option<Gd<Node3D>>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for ForceGlobalScaleComponent {
    fn process(&mut self, _delta: f64) {
        let Some(target) = self.target.as_mut() else {
            return;
        };
        let global_scale = target.get_global_transform().basis.get_scale();
        if global_scale != Vector3::ONE {
            let local = target.get_scale();
            target.set_scale(Vector3::new(
                local.x / global_scale.x,
                local.y / global_scale.y,
                local.z / global_scale.z,
            ));
        }
    }
}
