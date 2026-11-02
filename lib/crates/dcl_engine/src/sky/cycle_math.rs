//! The pure day/night math of the sky (issue #2516): the moon crossfade, the sun gate and energy,
//! and the minimum sun elevation. Times are normalized (0 is midnight, 0.5 midday).
//!
//! The baked `light_cycle` animation drives a single directional light as the sun by day and the
//! moon by night: its night arc (about 21:00 to 05:00) keeps 15 to 32 degrees of elevation while the
//! visual sun is below the horizon. The windows below are the crossfades between the two. Light
//! energy stays elevation-driven, which is what masks the animation's dusk and dawn jumps: the
//! light is off mid-jump.

use godot::builtin::Vector3;

/// About 21:04: the moon rises right after the whip.
pub const MOON_FADE_IN_START: f32 = 0.878;
/// About 21:43: the moon is fully in charge.
pub const MOON_FADE_IN_END: f32 = 0.905;
/// The fade-out is time-driven, not elevation: the baked moonset drops from +28 to 0 degrees in a
/// single keyframe interval, so an elevation gate would cut the light abruptly. It ends where the
/// arc crosses the horizon (about 04:21) and starts early enough to match the fade-in.
pub const MOON_FADE_OUT_START: f32 = 0.12;
pub const MOON_FADE_OUT_END: f32 = 0.181;
/// The baked animation teleports the light from its sunset parking spot to the night arc between
/// about 20:30 and 21:00: the azimuth whips 135 to -15 to 0 degrees while the elevation swings
/// -10, +48, +74, +15. Elevation gating cannot mask it, so all directional light is forced off
/// during the segment (a short ambient-only blue hour) and the moon fades in on the stable arc after.
pub const LIGHTS_OUT_START: f32 = 0.85;
pub const LIGHTS_OUT_END: f32 = 0.878;
/// The sun may only turn on while the animation is on the day arc: it rises at about 05:38 to
/// 06:21 and falls just after sunset (18:50 to 19:12), and stays 0 all night so it cannot pop back
/// when the night arc rises (the dusk flash).
pub const SUN_GATE_START: f32 = 0.235;
pub const SUN_GATE_END: f32 = 0.265;
pub const SUN_GATE_DOWN_START: f32 = 0.785;
pub const SUN_GATE_DOWN_END: f32 = 0.8;
/// Sun energy by elevation: zero at or below -0.05, full at or above 0.3.
pub const SUN_ENERGY_FADE_LOW: f32 = -0.05;
pub const SUN_ENERGY_FADE_HIGH: f32 = 0.3;

/// Hermite interpolation between two edges, clamped, as the engine's `smoothstep`.
pub fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    if (to - from).abs() < f32::EPSILON {
        return if x < from { 0.0 } else { 1.0 };
    }
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 0 during the day, 1 when the animation is on its night (moon) arc.
pub fn moon_factor(skybox_time: f32) -> f32 {
    let evening = smoothstep(MOON_FADE_IN_START, MOON_FADE_IN_END, skybox_time);
    let morning = 1.0 - smoothstep(MOON_FADE_OUT_START, MOON_FADE_OUT_END, skybox_time);
    (evening + morning).clamp(0.0, 1.0)
}

/// True during the baked dusk whip: no directional light may be on.
pub fn is_lights_out(skybox_time: f32) -> bool {
    (LIGHTS_OUT_START..LIGHTS_OUT_END).contains(&skybox_time)
}

/// 0 on the night arc, 1 on the day arc. Without it the sun would crossfade in during moonset
/// (about 04:00, elevation still 28 degrees) and flash warm light from the west at 4 am, and pop
/// back on at about 21:00 when the night arc rises after the whip.
pub fn sun_gate(skybox_time: f32) -> f32 {
    let morning = smoothstep(SUN_GATE_START, SUN_GATE_END, skybox_time);
    let evening = 1.0 - smoothstep(SUN_GATE_DOWN_START, SUN_GATE_DOWN_END, skybox_time);
    morning * evening
}

/// The sun light's energy factor from its elevation (the y of the direction to the sun): 0 at or
/// under the horizon, 1 at or above 0.3 (about 17.5 degrees).
pub fn sun_energy_factor(elevation: f32) -> f32 {
    smoothstep(SUN_ENERGY_FADE_LOW, SUN_ENERGY_FADE_HIGH, elevation)
}

