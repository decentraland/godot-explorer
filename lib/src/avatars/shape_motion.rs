//! Gait for a scene-moved AvatarShape (Unity SDKAvatarShapesMotionSystem): the scene only writes
//! the entity's Transform, so the walk/jog/run state comes from the avatar's own position deltas.

use godot::prelude::*;

use crate::godot_classes::dcl_avatar::{DclAvatar, IDLE_SPEED_FLOOR};

/// Unity DISTANCE_EPSILON: below it the avatar has not moved.
const STEP_EPSILON: f32 = 0.0001;
/// From a stand, one position change longer than this is a teleport, not a gait.
const TELEPORT_DISTANCE: f32 = 5.0;
/// While moving, a step faster than this is a teleport: slow scene ticks make long steps, so
/// a distance alone can't tell them from a jump.
const TELEPORT_SPEED: f32 = 50.0;
/// Cap on the gap the first step after a stand is divided by, so it reads as motion.
const MAX_STEP_GAP: f32 = 0.5;
/// Bounds of the step interval estimate. The upper one is the slowest scene tick that still
/// reads as continuous motion (0.5 Hz).
const MIN_INTERVAL: f32 = 0.03;
const MAX_INTERVAL: f32 = 2.0;
/// Below this height the body is hidden (scenes shrink NPCs to hide them), not walking.
const MIN_BODY_SCALE: f32 = 0.05;

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
            if self.since_step > self.stop_after() {
                self.speed = 0.0;
            }
            return self.speed;
        }
        // Standing: no step for longer than a stop takes to read.
        let from_stand = self.since_step > self.stop_after();
        let since_step = self.since_step.max(0.001);
        self.last_position = Some(position);
        self.since_step = 0.0;
        // Every step, teleports included, so a slow scene's period is learnt from its first steps.
        let measured = since_step.clamp(MIN_INTERVAL, MAX_INTERVAL);
        self.interval = if self.interval <= 0.0 {
            measured
        } else {
            self.interval * 0.7 + measured * 0.3
        };
        let (gap, teleport) = if from_stand {
            (since_step.min(MAX_STEP_GAP), distance > TELEPORT_DISTANCE)
        } else {
            (since_step, distance / since_step > TELEPORT_SPEED)
        };
        if teleport {
            self.speed = 0.0;
            return 0.0;
        }
        // Same smoothing as the remote-avatar packets.
        self.speed = self.speed * 0.6 + (distance / gap) * 0.4;
        self.speed
    }

    /// How long a still position takes to read as stopped: past the scene's step interval.
    fn stop_after(&self) -> f32 {
        (1.5 * self.interval).max(0.3)
    }
}

/// Speed for the body's size, which picks the gait. Height, not footprint: stride length follows
/// leg length, so a (3, 1, 1) NPC is a wide body on 1x legs.
fn body_speed(world_speed: f32, scale_y: f32, emoting: bool) -> f32 {
    if emoting || scale_y.abs() < MIN_BODY_SCALE {
        return 0.0;
    }
    world_speed / scale_y.abs()
}

impl DclAvatar {
    /// Walk/jog/run from the scene-driven motion, each frame for AvatarShapes. A full-body emote
    /// wins: Unity only cancels emotes on player input, so a dancing NPC keeps dancing.
    pub(crate) fn update_shape_motion(&mut self, delta: f64, emoting: bool) {
        let transform = self.base().get_global_transform();
        let position = Vector2::new(transform.origin.x, transform.origin.z);
        let world_speed = self.shape_motion.sample(position, delta as f32);
        let speed = body_speed(world_speed, transform.basis.get_scale().y, emoting);
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

    /// Every sample over `seconds`, after `warm_up` seconds of the same motion.
    fn samples(speed: f32, tick_frames: usize, warm_up: f32, seconds: f32) -> Vec<f32> {
        let mut motion = ShapeMotion::default();
        let mut x = 0.0;
        let warm = (warm_up / FRAME) as usize;
        let frames = warm + (seconds / FRAME) as usize;
        let mut out = Vec::new();
        for frame in 0..frames {
            if frame % tick_frames == 0 {
                x += speed * FRAME * tick_frames as f32;
            }
            let read = motion.sample(Vector2::new(x, 0.0), FRAME);
            if frame >= warm {
                out.push(read);
            }
        }
        out
    }

    #[test]
    fn one_hertz_ticks_read_the_real_speed() {
        // A scene that moves its NPC once a second, 3 m per step.
        let mut motion = ShapeMotion::default();
        let speed = drive(&mut motion, 3.0, 60, 8.0);
        assert!((speed - 3.0).abs() < 0.15, "read {speed}");
    }

    #[test]
    fn one_hertz_ticks_keep_walking() {
        // Between two steps the NPC is still moving: no frame may read idle (0) or a jog (>= 4).
        let read = samples(3.0, 60, 4.0, 5.0);
        let (min, max) = read
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        assert!(min > 0.0 && max < 4.0, "read between {min} and {max}");
    }

    #[test]
    fn big_steps_from_slow_ticks_are_not_a_teleport() {
        // A 6x NPC at 30 m/s in a 5 Hz scene: 6 m per step, still a run for its size.
        let mut motion = ShapeMotion::default();
        let speed = drive(&mut motion, 30.0, 12, 3.0);
        assert!((speed - 30.0).abs() < 1.0, "read {speed}");
    }

    #[test]
    fn a_long_jump_after_standing_is_a_teleport() {
        let mut motion = ShapeMotion::default();
        for _ in 0..600 {
            motion.sample(Vector2::ZERO, FRAME);
        }
        assert_eq!(motion.sample(Vector2::new(20.0, 0.0), FRAME), 0.0);
    }

    #[test]
    fn a_shrunk_body_does_not_run() {
        // Scenes hide an NPC by scaling it to almost nothing: its drift is not a gait.
        assert_eq!(body_speed(0.1, 0.001, false), 0.0);
        assert_eq!(body_speed(3.0, 1.0, false), 3.0);
        assert!((body_speed(10.0, 3.0, false) - 10.0 / 3.0).abs() < 1e-5);
        assert_eq!(body_speed(3.0, 1.0, true), 0.0);
    }

    #[test]
    fn a_teleport_is_not_a_gait() {
        let mut motion = ShapeMotion::default();
        motion.sample(Vector2::ZERO, FRAME);
        assert_eq!(motion.sample(Vector2::new(40.0, 30.0), FRAME), 0.0);
        assert_eq!(motion.sample(Vector2::new(40.0, 30.0), FRAME), 0.0);
    }
}
