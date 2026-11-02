//! What pressing jump does, for the joypad icon (#2960). The icon and the press must reach the
//! same verdict: where they disagree for a single physics tick, the icon flashes Glider.

use super::body::{
    COYOTE_WINDOW, GLIDE_CLOSING, GLIDE_COOLDOWN, GLIDE_GLIDING, GLIDE_MIN_GROUND_DISTANCE,
    GLIDE_OPENING, JUMP_ACTION_GLIDE_TOGGLE, JUMP_ACTION_JUMP, JUMP_ACTION_NONE, JUMP_COOLDOWN,
    JUMP_TO_GLIDE_INTERVAL, MAX_AIR_JUMPS,
};

/// The coyote window: a buffered jump still runs the ground jump here, so a one-tick
/// `is_on_floor()` drop must not offer the glider. Only a walk-off opens it, never the way up (a
/// jump-pad launch stays a cancellable "ground").
pub fn in_coyote_window(supported: bool, fall_elapsed: f32, vertical_velocity: f32) -> bool {
    !supported && fall_elapsed <= COYOTE_WINDOW && vertical_velocity <= 0.0
}

/// Ground distance from a 20 m down ray. A miss means nothing solid is below, and the realm
/// floor (y = 0) is held by the clamp, not by a body, so a miss measures the height above that
/// floor instead of reading as unlimited altitude, which would pass every altitude gate.
pub fn resolve_ground_distance(hit_y: Option<f32>, from_y: f32) -> f32 {
    match hit_y {
        Some(hit_y) => from_y - hit_y,
        None => from_y.max(0.0),
    }
}

pub struct JumpActionInputs {
    pub jump_disabled: bool,
    pub all_input_disabled: bool,
    pub hard_landing: bool,
    /// That tick's `supported || in_coyote`: a one-tick `is_on_floor()` drop is still a ground
    /// jump.
    pub grounded: bool,
    pub glide_state: i32,
    pub jump_count: i32,
    pub double_jump_disabled: bool,
    pub glide_disabled: bool,
    pub ground_distance: f32,
    pub fall_elapsed: f32,
    pub since_last_jump: f32,
    pub since_glide_end: f32,
}

