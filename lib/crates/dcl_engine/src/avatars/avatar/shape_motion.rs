//! Gait for a scene-moved AvatarShape (Unity SDKAvatarShapesMotionSystem): the scene only writes
//! the entity's Transform, so the walk/jog/run state comes from the avatar's own position deltas.

use godot::prelude::*;

use crate::godot_classes::dcl_avatar::{DclAvatar, IDLE_SPEED_FLOOR};

/// Unity DISTANCE_EPSILON: below it the avatar has not moved.
const STEP_EPSILON: f32 = 0.0001;
/// One position change longer than this is a teleport, not a gait.
const TELEPORT_DISTANCE: f32 = 5.0;
/// Cap on the gap a step is divided by: the first step after a long stand still reads as motion.
const MAX_STEP_GAP: f32 = 0.5;

/// Speed estimate over the scene's position changes. Scene ticks are slower than render frames,
/// so a per-frame delta alternates between 0 and a multiple of the real speed: each step is
/// divided by the time since the previous step instead.
#[derive(Default)]
pub struct ShapeMotion {
    last_position: Option<Vector2>,
    since_step: f32,
    /// Step interval EMA: sets how long a still position takes to read as stopped.
    interval: f32,
    speed: f32,
}

impl ShapeMotion {
    /// Feeds one frame's horizontal position (x, z); returns the smoothed speed in m/s.
    pub fn sample(&mut self, position: Vector2, dt: f32) -> f32 {
        let Some(last) = self.last_position else {
            self.last_position = Some(position);
            return 0.0;
        };
        self.since_step += dt;
        let distance = position.distance_to(last);
        if distance < STEP_EPSILON {
            if self.since_step > (1.5 * self.interval).max(0.3) {
                self.speed = 0.0;
            }
            return self.speed;
        }
        self.last_position = Some(position);
        let gap = self.since_step.clamp(0.001, MAX_STEP_GAP);
        self.since_step = 0.0;
        if distance > TELEPORT_DISTANCE {
            self.speed = 0.0;
            return 0.0;
        }
        let measured = gap.clamp(0.03, MAX_STEP_GAP);
        self.interval = if self.interval <= 0.0 {
            measured
        } else {
            self.interval * 0.7 + measured * 0.3
        };
        // Same smoothing as the remote-avatar packets.
        self.speed = self.speed * 0.6 + (distance / gap) * 0.4;
        self.speed
    }
}

impl DclAvatar {
    /// Walk/jog/run from the scene-driven motion. A full-body emote wins: Unity only cancels
    /// emotes on player input, so an NPC dancing on a moving platform keeps dancing.
    pub(crate) fn shape_motion_process(&mut self, delta: f64) {
        let transform = self.base().get_global_transform();
        let position = Vector2::new(transform.origin.x, transform.origin.z);
        let world_speed = self.shape_motion.sample(position, delta as f32);
        let speed = if self.emote.playing_single || self.emote.playing_mixed {
            0.0
        } else {
            // The gait fits the body: a 3x avatar covers 3x the ground per stride.
            world_speed / transform.basis.get_scale().y.abs().max(0.01)
        };
        self.classify_locomotion(speed);
        // The idle floor is about ground covered, so it reads the world speed: a 9x body at
        // 3 m/s is 0.33 m/s for its size, a slow walk rather than an idle slide.
        let gait = self.walk || self.jog || self.run;
        if !gait && speed > 0.0 && world_speed > IDLE_SPEED_FLOOR {
            self.walk = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    /// Drives `seconds` of frames, the scene stepping the position every `tick_frames` frames.
    fn drive(motion: &mut ShapeMotion, speed: f32, tick_frames: usize, seconds: f32) -> f32 {
        let mut x = 0.0;
        let mut out = 0.0;
        let frames = (seconds / FRAME) as usize;
        for frame in 0..frames {
            if frame % tick_frames == 0 {
                x += speed * FRAME * tick_frames as f32;
            }
            out = motion.sample(Vector2::new(x, 0.0), FRAME);
        }
        out
    }

    #[test]
    fn stepped_scene_ticks_read_the_real_speed() {
        // A 30 Hz scene under a 60 fps render: half the frames see no change.
        let mut motion = ShapeMotion::default();
        let speed = drive(&mut motion, 3.0, 2, 2.0);
        assert!((speed - 3.0).abs() < 0.05, "read {speed}");
        // A 10 Hz scene.
        let mut motion = ShapeMotion::default();
        let speed = drive(&mut motion, 10.0, 6, 2.0);
        assert!((speed - 10.0).abs() < 0.1, "read {speed}");
    }

    #[test]
    fn stops_when_the_scene_stops_moving() {
        let mut motion = ShapeMotion::default();
        drive(&mut motion, 3.0, 2, 1.0);
        let position = motion.last_position.unwrap();
        let mut speed = 1.0;
        for _ in 0..30 {
            speed = motion.sample(position, FRAME);
        }
        assert_eq!(speed, 0.0);
    }

    #[test]
    fn a_teleport_is_not_a_gait() {
        let mut motion = ShapeMotion::default();
        motion.sample(Vector2::ZERO, FRAME);
        assert_eq!(motion.sample(Vector2::new(40.0, 30.0), FRAME), 0.0);
        assert_eq!(motion.sample(Vector2::new(40.0, 30.0), FRAME), 0.0);
    }
}
