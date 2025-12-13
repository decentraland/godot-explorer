use godot::engine::ImageTexture;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclVideoPlayer {
    // Used to mute and restore the volume
    dcl_volume: f32,

    // Track muted state to know if we should apply volume changes
    is_muted: bool,

    #[export]
    dcl_source: GString,

    #[export]
    dcl_texture: Option<Gd<ImageTexture>>,

    base: Base<AudioStreamPlayer>,

    #[var]
    dcl_scene_id: i32,

    pub resolve_resource_sender: Option<tokio::sync::oneshot::Sender<String>>,
}

#[godot_api]
impl DclVideoPlayer {
    #[func]
    fn resolve_resource(&mut self, file_path: GString) {
        let Some(sender) = self.resolve_resource_sender.take() else {
            return;
        };
        let _ = sender.send(file_path.to_string());
    }

    #[func]
    pub fn get_dcl_volume(&self) -> f32 {
        self.dcl_volume
    }

    #[func]
    pub fn set_dcl_volume(&mut self, value: f32) {
        self.dcl_volume = value;
        // Update the actual audio volume if not muted
        if !self.is_muted {
            let db_volume = if value <= 0.0 {
                -80.0
            } else {
                20.0 * f32::log10(value)
            };
            self.base_mut().set_volume_db(db_volume);
        }
    }

    pub fn set_muted(&mut self, value: bool) {
        self.is_muted = value;
        if value {
            self.base_mut().set_volume_db(-80.0);
        } else {
            let db_volume = if self.dcl_volume <= 0.0 {
                -80.0
            } else {
                20.0 * f32::log10(self.dcl_volume)
            };
            self.base_mut().set_volume_db(db_volume);
        }
    }
}
