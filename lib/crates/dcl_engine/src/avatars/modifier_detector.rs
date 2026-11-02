//! The Area3D every avatar carries to learn which `AvatarModifierArea`s it stands in.

use godot::classes::{Area3D, IArea3D};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_avatar_modifier_area_3d::DclAvatarModifierArea3D;

#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct AvatarModifierAreaDetector {
    overlapping: Vec<Gd<DclAvatarModifierArea3D>>,
    base: Base<Area3D>,
}

#[godot_api]
impl IArea3D for AvatarModifierAreaDetector {
    fn ready(&mut self) {
        let this = self.to_gd();
        let base = self.base().clone();
        base.signals()
            .area_entered()
            .connect_other(&this, |this: &mut Self, area| this.on_area_entered(area));
        base.signals()
            .area_exited()
            .connect_other(&this, |this: &mut Self, area| this.on_area_exited(area));
        // Arbitrary order.
        for area in self.base().get_overlapping_areas().iter_shared() {
            self.track(area);
        }
    }
}

impl AvatarModifierAreaDetector {
    fn track(&mut self, area: Gd<Area3D>) {
        if let Ok(area) = area.try_cast::<DclAvatarModifierArea3D>() {
            self.overlapping.push(area);
        }
    }

    fn avatar(&self) -> Option<Gd<DclAvatar>> {
        self.base().get_parent()?.try_cast::<DclAvatar>().ok()
    }

    /// The areas of the avatar's own scene: an area only modifies avatars inside its scene.
    pub fn areas_in_scene(&self, scene_id: i32) -> Vec<Gd<DclAvatarModifierArea3D>> {
        self.overlapping
            .iter()
            .filter(|area| area.is_instance_valid() && area.bind().get_scene_id() == scene_id)
            .cloned()
            .collect()
    }
}

#[godot_api]
impl AvatarModifierAreaDetector {
    #[func]
    fn on_area_entered(&mut self, area: Gd<Area3D>) {
        self.track(area);
        self.reevaluate();
    }

    #[func]
    fn on_area_exited(&mut self, area: Gd<Area3D>) {
        self.overlapping
            .retain(|a| a.instance_id() != area.instance_id());
        self.reevaluate();
    }

    /// Hands the avatar the areas it is in. Called from the signals, never from inside a call
    /// that already holds the avatar.
    fn reevaluate(&mut self) {
        let Some(mut avatar) = self.avatar() else {
            return;
        };
        let scene_id = avatar.bind().current_parcel_scene_id;
        let areas = self.areas_in_scene(scene_id);
        avatar.bind_mut().apply_modifier_areas(&areas);
    }
}
