//! Footsteps, jump and landing sounds of an avatar, picked by its locomotion flags.

use godot::classes::{AudioStream, AudioStreamPlayer3D, INode3D, Node3D, Time};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;

const WALK_COUNT: usize = 8;
const JOG_COUNT: usize = 4;
const RUN_COUNT: usize = 8;
const JUMP_COUNT: usize = 3;
const LAND_COUNT: usize = 2;

const WALK_INTERVAL_MS: u64 = 500;
const JOG_INTERVAL_MS: u64 = 350;
const RUN_INTERVAL_MS: u64 = 300;

fn sound(kind: &str, index: usize) -> Gd<AudioStream> {
    load(
        format!(
            "res://assets/sfx/avatar/avatar_footstep_{kind}{:02}.wav",
            index + 1
        )
        .as_str(),
    )
}

/// The next index of a round-robin over `count` sounds.
pub fn next_index(index: usize, count: usize) -> usize {
    if index < count - 1 {
        index + 1
    } else {
        0
    }
}

/// Both players keep `max_distance = 50` where SDK audio sources use 0: deliberate. Godot's
/// `max_distance` adds a linear ramp on top of the 1/d rolloff, so this is not Unity-exact
/// (about -1.9 dB at 10 m), but it stops one voice per avatar mixing forever at a level nobody
/// can hear. SDK sources are single and scene-authored; avatars are many.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct AvatarSfxs {
    next_tick: u64,
    walk_index: usize,
    jog_index: usize,
    run_index: usize,
    last_land: bool,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for AvatarSfxs {
    fn ready(&mut self) {
        if let Some(avatar) = self.avatar() {
            self.last_land = avatar.bind().land;
        }
    }

    fn process(&mut self, _delta: f64) {
        let Some(avatar) = self.avatar() else {
            return;
        };
        let (blocked, land, run, jog, walk, rise) = {
            let avatar = avatar.bind();
            (
                avatar.blocked,
                avatar.land,
                avatar.run,
                avatar.jog,
                avatar.walk,
                avatar.rise,
            )
        };
        if blocked {
            return;
        }
        let (Some(mut steps), Some(mut effects)) = (
            self.player("AudioPlayer_Steps"),
            self.player("AudioPlayer_Effects"),
        ) else {
            return;
        };
        let now = Time::singleton().get_ticks_msec();
        if !steps.is_playing() && land && now > self.next_tick {
            if run {
                steps.set_stream(&sound("run", self.run_index));
                steps.play();
                self.run_index = next_index(self.run_index, RUN_COUNT);
                self.next_tick = now + RUN_INTERVAL_MS;
            } else if jog {
                steps.set_stream(&sound("light", self.jog_index));
                steps.play();
                self.jog_index = next_index(self.jog_index, JOG_COUNT);
                self.next_tick = now + JOG_INTERVAL_MS;
            } else if walk {
                steps.set_stream(&sound("walk", self.walk_index));
                steps.play();
                self.walk_index = next_index(self.walk_index, WALK_COUNT);
                self.next_tick = now + WALK_INTERVAL_MS;
            }
        }

        if self.last_land != land {
            self.last_land = land;
            // Leaving the ground is a jump; coming back to it is a landing.
            let stream = if !rise {
                sound("jump", random_index(JUMP_COUNT))
            } else {
                sound("land", random_index(LAND_COUNT))
            };
            effects.set_stream(&stream);
            effects.play();
        }
    }
}

fn random_index(count: usize) -> usize {
    (godot::global::randi() as usize) % count
}

impl AvatarSfxs {
    fn avatar(&self) -> Option<Gd<DclAvatar>> {
        self.base().get_parent()?.try_cast().ok()
    }

    fn player(&self, name: &str) -> Option<Gd<AudioStreamPlayer3D>> {
        self.base().get_node_or_null(name)?.try_cast().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::next_index;

    #[test]
    fn round_robin_wraps() {
        assert_eq!(next_index(0, 4), 1);
        assert_eq!(next_index(3, 4), 0);
    }
}
