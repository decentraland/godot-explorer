use godot::classes::{Area3D, IArea3D};
use godot::prelude::*;

use crate::godot_classes::dcl_camera_mode_area_3d::DclCameraModeArea3D;

/// Reports the forced camera mode of the scene's camera-mode areas the player stands in.
#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct CameraModeAreaDetector {
    base: Base<Area3D>,
    overlapping: Vec<Gd<DclCameraModeArea3D>>,
}

#[godot_api]
impl IArea3D for CameraModeAreaDetector {
    fn ready(&mut self) {
        let this = self.to_gd();
        let base = self.base().clone();
        base.signals()
            .area_entered()
            .connect_other(&this, |this: &mut Self, area: Gd<Area3D>| {
                if let Ok(area) = area.try_cast::<DclCameraModeArea3D>() {
                    this.overlapping.push(area);
                    this.check_areas();
                }
            });
        base.signals()
            .area_exited()
            .connect_other(&this, |this: &mut Self, area: Gd<Area3D>| {
                if let Ok(area) = area.try_cast::<DclCameraModeArea3D>() {
                    this.overlapping.retain(|a| a != &area);
                    this.check_areas();
                }
            });
    }
}

#[godot_api]
impl CameraModeAreaDetector {
    #[signal]
    pub fn block_camera_mode(mode: i32);
    #[signal]
    pub fn unblock_camera_mode();
}

impl CameraModeAreaDetector {
    fn check_areas(&self) {
        let base = self.to_gd();
        match self.overlapping.last() {
            Some(area) => {
                let mode = area.bind().get_forced_camera_mode();
                base.signals().block_camera_mode().emit(mode);
            }
            None => {
                base.signals().unblock_camera_mode().emit();
            }
        }
    }
}
