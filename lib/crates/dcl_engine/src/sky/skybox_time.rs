//! The sky's clock: the normalized time of day the sky shader and the sun follow. A scene can take
//! it over (PBSkyboxTime), a test can pin it, otherwise it follows the player's setting or the
//! world time.

use godot::classes::{INode, RenderingServer};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_global_time::DclGlobalTime;

/// Normalized time per second a scene-driven change moves at: 0.5 is half a day per second.
const TRANSITION_SPEED: f32 = 0.5;
const SECONDS_PER_DAY: f32 = 86_400.0;
/// 3 pm, for the fixed-skybox test mode.
const FIXED_TEST_TIME: f32 = 0.625;
/// The debug mode runs a whole day in this many seconds.
const DEBUG_DAY_SECONDS: f32 = 10.0;

/// One step of `current` toward `target` at the transition speed, going the way `forward` says
/// through midnight (a backward change may take the long way round), wrapped to `[0, 1)`.
pub fn interpolate_time(current: f32, target: f32, forward: bool, delta: f32) -> f32 {
    let mut diff = target - current;
    if forward {
        if diff < 0.0 {
            diff += 1.0;
        }
    } else if diff > 0.0 {
        diff -= 1.0;
    }
    let step = TRANSITION_SPEED * delta;
    if diff.abs() <= step {
        return target;
    }
    let mut next = current + diff.signum() * step;
    if next < 0.0 {
        next += 1.0;
    } else if next >= 1.0 {
        next -= 1.0;
    }
    next
}

fn is_approx(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-5 * a.abs().max(b.abs()).max(1.0)
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct SkyboxTime {
    base: Base<Node>,
    /// Makes a full day and night cycle every 10 seconds.
    #[export]
    debug_time_rotation: bool,
    last_value: f32,
    normalized_time: f32,
    debug_accumulator: f32,
    is_transitioning: bool,
}

#[godot_api]
impl INode for SkyboxTime {
    fn process(&mut self, delta: f64) {
        let delta = delta as f32;
        if self.debug_time_rotation {
            self.debug_accumulator += delta;
            self.normalized_time = (self.debug_accumulator / DEBUG_DAY_SECONDS) % 1.0;
            self.publish();
            return;
        }

        let (sdk_active, sdk_time, sdk_forward, fixed) = {
            let global = DclGlobal::singleton();
            let global = global.bind();
            (
                global.sdk_skybox_time_active,
                global.sdk_skybox_fixed_time,
                global.sdk_skybox_transition_forward,
                global.fixed_skybox_time,
            )
        };
        if sdk_active {
            // The scene controls it: ease toward its time.
            let target = sdk_time as f32 / SECONDS_PER_DAY;
            if is_approx(self.normalized_time, target) {
                self.is_transitioning = false;
            } else {
                self.is_transitioning = true;
                self.normalized_time =
                    interpolate_time(self.normalized_time, target, sdk_forward, delta);
            }
        } else if fixed {
            self.normalized_time = FIXED_TEST_TIME;
            self.is_transitioning = false;
        } else {
            let target = self.settings_target();
            // Out of scene control, ease back if it is still on its way.
            if self.is_transitioning {
                self.normalized_time = interpolate_time(self.normalized_time, target, true, delta);
                if is_approx(self.normalized_time, target) {
                    self.is_transitioning = false;
                }
            } else {
                self.normalized_time = target;
            }
        }
        self.publish();
    }
}

#[godot_api]
impl SkyboxTime {
    #[func]
    pub fn get_normalized_time(&self) -> f32 {
        self.normalized_time
    }
}

impl SkyboxTime {
    /// The time the player's setting asks for: the world's clock, or a fixed time.
    fn settings_target(&self) -> f32 {
        let engine = globals::engine();
        let engine = engine.bind();
        let dynamic = engine
            .setting("dynamic_skybox".into())
            .try_to::<bool>()
            .unwrap_or(false);
        let seconds = if dynamic {
            DclGlobalTime::get_world_time() as f32
        } else {
            engine
                .setting("skybox_time".into())
                .try_to::<f64>()
                .unwrap_or(0.0) as f32
        };
        seconds / SECONDS_PER_DAY
    }

    fn publish(&mut self) {
        if self.last_value != self.normalized_time {
            RenderingServer::singleton()
                .global_shader_parameter_set("day_night_cycle", &self.normalized_time.to_variant());
            self.last_value = self.normalized_time;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_moves_at_the_transition_speed() {
        let next = interpolate_time(0.2, 0.9, true, 0.1);
        assert!((next - 0.25).abs() < 1e-6);
    }

    #[test]
    fn close_enough_snaps_to_the_target() {
        assert_eq!(interpolate_time(0.5, 0.52, true, 0.1), 0.52);
    }

    #[test]
    fn forward_goes_through_midnight_when_the_target_is_behind() {
        // From 0.9 toward 0.1 going forward passes 1.0 and wraps.
        let next = interpolate_time(0.9, 0.1, true, 0.1);
        assert!((next - 0.95).abs() < 1e-6);
        let wrapped = interpolate_time(0.99, 0.1, true, 0.1);
        assert!(wrapped < 0.1, "{wrapped}");
    }

    #[test]
    fn backward_prefers_decreasing_time() {
        let next = interpolate_time(0.1, 0.9, false, 0.1);
        assert!((next - 0.05).abs() < 1e-6, "{next}");
        // Going backward from near zero wraps below it.
        let wrapped = interpolate_time(0.01, 0.9, false, 0.1);
        assert!(wrapped > 0.9, "{wrapped}");
    }

    #[test]
    fn times_are_approximately_equal_within_float_noise() {
        assert!(is_approx(0.5, 0.500_001));
        assert!(!is_approx(0.5, 0.51));
    }
}
