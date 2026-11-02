//! The ambient music: one stream on the `Music` bus, with the last track kept so it can resume.

use godot::classes::{AudioStream, AudioStreamPlayer, INode};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct MusicPlayer {
    base: Base<Node>,
    current: String,
    last: String,
    player: Option<Gd<AudioStreamPlayer>>,
}

#[godot_api]
impl INode for MusicPlayer {
    fn ready(&mut self) {
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_bus("Music");
        self.base_mut().add_child(&player);
        self.player = Some(player);
    }
}

#[godot_api]
impl MusicPlayer {
    /// Plays `res://assets/sfx/ambient/{name}.ogg`; an unknown name leaves what plays untouched.
    #[func]
    pub fn play(&mut self, music_name: StringName) {
        let name = music_name.to_string();
        let path = format!("res://assets/sfx/ambient/{name}.ogg");
        let Ok(stream) = godot::tools::try_load::<AudioStream>(&path) else {
            return;
        };
        let Some(player) = self.player.as_mut() else {
            return;
        };
        if !self.current.is_empty() {
            self.last = self.current.clone();
        }
        player.set_stream(&stream);
        player.play();
        self.current = name;
    }

    #[func]
    pub fn restore_music(&mut self) {
        let last = self.last.as_str().into();
        self.play(last);
    }

    #[func]
    pub fn stop(&mut self) {
        self.last = std::mem::take(&mut self.current);
        if let Some(player) = self.player.as_mut() {
            player.stop();
        }
    }

    #[func]
    pub fn get_current_music(&self) -> GString {
        self.current.as_str().into()
    }
}
