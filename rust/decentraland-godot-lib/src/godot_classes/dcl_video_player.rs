use godot::engine::{AudioStreamPlayer, ImageTexture};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclVideoPlayer {
    // Used to mute and restore the volume
    #[export]
    dcl_volume: f32,

    #[export]
    dcl_source: GodotString,

    #[export]
    dcl_texture: Option<Gd<ImageTexture>>,

    #[base]
    base: Base<AudioStreamPlayer>,

    #[var]
    dcl_scene_id: u32,

    pub resolve_resource_sender: Option<tokio::sync::oneshot::Sender<String>>,
}

#[godot_api]
impl DclVideoPlayer {
    #[func]
    fn resolve_resource(&mut self, file_path: GodotString) {
        let Some(sender) = self.resolve_resource_sender.take() else {
            return;
        };
        let _ = sender.send(file_path.to_string());
    }

    pub fn set_muted(&mut self, value: bool) {
        if value {
            self.base.set_volume_db(-80.0);
        } else {
            let db_volume = 20.0 * f32::log10(self.get_dcl_volume());
            self.base.set_volume_db(db_volume);
        }
    }
}