/// Mirrors the decision tree of the jump press in `Player::physics_process`.
pub fn resolve_jump_action(i: &JumpActionInputs) -> i32 {
    if i.jump_disabled || i.all_input_disabled || i.hard_landing {
        return JUMP_ACTION_NONE;
    }
    if i.grounded {
        return JUMP_ACTION_JUMP;
    }
    // Airborne. GLIDE_TOGGLE is reported while the glider is open even if the scene disables
    // gliding: the force-close transitions to CLOSING next tick, and NONE here would flicker the
    // icon in between.
    if i.glide_state == GLIDE_OPENING || i.glide_state == GLIDE_GLIDING {
        return JUMP_ACTION_GLIDE_TOGGLE;
    }
    if i.glide_state == GLIDE_CLOSING {
        return JUMP_ACTION_NONE;
    }
    // Closed: an air-jump takes priority over glide-open.
    if i.jump_count <= MAX_AIR_JUMPS
        && !i.double_jump_disabled
        && i.since_last_jump >= JUMP_COOLDOWN
    {
        return JUMP_ACTION_JUMP;
    }
    if !i.glide_disabled
        && i.ground_distance > GLIDE_MIN_GROUND_DISTANCE
        // Not a single skipped support tick (step-up, edge, margin cloud).
        && i.fall_elapsed > COYOTE_WINDOW
        && i.since_last_jump >= JUMP_TO_GLIDE_INTERVAL
        && i.since_glide_end >= GLIDE_COOLDOWN
    {
        return JUMP_ACTION_GLIDE_TOGGLE;
    }
    JUMP_ACTION_NONE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::body::GLIDE_CLOSED;

    // Every jump press this fall has been spent, so the glide gates are the ones under test.
    const SPENT_AIR_JUMPS: i32 = 2;

    /// Nothing disabled, no cooldown pending, falling for a while (past the coyote window).
    fn action(grounded: bool, glide_state: i32, jump_count: i32, ground_distance: f32) -> i32 {
        action_after(grounded, glide_state, jump_count, ground_distance, 10.0)
    }

    fn action_after(
        grounded: bool,
        glide_state: i32,
        jump_count: i32,
        ground_distance: f32,
        fall_elapsed: f32,
    ) -> i32 {
        resolve_jump_action(&JumpActionInputs {
            jump_disabled: false,
            all_input_disabled: false,
            hard_landing: false,
            grounded,
            glide_state,
            jump_count,
            double_jump_disabled: false,
            glide_disabled: false,
            ground_distance,
            fall_elapsed,
            since_last_jump: 10.0,
            since_glide_end: 10.0,
        })
    }

    #[test]
    fn coyote_window_follows_support_and_direction() {
        assert!(
            !in_coyote_window(true, 0.0, -0.1),
            "supported is never coyote"
        );
        assert!(
            in_coyote_window(false, 0.033, -0.1),
            "a 1-tick drop is coyote"
        );
        assert!(in_coyote_window(false, 0.149, -0.5));
        assert!(!in_coyote_window(false, 0.151, -0.5), "past the window");
        assert!(!in_coyote_window(false, 0.033, 3.0), "rising is not coyote");
    }

    #[test]
    fn a_ray_miss_is_not_altitude() {
        assert_eq!(resolve_ground_distance(None, 0.12), 0.12);
        assert_eq!(resolve_ground_distance(None, 10.1), 10.1);
        assert_eq!(resolve_ground_distance(None, -0.5), 0.0);
        assert!((resolve_ground_distance(Some(-0.2), 0.1) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn grounded_wins_over_any_ground_reading() {
        assert_eq!(action(true, GLIDE_CLOSED, 0, 2.0), JUMP_ACTION_JUMP);
        assert_eq!(
            action(true, GLIDE_CLOSED, 0, f32::INFINITY),
            JUMP_ACTION_JUMP
        );
    }

    #[test]
    fn airborne_with_altitude_offers_the_glider() {
        assert_eq!(
            action(false, GLIDE_CLOSED, SPENT_AIR_JUMPS, 2.0),
            JUMP_ACTION_GLIDE_TOGGLE
        );
    }

    #[test]
    fn a_step_up_tick_offers_nothing() {
        // The riser slide spends a frame unsupported with vy > 0: the coyote window is closed
        // and the ground below the step is past the altitude gate.
        assert_eq!(
            action_after(false, GLIDE_CLOSED, SPENT_AIR_JUMPS, 2.0, 0.0167),
            JUMP_ACTION_NONE
        );
        assert_eq!(
            action_after(false, GLIDE_CLOSED, SPENT_AIR_JUMPS, 2.0, 0.167),
            JUMP_ACTION_GLIDE_TOGGLE
        );
    }

    #[test]
    fn an_air_jump_owns_the_press_while_it_is_available() {
        assert_eq!(action(false, GLIDE_CLOSED, 1, 2.0), JUMP_ACTION_JUMP);
        // A step off a ledge still has its air jump.
        assert_eq!(action(false, GLIDE_CLOSED, 0, 2.0), JUMP_ACTION_JUMP);
    }

    #[test]
    fn too_low_never_offers_the_glider() {
        let low = GLIDE_MIN_GROUND_DISTANCE * 0.5;
        assert_eq!(
            action(false, GLIDE_CLOSED, SPENT_AIR_JUMPS, low),
            JUMP_ACTION_NONE
        );
    }

    #[test]
    fn an_open_glider_toggles_closed_and_a_closing_one_offers_nothing() {
        assert_eq!(
            action(false, GLIDE_GLIDING, 1, 0.2),
            JUMP_ACTION_GLIDE_TOGGLE
        );
        assert_eq!(action(false, GLIDE_CLOSING, 1, 2.0), JUMP_ACTION_NONE);
    }

    #[test]
    fn disabled_input_and_hard_landing_offer_nothing() {
        let mut inputs = JumpActionInputs {
            jump_disabled: true,
            all_input_disabled: false,
            hard_landing: false,
            grounded: true,
            glide_state: GLIDE_CLOSED,
            jump_count: 0,
            double_jump_disabled: false,
            glide_disabled: false,
            ground_distance: 0.0,
            fall_elapsed: 0.0,
            since_last_jump: 10.0,
            since_glide_end: 10.0,
        };
        assert_eq!(resolve_jump_action(&inputs), JUMP_ACTION_NONE);
        inputs.jump_disabled = false;
        inputs.hard_landing = true;
        assert_eq!(resolve_jump_action(&inputs), JUMP_ACTION_NONE);
    }
}
