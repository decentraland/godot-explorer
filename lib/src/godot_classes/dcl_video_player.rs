use godot::classes::{ImageTexture, AudioStreamPlayer};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclVideoPlayer {
    // Used to mute and restore the volume
    #[export]
    dcl_volume: f32,

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

    pub fn set_muted(&mut self, value: bool) {
        if value {
            self.base_mut().set_volume_db(-80.0);
        } else {
            let db_volume = 20.0 * f32::log10(self.get_dcl_volume());
            self.base_mut().set_volume_db(db_volume);
        }
    }
}
