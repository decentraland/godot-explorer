//! Where a pinch may form (#3317): a box around the avatar's on-screen position, minus the side
//! bands where the joystick and its second-finger look live. A free finger anywhere else plus a
//! thumb in the idle joystick rect is not a pinch, so a second finger while walking never zooms.

use godot::prelude::Vector2;

/// Half side of the pinch zone, as a fraction of the viewport per axis: the zone is 30% of the
/// screen wide and tall, centred on the avatar (40% swallowed fingers landing near the middle).
pub const ZONE_HALF_FRACTION: f32 = 0.15;

/// Band excluded from anchoring a pinch on each side (fraction of the screen width): the
/// joystick's active area is the left 40% of the screen, and the avatar's projection can drift
/// sideways.
pub const SIDE_DEADZONE_FRACTION: f32 = 0.12;

/// Half side of the pinch zone in pixels for a viewport.
pub fn zone_half_size(viewport_size: Vector2) -> Vector2 {
    viewport_size * ZONE_HALF_FRACTION
}

/// Inside the zone, border included (excluding it would make the edge flicker).
pub fn is_inside_zone(point: Vector2, center: Vector2, half_size: Vector2) -> bool {
    (point.x - center.x).abs() <= half_size.x && (point.y - center.y).abs() <= half_size.y
}

pub fn is_in_side_deadzone(point: Vector2, viewport_size: Vector2) -> bool {
    let margin = viewport_size.x * SIDE_DEADZONE_FRACTION;
    point.x <= margin || point.x >= viewport_size.x - margin
}

/// The finger that anchors a pinch: inside the zone and clear of the side bands.
pub fn can_anchor_pinch(
    point: Vector2,
    center: Vector2,
    half_size: Vector2,
    viewport_size: Vector2,
) -> bool {
    is_inside_zone(point, center, half_size) && !is_in_side_deadzone(point, viewport_size)
}

/// A pair forms only on the avatar: one finger anchors it (the other may be a thumb resting
/// outside, on the idle joystick), or the pair is compact and its midpoint anchors it. The
/// midpoint case is the horizontal pinch, whose fingers straddle the zone.
pub fn pair_has_zone_member(
    a: Vector2,
    b: Vector2,
    center: Vector2,
    half_size: Vector2,
    viewport_size: Vector2,
) -> bool {
    if can_anchor_pinch(a, center, half_size, viewport_size)
        || can_anchor_pinch(b, center, half_size, viewport_size)
    {
        return true;
    }
    // A spread across most of the screen is walk+look, not a pinch on the avatar.
    if a.distance_to(b) > half_size.x.max(half_size.y) * 2.0 {
        return false;
    }
    can_anchor_pinch((a + b) * 0.5, center, half_size, viewport_size)
}

/// The further both fingers are from the zone centre, the worse the pair.
pub fn pair_score(a: Vector2, b: Vector2, center: Vector2) -> f32 {
    a.distance_to(center) + b.distance_to(center)
}