/// Lifts `dir_to_light` (a unit direction to the sun or moon) so its elevation never drops below
/// `min_elevation` (radians), keeping the azimuth. Directions at or under the horizon come back
/// untouched: they produce no light anyway. It keeps a low-but-visible sun casting a readable
/// ground shadow (the morning and afternoon sun grazes the horizon).
pub fn clamp_direction_elevation(dir_to_light: Vector3, min_elevation: f32) -> Vector3 {
    let elevation = dir_to_light.y.clamp(-1.0, 1.0).asin();
    if elevation <= 0.0 || elevation >= min_elevation {
        return dir_to_light;
    }
    let horizontal_len = (dir_to_light.x * dir_to_light.x + dir_to_light.z * dir_to_light.z).sqrt();
    if horizontal_len < 0.0001 {
        return dir_to_light;
    }
    let scale = min_elevation.cos() / horizontal_len;
    Vector3::new(
        dir_to_light.x * scale,
        min_elevation.sin(),
        dir_to_light.z * scale,
    )
    .normalized()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f32, expected: f32, tolerance: f32, what: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{what}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn the_moon_owns_the_night_and_the_sun_the_day() {
        near(moon_factor(0.0), 1.0, 1e-4, "midnight");
        near(moon_factor(0.1), 1.0, 1e-4, "02:24");
        near(moon_factor(0.5), 0.0, 1e-4, "midday");
        near(moon_factor(0.875), 0.0, 1e-4, "the whip tail");
        near(moon_factor(0.878), 0.0, 1e-4, "fade-in starts");
        near(moon_factor(0.905), 1.0, 1e-4, "fully in");
        near(moon_factor(0.12), 1.0, 1e-4, "full before moonset");
        near(moon_factor(0.181), 0.0, 1e-4, "out at the horizon");
        near(moon_factor(0.22), 0.0, 1e-4, "05:17");
        near(moon_factor(0.8915), 0.5, 0.05, "dusk midpoint");
        near(moon_factor(0.1505), 0.5, 0.05, "moonset midpoint");
    }

    #[test]
    fn all_light_is_off_only_during_the_whip() {
        assert!(!is_lights_out(0.5));
        assert!(!is_lights_out(0.0));
        assert!(!is_lights_out(0.83));
        assert!(is_lights_out(0.85));
        assert!(is_lights_out(0.861));
        assert!(!is_lights_out(0.878));
    }

    #[test]
    fn the_sun_only_lives_on_the_day_arc() {
        near(sun_gate(0.0), 0.0, 1e-4, "midnight");
        near(sun_gate(0.17), 0.0, 1e-4, "no 4 am flash");
        near(sun_gate(0.235), 0.0, 1e-4, "gate starts");
        near(sun_gate(0.265), 1.0, 1e-4, "gate full");
        near(sun_gate(0.5), 1.0, 1e-4, "midday");
        near(sun_gate(0.785), 1.0, 1e-4, "until sunset is done");
        near(sun_gate(0.8), 0.0, 1e-4, "gated for the night");
        near(sun_gate(0.878), 0.0, 1e-4, "gated when the night arc rises");
        near(sun_gate(0.25), 0.5, 0.05, "sunrise midpoint");
        near(sun_gate(0.7925), 0.5, 0.05, "sunset midpoint");
    }

    #[test]
    fn sun_energy_follows_elevation() {
        near(sun_energy_factor(-1.0), 0.0, 1e-4, "deep night");
        near(sun_energy_factor(-0.05), 0.0, 1e-4, "low bound");
        near(
            sun_energy_factor(0.0),
            0.0554,
            0.001,
            "horizon is nearly off",
        );
        near(sun_energy_factor(0.3), 1.0, 1e-4, "high bound");
        near(sun_energy_factor(0.99), 1.0, 1e-4, "zenith");
    }

    #[test]
    fn a_low_sun_is_lifted_keeping_its_azimuth() {
        let min = 12.0_f32.to_radians();
        let low =
            Vector3::new(0.0, 2.0_f32.to_radians().sin(), 2.0_f32.to_radians().cos()).normalized();
        let clamped = clamp_direction_elevation(low, min);
        near(clamped.y, min.sin(), 1e-3, "lifted to the minimum");
        near(clamped.x, 0.0, 1e-4, "azimuth x");
        assert!(clamped.z > 0.0);
        near(clamped.length(), 1.0, 1e-4, "unit");
    }

    #[test]
    fn high_and_underground_directions_are_untouched() {
        let min = 12.0_f32.to_radians();
        let high = Vector3::new(0.3, 0.8, 0.5).normalized();
        assert_eq!(clamp_direction_elevation(high, min), high);
        let under = Vector3::new(0.0, -0.3, 0.9).normalized();
        assert_eq!(clamp_direction_elevation(under, min), under);
    }

    #[test]
    fn straight_up_does_not_produce_nan() {
        let zenith = Vector3::new(0.0, 1.0, 0.0);
        assert!(!clamp_direction_elevation(zenith, 80.0_f32.to_radians())
            .y
            .is_nan());
    }
}
