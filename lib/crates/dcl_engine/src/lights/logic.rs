//! The numbers behind LightSource: SDK units to Godot's, activation range, culling, fading and
//! the global budget. Pure, so the rules are tested without lights.

use godot::builtin::Vector3;

/// SDK intensity is candela (proto default 16000). The reference client passes it through to
/// Unity's physical-unit intensity, so candela to Godot energy divides by the proto default:
/// 16000 is 1.0. Validated against mainnet scenes (rituals, doriangray, RAGE). The sdk7
/// dynamic-lights test scene authors ~100, which renders nearly invisible: its values are 100x
/// below real-world authoring, not a conversion bug.
pub const INTENSITY_DIVISOR: f32 = 16000.0;
/// Auto activation range: the distance at which the light's intensity is still noticeable.
pub const AUTO_RANGE_DIVISOR: f32 = 160.0;
/// Light energy fades in or out over this many seconds when the render state changes (the
/// reference client's FadeDuration).
pub const FADE_DURATION: f32 = 0.25;
/// Lights behind the camera do not render, unless their influence reaches the camera (you stand
/// in their glow) or they are very close (they wrap into view when the camera turns).
pub const CAMERA_CULL_CLOSE_RADIUS: f32 = 6.0;
/// `dot(camera_forward, to_light)` threshold: 0.2 is about 78 degrees, the FOV plus a margin.
pub const CAMERA_CULL_MIN_DOT: f32 = 0.2;

pub fn convert_intensity(intensity: f32) -> f32 {
    intensity / INTENSITY_DIVISOR
}

/// Where a light activates: its range, widened for bright lights, capped. Without auto range it
/// is the authored range.
pub fn activation_range(light_range: f32, intensity: f32, auto_range: bool, cap: f32) -> f32 {
    if !auto_range {
        return light_range;
    }
    let intensity_range = (intensity.max(0.0) / AUTO_RANGE_DIVISOR).sqrt();
    light_range.max(intensity_range).min(cap)
}

/// How the avatar relates to the light: -1 no reference or no range, 0 outside the activation
/// range, 1 inside.
pub fn avatar_range_state(
    has_reference: bool,
    activation_range: f32,
    distance_to_reference: f32,
) -> i32 {
    if !has_reference || activation_range <= 0.0 {
        return -1;
    }
    if distance_to_reference <= activation_range {
        1
    } else {
        0
    }
}

/// Whether the camera can see the light, or its influence reaches it: turning those off would
/// visibly kill lit surfaces on screen.
pub fn in_camera_view(
    has_camera: bool,
    camera_position: Vector3,
    camera_forward: Vector3,
    light_position: Vector3,
    activation_range: f32,
) -> bool {
    if !has_camera {
        return true;
    }
    let to_light = light_position - camera_position;
    let dist = to_light.length();
    let reach = CAMERA_CULL_CLOSE_RADIUS.max(activation_range);
    if dist <= reach || dist <= 0.0 {
        return true;
    }
    camera_forward.dot(to_light / dist) >= CAMERA_CULL_MIN_DOT
}

/// One fade step: the energy scale moves toward 1 (rendering) or 0, and the fade is active until
/// it gets there.
pub fn fade_step(scale: f32, rendering: bool, delta: f32) -> (f32, bool) {
    let step = delta / FADE_DURATION;
    if rendering {
        let scale = (scale + step).min(1.0);
        (scale, scale < 1.0)
    } else {
        let scale = (scale - step).max(0.0);
        (scale, scale > 0.0)
    }
}

/// The budget: of `(id, distance)` candidates, the `max` nearest get `Some(rank)` (1-based) and
/// are enabled; the rest are ranked but not enabled. Returns `(id, rank, enabled)` nearest first.
pub fn rank_by_distance<T: Copy>(candidates: &[(T, f32)], max: usize) -> Vec<(T, usize, bool)> {
    let mut sorted: Vec<(T, f32)> = candidates.to_vec();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
    sorted
        .into_iter()
        .enumerate()
        .map(|(i, (id, _))| (id, i + 1, i < max))
        .collect()
}

/// A spot light's Godot angle: the SDK gives the full cone, Godot the radius (max 89.99), and the
/// inner angle becomes the attenuation exponent since Godot has no inner plateau (a higher value
/// holds brightness longer toward the edge, like a wide inner cone). `(spot_angle, attenuation)`.
pub fn spot_angles(inner_angle: f32, outer_angle: f32) -> (f32, f32) {
    let outer = (outer_angle * 0.5).min(89.99);
    let inner = (inner_angle * 0.5).min(outer);
    (outer, outer / (outer - inner).max(0.01))
}

/// Gizmo cone `(length, radius)`: the length is the authored range, the radius follows the angle
/// but is capped, since a 179 degree cone would draw kilometer-long lines.
pub fn debug_cone_dimensions(light_range: f32, angle_degrees: f32) -> (f32, f32) {
    const MAX_RADIUS: f32 = 30.0;
    const MIN_LENGTH: f32 = 0.18;
    let half = (angle_degrees.clamp(0.1, 175.0) * 0.5).to_radians();
    let length = light_range.max(MIN_LENGTH);
    (length, (half.tan() * length).min(MAX_RADIUS))
}