/// The qualifying pair closest to the avatar, so an idle-joystick thumb plus two free fingers
/// resolves to the two on the avatar. Ties keep the earlier pair in `touches` order.
pub fn closest_pair(
    touches: &[(i32, Vector2)],
    center: Vector2,
    half_size: Vector2,
    viewport_size: Vector2,
) -> Option<[i32; 2]> {
    let mut best = None;
    let mut best_score = f32::INFINITY;
    for (i, &(index_a, a)) in touches.iter().enumerate() {
        for &(index_b, b) in &touches[i + 1..] {
            if !pair_has_zone_member(a, b, center, half_size, viewport_size) {
                continue;
            }
            let score = pair_score(a, b, center);
            if score < best_score {
                best = Some([index_a, index_b]);
                best_score = score;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use godot::builtin::math::ApproxEq;

    use super::*;

    /// Portrait phone the numbers are checked against (Samsung A54).
    const VIEWPORT: Vector2 = Vector2::new(1080.0, 2400.0);
    const CENTER: Vector2 = Vector2::new(540.0, 1200.0);

    #[test]
    fn the_zone_is_thirty_percent_of_the_screen_per_axis() {
        let half = zone_half_size(VIEWPORT);
        assert!(half.approx_eq(&Vector2::new(162.0, 360.0)));
        assert!((half * 2.0).approx_eq(&(VIEWPORT * 0.3)));
    }

    #[test]
    fn the_zone_border_is_inclusive() {
        let half = Vector2::new(162.0, 360.0);
        assert!(is_inside_zone(CENTER, CENTER, half));
        assert!(is_inside_zone(
            CENTER + Vector2::new(100.0, -200.0),
            CENTER,
            half
        ));
        assert!(!is_inside_zone(
            CENTER + Vector2::new(half.x + 0.5, 0.0),
            CENTER,
            half
        ));
        assert!(!is_inside_zone(
            CENTER + Vector2::new(0.0, -half.y - 0.5),
            CENTER,
            half
        ));
        assert!(is_inside_zone(
            CENTER + Vector2::new(half.x, -half.y),
            CENTER,
            half
        ));
    }

    #[test]
    fn one_finger_on_the_avatar_is_enough() {
        let half = Vector2::new(162.0, 360.0);
        assert!(pair_has_zone_member(
            CENTER,
            CENTER + Vector2::new(100.0, 0.0),
            CENTER,
            half,
            VIEWPORT
        ));
        assert!(pair_has_zone_member(
            CENTER + Vector2::new(0.0, 100.0),
            Vector2::new(140.0, 2200.0),
            CENTER,
            half,
            VIEWPORT
        ));
        assert!(!pair_has_zone_member(
            Vector2::new(140.0, 2300.0),
            Vector2::new(1000.0, 120.0),
            CENTER,
            half,
            VIEWPORT
        ));
    }

    #[test]
    fn a_horizontal_pinch_counts_by_its_midpoint() {
        let half = zone_half_size(VIEWPORT);
        let left = Vector2::new(250.0, 1200.0);
        let right = Vector2::new(830.0, 1200.0);
        assert!(!is_inside_zone(left, CENTER, half) && !is_inside_zone(right, CENTER, half));
        assert!(pair_has_zone_member(left, right, CENTER, half, VIEWPORT));
        // A pair spanning most of the screen is walk+look.
        assert!(!pair_has_zone_member(
            Vector2::new(120.0, 2200.0),
            Vector2::new(1000.0, 200.0),
            CENTER,
            half,
            VIEWPORT
        ));
    }

    #[test]
    fn with_three_fingers_the_pair_on_the_avatar_wins() {
        let half = zone_half_size(VIEWPORT);
        let thumb = Vector2::new(150.0, 2050.0); // resting in the joystick rect
        let on_avatar = Vector2::new(520.0, 1180.0);
        let also_on_avatar = Vector2::new(380.0, 900.0);
        assert!(
            pair_score(on_avatar, also_on_avatar, CENTER) < pair_score(thumb, on_avatar, CENTER)
        );
        let touches = [(0, thumb), (1, on_avatar), (2, also_on_avatar)];
        assert_eq!(closest_pair(&touches, CENTER, half, VIEWPORT), Some([1, 2]));
    }

    #[test]
    fn the_side_bands_never_anchor_a_pinch() {
        let margin = VIEWPORT.x * SIDE_DEADZONE_FRACTION;
        let center = Vector2::new(100.0, 1200.0); // avatar drifted into the left band
        let half = zone_half_size(VIEWPORT);
        assert!(is_in_side_deadzone(
            Vector2::new(margin - 1.0, 1200.0),
            VIEWPORT
        ));
        assert!(is_in_side_deadzone(
            Vector2::new(VIEWPORT.x - margin + 1.0, 1200.0),
            VIEWPORT
        ));
        assert!(!is_in_side_deadzone(
            Vector2::new(margin + 1.0, 1200.0),
            VIEWPORT
        ));
        assert!(is_inside_zone(center, center, half));
        assert!(!can_anchor_pinch(center, center, half, VIEWPORT));
        assert!(!pair_has_zone_member(
            center,
            Vector2::new(900.0, 1200.0),
            center,
            half,
            VIEWPORT
        ));
    }

    /// #3317: a joystick thumb plus a free finger far from the avatar is no pinch; the
    /// thumb-anchored pinch with the second finger on the avatar still is.
    #[test]
    fn a_joystick_thumb_and_a_corner_finger_are_not_a_pinch() {
        let half = zone_half_size(VIEWPORT);
        let joystick_thumb = Vector2::new(140.0, 2100.0);
        let corner_finger = Vector2::new(1020.0, 180.0);
        assert!(!pair_has_zone_member(
            joystick_thumb,
            corner_finger,
            CENTER,
            half,
            VIEWPORT
        ));
        let touches = [(0, joystick_thumb), (1, corner_finger)];
        assert_eq!(closest_pair(&touches, CENTER, half, VIEWPORT), None);
        let pinch_finger = CENTER + Vector2::new(80.0, 40.0);
        assert!(pair_has_zone_member(
            joystick_thumb,
            pinch_finger,
            CENTER,
            half,
            VIEWPORT
        ));
    }
}
