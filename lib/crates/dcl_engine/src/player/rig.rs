//! Camera rig numbers and the pure math around them, free of any node so they can be unit
//! tested: the rig constants, the pinch recogniser and the grounded-flag debounce.

/// Solid world geometry lives on CL_PHYSICS (layer 2): the mask the player body collides with.
/// The camera's spring arm shortens against exactly the walls the player cannot walk through.
pub const CAMERA_COLLISION_MASK: u32 = 2;

/// Centered third-person framing (the avatar is centered on screen, no over-shoulder offset):
/// x is the lateral camera offset, z the back distance.
pub const THIRD_PERSON_CAMERA_X: f32 = 0.0;
pub const THIRD_PERSON_CAMERA_Z: f32 = 3.0;
/// First person sits just in front of the pivot (inside the head).
pub const FIRST_PERSON_SPRING_LENGTH: f32 = -0.2;

/// Radius of the sphere swept to the real camera position; doubles as the standoff from walls.
/// Not larger than ~0.4: bigger spheres touch both frames of ~1m doorways and glue the camera to
/// the avatar's head in every doorway.
pub const CLAMP_SPHERE_RADIUS: f32 = 0.4;
/// Extra pull-in beyond the sphere contact point.
pub const CLAMP_EXTRA_MARGIN: f32 = 0.02;
/// The camera's own collision bubble, depenetrated along contact normals each frame. At or below
/// the player capsule radius so hugging a wall does not constantly fight the arm's collapsed
/// position, and well above the near plane (0.05).
pub const CLAMP_CONTACT_RADIUS: f32 = 0.2;
/// Perpendicular clearance the camera keeps from wall faces: the near plane is 0.05, and less
/// than this pokes into the wall even when the clamp "worked", worst on angled/curved trimesh.
pub const CLAMP_NEAR_CLEARANCE: f32 = 0.08;
/// Extension recovery speed (m/s). Shortening is instant (never clip), extending is smoothed so
/// geometry does not pop through on the way out.
pub const CLAMP_EXTEND_SPEED: f32 = 8.0;
/// Some scene ground meshes have no usable collider, so the camera is kept at least this far
/// above the player's own ground contact.
pub const FLOOR_CLEARANCE: f32 = 0.15;

/// Spring-arm length and camera X offset for a camera mode. The X offset is applied on the camera
/// below the spring arm, never on the arm's pivot: the pivot stays centered on the capsule so the
/// collision cast starts from a protected point. The arm length is the pure back distance; the
/// lateral part is the offset's.
pub fn rig_targets(third_person: bool) -> (f32, f32) {
    if third_person {
        (THIRD_PERSON_CAMERA_Z, THIRD_PERSON_CAMERA_X)
    } else {
        (FIRST_PERSON_SPRING_LENGTH, 0.0)
    }
}

/// Godot's `normalized()` returns the zero vector for a zero vector; gdext's panics.
pub fn normalized_or_zero(v: godot::prelude::Vector3) -> godot::prelude::Vector3 {
    if v.length_squared() > 0.0 {
        v.normalized()
    } else {
        godot::prelude::Vector3::ZERO
    }
}

// --- Pinch-to-zoom ---------------------------------------------------------------------------

/// Finger-spread change (px) from where the candidate pair formed that commits the gesture:
/// small enough that a thumb-anchored or angled pinch still trips it.
pub const COMMIT_SPREAD: f32 = 16.0;

/// Accumulated spread change (px) in one direction that swaps 1p<->3p: two fixed camera
/// positions, no continuous zoom curve.
pub const MODE_TOGGLE_SPREAD: f32 = 80.0;

/// The pinch commits once the spread has changed past `COMMIT_SPREAD` from where the pair formed,
/// in either direction.
pub fn should_commit(start_spread: f32, current_spread: f32) -> bool {
    (current_spread - start_spread).abs() >= COMMIT_SPREAD
}

/// Net spread direction once the accumulated delta passes `MODE_TOGGLE_SPREAD`: 1 spreading
/// (fingers opening), -1 closing, 0 below the threshold.
pub fn toggle_direction(accumulated_delta: f32) -> i32 {
    if accumulated_delta >= MODE_TOGGLE_SPREAD {
        1
    } else if accumulated_delta <= -MODE_TOGGLE_SPREAD {
        -1
    } else {
        0
    }
}

// --- Grounded flag ---------------------------------------------------------------------------

/// Debounce on the animation grounded flag: at mobile physics rates a one-tick `is_on_floor()`
/// flicker replayed the landing pose.
pub const GROUNDED_GRACE_WINDOW: f32 = 0.15;
pub const JUMP_COOLDOWN: f32 = 0.3;

/// Grace is suppressed while jump is held and during the jump cooldown: the cooldown is what
/// keeps tap-jumps (released before the next tick reads `jump_held == false`) from lingering
/// grounded.
pub fn resolve_is_grounded(
    on_floor: bool,
    fall_elapsed: f32,
    jump_held: bool,
    since_last_jump: f32,
) -> bool {
    on_floor
        || (fall_elapsed < GROUNDED_GRACE_WINDOW && !jump_held && since_last_jump >= JUMP_COOLDOWN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_mask_is_the_physics_layer() {
        assert_eq!(CAMERA_COLLISION_MASK, 2);
    }

    #[test]
    fn rig_targets_use_the_back_distance_and_no_lateral_offset() {
        assert_eq!(rig_targets(true), (3.0, 0.0));
        assert_eq!(rig_targets(false), (-0.2, 0.0));
    }

    #[test]
    fn a_pinch_commits_past_the_spread_threshold_in_either_direction() {
        assert!(!should_commit(100.0, 110.0));
        assert!(should_commit(100.0, 116.0));
        assert!(should_commit(100.0, 80.0));
    }

    #[test]
    fn the_toggle_needs_a_deliberate_spread() {
        assert_eq!(toggle_direction(79.9), 0);
        assert_eq!(toggle_direction(80.0), 1);
        assert_eq!(toggle_direction(-80.0), -1);
        assert_eq!(toggle_direction(-10.0), 0);
    }

    #[test]
    fn a_floor_flicker_inside_the_grace_window_keeps_the_avatar_grounded() {
        // Walked off a ledge a tick ago, not jumping.
        assert!(resolve_is_grounded(false, 0.05, false, 5.0));
        // Past the grace window.
        assert!(!resolve_is_grounded(false, 0.2, false, 5.0));
        // Jump held: no grace.
        assert!(!resolve_is_grounded(false, 0.05, true, 5.0));
        // Right after a jump (tap released): the cooldown stops the grace.
        assert!(!resolve_is_grounded(false, 0.05, false, 0.1));
        assert!(resolve_is_grounded(true, 9.0, true, 0.0));
    }
}
