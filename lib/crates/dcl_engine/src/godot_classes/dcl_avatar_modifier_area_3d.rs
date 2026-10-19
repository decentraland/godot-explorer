use godot::classes::{Area3D, BoxShape3D, CollisionShape3D, IArea3D};

use crate::godot_classes::dcl_scene_node::DclSceneNode;
use godot::prelude::*;

#[derive(Clone, Copy, Var, GodotConvert, Export, PartialEq, Debug)]
#[godot(via = i32)]
pub enum AvatarModifierType {
    HideAvatar = 0,
    DisablePassports = 1,
    HideNameTags = 2,
}

#[derive(GodotClass)]
#[class(base=Area3D)]
pub struct DclAvatarModifierArea3D {
    #[export(enum = (HideAvatar, DisablePassports, HideNameTags))]
    #[var(pub)]
    avatar_modifiers: Array<i32>,

    #[export]
    #[var(pub)]
    exclude_ids: Array<GString>,

    #[export]
    #[var(pub)]
    area: Vector3,

    /// The scene the area belongs to; -1 until it is placed in one.
    #[var(pub)]
    scene_id: i32,

    _base: Base<Area3D>,
}

#[godot_api]
impl IArea3D for DclAvatarModifierArea3D {
    fn ready(&mut self) {
        if let Some(scene) = DclSceneNode::search(&self.base().clone().upcast::<Node3D>()) {
            self.scene_id = scene.bind().get_scene_id();
        }
        let mut shape = BoxShape3D::new_gd();
        shape.set_size(self.area);
        if let Some(mut collision) = self
            .base()
            .get_node_or_null("CollisionShape3D")
            .and_then(|n| n.try_cast::<CollisionShape3D>().ok())
        {
            collision.set_shape(&shape);
        }
    }

    fn init(base: Base<Area3D>) -> Self {
        Self {
            avatar_modifiers: Array::new(),
            exclude_ids: Array::new(),
            area: Vector3::ZERO,
            scene_id: -1,
            _base: base,
        }
    }
}

impl DclAvatarModifierArea3D {
    /// Makes every avatar evaluate its modifier areas again: scenes commonly push `excludeIds`
    /// after the avatar entered the area, and the setters alone do not re-trigger detection.
    /// `Area3D::get_overlapping_areas` is not used: it is stale during a CRDT update tick
    /// (physics has not settled), so each avatar's detector decides whether this area applies
    /// to it (#2166).
    pub fn refresh_overlapping_detectors(&self) {
        for mut avatar in crate::avatars::impostor_capture::registered() {
            avatar.bind_mut().try_show();
        }
    }
}
