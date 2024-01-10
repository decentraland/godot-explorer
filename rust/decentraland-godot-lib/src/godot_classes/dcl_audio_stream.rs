use godot::engine::AudioStreamPlayer;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=AudioStreamPlayer)]
pub struct DclAudioStream {
    // Used to mute and restore the volume
    #[export]
    dcl_volume: f32,

    #[export]
    dcl_url: GString,

    #[base]
    base: Base<AudioStreamPlayer>,
}

#[godot_api]
impl DclAudioStream {
    pub fn set_muted(&mut self, value: bool) {
        if value {
            self.base.set_volume_db(-80.0);
        } else {
            let db_volume = 20.0 * f32::log10(self.get_dcl_volume());
            self.base.set_volume_db(db_volume);
        }
    }
}
