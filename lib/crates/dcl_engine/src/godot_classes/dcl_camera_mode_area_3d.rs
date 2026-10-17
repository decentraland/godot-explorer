use godot::classes::{Area3D, BoxShape3D, CollisionShape3D, IArea3D};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_scene_node::DclSceneNode;

/// Forces a camera mode while the player stands in it and its scene is the current one.
#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct DclCameraModeArea3D {
    #[export]
    #[var(pub)]
    forced_camera_mode: i32,

    #[export]
    #[var(pub)]
    area: Vector3,

    my_scene_id: i32,

    base: Base<Area3D>,
}

#[godot_api]
impl IArea3D for DclCameraModeArea3D {
    fn ready(&mut self) {
        let Some(mut collision) = self.collision_shape() else {
            return;
        };
        let mut shape = BoxShape3D::new_gd();
        shape.set_size(self.area);
        collision.set_shape(&shape);
        collision.set_disabled(true);

        let Some(scene) = DclSceneNode::search(&self.base().clone().upcast::<Node3D>()) else {
            return;
        };
        if scene.bind().is_global() {
            // Always active.
            collision.set_disabled(false);
            return;
        }
        self.my_scene_id = scene.bind().get_scene_id();

        self.run_deferred(|this| this.deferred_start());
        let this = self.to_gd();
        globals::scene_runner()
            .signals()
            .on_change_scene_id()
            .connect_other(&this, |this: &mut Self, scene_id| {
                this.on_change_scene_id(scene_id)
            });
    }
}

#[godot_api]
impl DclCameraModeArea3D {
    #[func]
    fn on_change_scene_id(&mut self, scene_id: i32) {
        self.set_scene_active(self.my_scene_id == scene_id);
    }

    /// The first evaluation sets the initial value.
    #[func]
    fn deferred_start(&mut self) {
        let current = globals::scene_runner().bind().get_current_parcel_scene_id();
        self.set_scene_active(self.my_scene_id == current);
    }
}

impl DclCameraModeArea3D {
    fn collision_shape(&self) -> Option<Gd<CollisionShape3D>> {
        self.base()
            .get_node_or_null("CollisionShape3D")?
            .try_cast()
            .ok()
    }

    fn set_scene_active(&mut self, active: bool) {
        if let Some(mut shape) = self.collision_shape() {
            shape.set_disabled(!active);
        }
    }
}
