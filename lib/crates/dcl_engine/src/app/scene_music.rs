//! Ambient music on empty parcels: a random woodland track while the player stands on one, stopped
//! when a scene takes over.

use godot::classes::{INode, Node};
use godot::prelude::*;
use rand::seq::SliceRandom;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

const RANDOM_MUSIC: [&str; 3] = [
    "ambience_woods_day_nowind01",
    "ambience_woods_day_nowind02",
    "ambience_woods_day_nowind03",
];
/// The scene id the runner reports for a parcel with no scene.
const EMPTY_PARCEL: i32 = -1;

/// Whether `current` is one of the empty-parcel tracks.
pub fn is_ambient(current: &str) -> bool {
    RANDOM_MUSIC.contains(&current)
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct SceneMusicChanger {
    base: Base<Node>,
}

#[godot_api]
impl INode for SceneMusicChanger {
    fn ready(&mut self) {
        let me = self.to_gd();
        globals::scene_runner()
            .signals()
            .on_change_scene_id()
            .connect_other(&me, |this: &mut Self, scene_id| {
                this.on_change_scene_id(scene_id)
            });
    }
}

impl SceneMusicChanger {
    fn on_change_scene_id(&mut self, scene_id: i32) {
        let Some(mut music) = DclGlobal::singleton().bind().music_player.clone() else {
            return;
        };
        if scene_id == EMPTY_PARCEL {
            if let Some(track) = RANDOM_MUSIC.choose(&mut rand::thread_rng()) {
                music.bind_mut().play((*track).into());
            }
        } else if is_ambient(&music.bind().get_current_music().to_string()) {
            // No music plays on a parcel that has a scene.
            music.bind_mut().stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_ambient;

    #[test]
    fn only_the_woodland_tracks_are_ambient() {
        assert!(is_ambient("ambience_woods_day_nowind02"));
        assert!(!is_ambient("something_else"));
        assert!(!is_ambient(""));
    }
}
