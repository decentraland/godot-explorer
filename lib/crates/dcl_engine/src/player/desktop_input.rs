use godot::classes::{input::MouseMode, INode, Input, InputEvent, InputEventMouseMotion, Node, Os};
use godot::prelude::*;

use super::body::Player;
use crate::godot_classes::dcl_global::DclGlobal;

const VERTICAL_SENS: f32 = 0.5;
const HORIZONTAL_SENS: f32 = 0.5;

// macOS trackpad specific sensitivity.
const MACOS_VERTICAL_SENS: f32 = 0.3;
const MACOS_HORIZONTAL_SENS: f32 = 0.3;

/// Mouse look for the desktop player.
#[derive(GodotClass)]
#[class(no_init, base=Node)]
pub struct PlayerDesktopInput {
    base: Base<Node>,
    player: Gd<Player>,
    is_macos: bool,
}

impl PlayerDesktopInput {
    pub fn create(player: Gd<Player>) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            player,
            is_macos: Os::singleton().get_name() == "macOS",
        })
    }
}

#[godot_api]
impl INode for PlayerDesktopInput {
    fn input(&mut self, event: Gd<InputEvent>) {
        let focused = DclGlobal::explorer_has_focus();
        if !focused {
            return;
        }
        let Ok(motion) = event.try_cast::<InputEventMouseMotion>() else {
            return;
        };
        if Input::singleton().get_mouse_mode() != MouseMode::CAPTURED {
            return;
        }
        let (h_sens, v_sens) = if self.is_macos {
            (MACOS_HORIZONTAL_SENS, MACOS_VERTICAL_SENS)
        } else {
            (HORIZONTAL_SENS, VERTICAL_SENS)
        };
        // Smoothing for trackpad input on macOS.
        let mut relative = motion.get_relative();
        if self.is_macos {
            relative *= 0.8;
        }
        // The avatar is top-level, so the player's Y rotation does not propagate to it.
        let mut player = self.player.clone();
        player.rotate_y((-relative.x).to_radians() * h_sens);
        if let Some(mut mount) = player.bind().get_mount_camera() {
            mount.rotate_x((-relative.y).to_radians() * v_sens);
        }
        player.bind_mut().clamp_camera_rotation();
    }
}
