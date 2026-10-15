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
        self.base_mut().connect(
            "area_entered",
            &Callable::from_fn("camera_mode_area_entered", {
                let mut this = this.clone();
                move |args| {
                    if let Ok(area) = args[0].try_to::<Gd<DclCameraModeArea3D>>() {
                        this.bind_mut().overlapping.push(area);
                        this.bind().check_areas();
                    }
                    Variant::nil()
                }
            }),
        );
        self.base_mut().connect(
            "area_exited",
            &Callable::from_fn("camera_mode_area_exited", {
                let this = this.clone();
                move |args| {
                    if let Ok(area) = args[0].try_to::<Gd<DclCameraModeArea3D>>() {
                        this.clone().bind_mut().overlapping.retain(|a| a != &area);
                        this.bind().check_areas();
                    }
                    Variant::nil()
                }
            }),
        );
    }
}

#[godot_api]
impl CameraModeAreaDetector {
    #[signal]
    fn block_camera_mode(mode: i32);
    #[signal]
    fn unblock_camera_mode();
}

impl CameraModeAreaDetector {
    fn check_areas(&self) {
        let mut base = self.to_gd();
        match self.overlapping.last() {
            Some(area) => {
                let mode = area.bind().get_forced_camera_mode();
                base.emit_signal("block_camera_mode", &[mode.to_variant()]);
            }
            None => {
                base.emit_signal("unblock_camera_mode", &[]);
            }
        }
    }
}