/// A float to two decimals without trailing zeros, for the gizmo labels.
pub fn fmtf(v: f32) -> String {
    format!("{}", (v * 100.0).round() / 100.0)
}

/// The last `max` characters of a long path, behind an ellipsis.
pub fn short_path(path: &str, max: usize) -> String {
    let count = path.chars().count();
    if count <= max {
        return path.to_string();
    }
    let tail: String = path.chars().skip(count - max).collect();
    format!("...{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proto_default_intensity_is_one() {
        assert_eq!(convert_intensity(16000.0), 1.0);
        assert_eq!(convert_intensity(0.0), 0.0);
    }

    #[test]
    fn auto_range_widens_bright_lights_up_to_the_cap() {
        // 16000 / 160 = 100 -> sqrt = 10 m, beyond a 5 m authored range.
        assert_eq!(activation_range(5.0, 16000.0, true, 40.0), 10.0);
        assert_eq!(activation_range(5.0, 16000.0, true, 8.0), 8.0);
        assert_eq!(activation_range(50.0, 16000.0, true, 40.0), 40.0);
        assert_eq!(activation_range(5.0, 16000.0, false, 40.0), 5.0);
        assert_eq!(activation_range(5.0, -1.0, true, 40.0), 5.0);
    }

    #[test]
    fn range_state_needs_a_reference_and_a_range() {
        assert_eq!(avatar_range_state(false, 10.0, 1.0), -1);
        assert_eq!(avatar_range_state(true, 0.0, 1.0), -1);
        assert_eq!(avatar_range_state(true, 10.0, 10.0), 1);
        assert_eq!(avatar_range_state(true, 10.0, 10.1), 0);
    }

    #[test]
    fn lights_behind_the_camera_cull_unless_close() {
        let cam = Vector3::ZERO;
        let fwd = Vector3::new(0.0, 0.0, -1.0);
        assert!(in_camera_view(
            false,
            cam,
            fwd,
            Vector3::new(0.0, 0.0, 100.0),
            1.0
        ));
        assert!(in_camera_view(
            true,
            cam,
            fwd,
            Vector3::new(0.0, 0.0, -50.0),
            1.0
        ));
        assert!(!in_camera_view(
            true,
            cam,
            fwd,
            Vector3::new(0.0, 0.0, 50.0),
            1.0
        ));
        // Close, or reaching the camera: kept.
        assert!(in_camera_view(
            true,
            cam,
            fwd,
            Vector3::new(0.0, 0.0, 5.0),
            1.0
        ));
        assert!(in_camera_view(
            true,
            cam,
            fwd,
            Vector3::new(0.0, 0.0, 30.0),
            40.0
        ));
    }

    #[test]
    fn fading_reaches_each_end_and_stops() {
        let (scale, active) = fade_step(0.0, true, 0.1);
        assert!((scale - 0.4).abs() < 1e-6 && active);
        assert_eq!(fade_step(0.9, true, 0.1), (1.0, false));
        let (scale, active) = fade_step(1.0, false, 0.1);
        assert!((scale - 0.6).abs() < 1e-6 && active);
        assert_eq!(fade_step(0.1, false, 0.1), (0.0, false));
    }

    #[test]
    fn the_budget_enables_the_nearest() {
        let ranked = rank_by_distance(&[(1, 30.0), (2, 5.0), (3, 10.0)], 2);
        assert_eq!(ranked, vec![(2, 1, true), (3, 2, true), (1, 3, false)]);
        assert!(rank_by_distance::<i32>(&[], 4).is_empty());
        assert!(rank_by_distance(&[(1, 1.0)], 0)
            .iter()
            .all(|(_, _, on)| !on));
    }

    #[test]
    fn spot_angles_halve_the_cone_and_cap_it() {
        assert_eq!(spot_angles(0.0, 60.0), (30.0, 1.0));
        let (outer, attenuation) = spot_angles(30.0, 60.0);
        assert_eq!(outer, 30.0);
        assert!((attenuation - 2.0).abs() < 1e-6);
        assert_eq!(spot_angles(0.0, 360.0).0, 89.99);
        // An inner angle wider than the outer is clamped, not infinite.
        assert!(spot_angles(120.0, 60.0).1.is_finite());
    }

    #[test]
    fn gizmo_cones_cap_the_radius_not_the_length() {
        let (length, radius) = debug_cone_dimensions(100.0, 170.0);
        assert_eq!((length, radius), (100.0, 30.0));
        let (length, radius) = debug_cone_dimensions(10.0, 90.0);
        assert_eq!(length, 10.0);
        assert!((radius - 10.0).abs() < 1e-4);
        assert_eq!(debug_cone_dimensions(0.0, 30.0).0, 0.18);
    }

    #[test]
    fn labels_format_compactly() {
        assert_eq!(fmtf(1.0), "1");
        assert_eq!(fmtf(1.2345), "1.23");
        assert_eq!(short_path("a/b.png", 34), "a/b.png");
        assert_eq!(short_path("0123456789", 4), "...6789");
    }
}
