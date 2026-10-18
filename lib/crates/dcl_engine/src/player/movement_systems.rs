//! Unity's movement systems on top of the base locomotion (#2852, #2854): slope speed, steep-slope
//! slide, edge slip, wall slide, the hard-landing fall height and moving-platform follow.

use godot::builtin::math::ApproxEq;
use godot::classes::{CollisionObject3D, PhysicsRayQueryParameters3D, ShapeCast3D, SphereShape3D};
use godot::prelude::*;

use super::body::{Player, CAPSULE_CENTER_Y, CAPSULE_RADIUS, WALKABLE_NORMAL_Y};

// #2852 M2: Unity SlopeVelocityModifier curve — linear keys (-55deg, 1.35) -> (0, 1) ->
// (55deg, 0.65), clamped beyond. Positive angle = uphill.
const SLOPE_MOD_MAX_DEG: f32 = 55.0;
const SLOPE_MOD_DOWNHILL: f32 = 1.35;
const SLOPE_MOD_UPHILL: f32 = 0.65;
// #2852 M4: ground contact further than this from the capsule axis tilts gravity toward it
// (Unity NoSlipDistance); ground straight below within EdgeSlipSafeDistance cancels it.
const EDGE_NO_SLIP_DIST: f32 = 0.1;
const EDGE_SLIP_SAFE_DISTANCE: f32 = 0.4;
// #2852 M5: head-on wall contact multiplies speed toward this (Unity
// WallSlideMaxMoveSpeedMultiplier = 0), lerped by |facing · wall normal|.
const WALL_SLIDE_MIN_MULT: f32 = 0.0;
/// #2852 M6: hard landing = fall height above this (Unity JumpHeightStun) with the scene's stun
/// duration (Unity LongFallStunTime, 0 disables the stun).
pub(super) const HARD_LANDING_FALL_HEIGHT: f32 = 8.0;
/// #2855: long-fall animation threshold (Unity AnimationLongFallSpeed = -8).
pub(super) const LONG_FALL_ANIM_SPEED: f32 = 8.0;
/// #1553: fall animation threshold (Unity AnimationFallSpeed = -3): the fall pose waits for real
/// falling speed; the apex floats in the Mid state.
pub(super) const FALL_ANIM_SPEED: f32 = 3.0;
/// Unity AnimationMovementBlendLogic.BLEND_EPSILON: a blend below this is idle, which is what
/// keeps a wall-pinned avatar from stepping in place.
const GAIT_BLEND_EPSILON: f32 = 0.01;
/// #2854 M12: external-force multiplier while gliding (Unity GlideWindResponse).
pub(super) const GLIDE_WIND_RESPONSE: f32 = 1.5;

/// Gait kind from the input mode (Unity MovementKind): 0 idle, 1 walk, 2 jog, 3 run.
pub(super) const KIND_IDLE: i32 = 0;
pub(super) const KIND_WALK: i32 = 1;
pub(super) const KIND_JOG: i32 = 2;
pub(super) const KIND_RUN: i32 = 3;

#[derive(Default)]
pub(super) struct MovementSystems {
    /// #2852 M6: apex of the current airborne stretch; the landing stun triggers on fall HEIGHT
    /// (apex - landing), not on scene-driven cooldowns.
    pub fall_apex_y: f32,
    /// XZ displacement the support platform caused last frame, subtracted from the measured
    /// displacement so the animation reads avatar-relative motion, not platform carry.
    pub platform_carry_xz: Vector2,
    platform: Option<Gd<CollisionObject3D>>,
    platform_last_transform: Transform3D,
    /// A sphere, not a ray: a ray down the capsule axis only ever hits on that axis, so the
    /// off-axis contact the slip looks for never shows up.
    edge_slip_cast: Option<Gd<ShapeCast3D>>,
}

/// The walk/jog/run flags and the gait blend for a measured speed. Unity: the gait KIND comes from
/// the input mode and the blend from the measured speed; reading the kind off the blend flips it
/// while a stick holds near a walk/jog cap.
pub(super) fn gait_flags(
    kind: i32,
    has_move_input: bool,
    speed: f32,
    limits: [f32; 3],
) -> [bool; 3] {
    let limit = match kind {
        KIND_WALK => limits[0],
        KIND_JOG => limits[1],
        KIND_RUN => limits[2],
        _ => 0.0,
    };
    let blend = if has_move_input && limit > 0.0 {
        (speed / limit).clamp(0.0, 1.0) * kind as f32
    } else {
        0.0
    };
    let moving = blend > GAIT_BLEND_EPSILON;
    [
        moving && kind == KIND_WALK,
        moving && kind == KIND_JOG,
        moving && kind == KIND_RUN,
    ]
}

/// The SlopeVelocityModifier curve for a signed slope angle in degrees (positive uphill).
fn slope_speed_curve(angle_deg: f32) -> f32 {
    let a = angle_deg.clamp(-SLOPE_MOD_MAX_DEG, SLOPE_MOD_MAX_DEG);
    if a < 0.0 {
        1.0 + (SLOPE_MOD_DOWNHILL - 1.0) * (-a / SLOPE_MOD_MAX_DEG)
    } else {
        1.0 + (SLOPE_MOD_UPHILL - 1.0) * (a / SLOPE_MOD_MAX_DEG)
    }
}

/// Downhill tangent of a surface with normal `n`.
fn downhill_tangent(n: Vector3) -> Vector3 {
    let g = Vector3::DOWN;
    (g - n * g.dot(n)).normalized_or_zero()
}

impl Player {
    pub(super) fn build_edge_slip_cast(&mut self) {
        let mut sphere = SphereShape3D::new_gd();
        sphere.set_radius(CAPSULE_RADIUS);
        let mut cast = ShapeCast3D::new_alloc();
        cast.set_shape(&sphere);
        cast.set_collision_mask(self.base().get_collision_mask());
        cast.set_target_position(Vector3::new(0.0, -CAPSULE_CENTER_Y * 1.2, 0.0));
        cast.set_position(Vector3::new(0.0, CAPSULE_CENTER_Y, 0.0));
        // Updated on demand; an always-on sweep would report the frame before.
        cast.set_enabled(false);
        self.base_mut().add_child(&cast);
        for rid in self.raycast_exclude.iter_shared() {
            cast.add_exception_rid(rid);
        }
        self.movement.edge_slip_cast = Some(cast);
    }

    /// #2852 M2 — ApplyCharacterMovementVelocity.cs:18-21. slopeForward is the input direction
    /// projected onto the slope plane; the signed angle from the input direction to it (around
    /// input × up) is positive uphill.
    pub(super) fn slope_speed_modifier(&self, input_dir: Vector3) -> f32 {
        let look = Vector3::new(input_dir.x, 0.0, input_dir.z);
        if look.length_squared() < 0.01 {
            return 1.0;
        }
        let look = look.normalized();
        let n = self.base().get_floor_normal();
        let slope_forward = n.cross(look.cross(n));
        if slope_forward.length_squared() < 0.01 {
            return 1.0;
        }
        let angle = look
            .signed_angle_to(slope_forward.normalized(), look.cross(Vector3::UP))
            .to_degrees();
        slope_speed_curve(angle)
    }

    /// #2852 M3 — ApplyGravity.cs SlopeGravity: the downhill tangent of a steeper-than-walkable
    /// slide contact (ZERO when none). n.y > 0.05 excludes vertical walls.
    pub(super) fn steep_slide_dir(&self) -> Vector3 {
        for i in 0..self.base().get_slide_collision_count() {
            let Some(collision) = self.base().get_slide_collision(i) else {
                continue;
            };
            let n = collision.get_normal();
            if n.y < WALKABLE_NORMAL_Y && n.y > 0.05 {
                return downhill_tangent(n);
            }
        }
        Vector3::ZERO
    }

    /// #2852 M4 — ApplyEdgeSlip.cs: a ground contact offset past NoSlipDistance with no ground
    /// straight below tilts gravity toward the contact normal's downhill tangent.
    pub(super) fn edge_slip_gravity_dir(&mut self) -> Vector3 {
        if !self.base().is_on_floor() {
            return Vector3::ZERO;
        }
        let Some(cast) = self.movement.edge_slip_cast.as_mut() else {
            return Vector3::ZERO;
        };
        cast.force_shapecast_update();
        if !cast.is_colliding() {
            return Vector3::ZERO;
        }
        let (point, normal) = (cast.get_collision_point(0), cast.get_collision_normal(0));
        let position = self.base().get_global_position();
        let mut rel = point - (position + Vector3::new(0.0, CAPSULE_CENTER_Y, 0.0));
        rel.y = 0.0;
        if rel.length() <= EDGE_NO_SLIP_DIST {
            return Vector3::ZERO;
        }
        let Some(mut space) = self
            .base()
            .get_world_3d()
            .and_then(|w| w.get_direct_space_state())
        else {
            return Vector3::ZERO;
        };
        let Some(mut query) = PhysicsRayQueryParameters3D::create(
            position,
            position + Vector3::new(0.0, -EDGE_SLIP_SAFE_DISTANCE, 0.0),
        ) else {
            return Vector3::ZERO;
        };
        query.set_collision_mask(self.base().get_collision_mask());
        query.set_exclude(&self.raycast_exclude);
        if !space.intersect_ray(&query).is_empty() {
            return Vector3::ZERO;
        }
        downhill_tangent(normal)
    }

    /// #2852 M5 — ApplyWallSlide.cs: facing into a wall brakes movement toward zero, parallel is
    /// free. Unity capsulecasts ahead to find the wall; the live contact gives the same result.
    pub(super) fn apply_wall_slide(&self, velocity: &mut Vector3) {
        if !self.base().is_on_wall() {
            return;
        }
        let mut wall_n = self.base().get_wall_normal();
        wall_n.y = 0.0;
        let look = Vector3::new(self.current_direction.x, 0.0, self.current_direction.z);
        if wall_n.length_squared() <= 0.01 || look.length_squared() <= 0.01 {
            return;
        }
        let wall_dot = look.normalized().dot(wall_n.normalized()).abs();
        let mult = 1.0 + (WALL_SLIDE_MIN_MULT - 1.0) * wall_dot;
        velocity.x *= mult;
        velocity.z *= mult;
    }

    /// #2852 M11 — CharacterPlatformSystem.cs: stay attached to the platform underfoot through
    /// translation AND rotation. Kinematic colliders moved by CRDT transform sync report no
    /// velocity, except when a real floor contact forms: what the engine carried is subtracted.
    pub(super) fn update_platform_follow(&mut self, supported: bool, dt: f32) {
        let mut current: Option<Gd<CollisionObject3D>> = None;
        if supported {
            let position = self.base().get_global_position();
            let space = self
                .base()
                .get_world_3d()
                .and_then(|w| w.get_direct_space_state());
            let query = PhysicsRayQueryParameters3D::create(
                position + Vector3::new(0.0, 0.05, 0.0),
                position + Vector3::new(0.0, -0.2, 0.0),
            );
            if let (Some(mut space), Some(mut query)) = (space, query) {
                query.set_collision_mask(self.base().get_collision_mask());
                query.set_exclude(&self.raycast_exclude);
                let hit = space.intersect_ray(&query);
                let walkable = hit
                    .get("normal")
                    .is_some_and(|n| n.to::<Vector3>().y >= WALKABLE_NORMAL_Y);
                if walkable {
                    current = hit
                        .get("collider")
                        .and_then(|c| c.try_to::<Gd<CollisionObject3D>>().ok());
                }
            }
        }
        let previous = self
            .movement
            .platform
            .as_ref()
            .filter(|p| p.is_instance_valid())
            .map(|p| p.instance_id());
        if current.as_ref().map(|c| c.instance_id()) != previous {
            self.movement.platform_last_transform = current
                .as_ref()
                .map(|c| c.get_global_transform())
                .unwrap_or(Transform3D::IDENTITY);
            self.movement.platform = current;
            self.movement.platform_carry_xz = Vector2::ZERO;
            return;
        }
        let Some(platform) = current else {
            self.movement.platform_carry_xz = Vector2::ZERO;
            return;
        };
        let xf_now = platform.get_global_transform();
        // World-space carry: prev^-1 * now for the yaw, now * prev^-1 for points.
        let last_inverse = self.movement.platform_last_transform.affine_inverse();
        let rel = last_inverse * xf_now;
        let carry = xf_now * last_inverse;
        self.movement.platform_last_transform = xf_now;
        if rel.approx_eq(&Transform3D::IDENTITY) {
            self.movement.platform_carry_xz = Vector2::ZERO;
            return;
        }
        // The avatar is top-level, so it does not inherit the body yaw: rotate it explicitly, and
        // the smoothed facing with it, or it counter-rotates on a spinner.
        let before = self.base().get_global_position();
        let yaw = rel.basis.get_euler().y;
        let carried = carry * before;
        self.movement.platform_carry_xz = Vector2::new(carried.x - before.x, carried.z - before.z);
        // Jolt applies platform velocity once a real floor contact forms: subtract what
        // move_and_slide already carried or the translation lands twice.
        let platform_velocity = self.base().get_platform_velocity();
        self.base_mut()
            .set_global_position(carried - platform_velocity * dt);
        let mut rotation = self.base().get_rotation();
        rotation.y += yaw;
        self.base_mut().set_rotation(rotation);
        if let Some(avatar) = &mut self.avatar {
            let mut rotation = avatar.get_rotation();
            rotation.y += yaw;
            avatar.set_rotation(rotation);
        }
        self.current_direction = rel.basis * self.current_direction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slope_curve_matches_unity_keys() {
        assert_eq!(slope_speed_curve(0.0), 1.0);
        assert!((slope_speed_curve(55.0) - 0.65).abs() < 1e-6);
        assert!((slope_speed_curve(-55.0) - 1.35).abs() < 1e-6);
        assert!((slope_speed_curve(80.0) - 0.65).abs() < 1e-6);
        assert!((slope_speed_curve(30.0) - 0.809).abs() < 0.01);
        assert!((slope_speed_curve(-30.0) - 1.191).abs() < 0.01);
    }

    #[test]
    fn gait_kind_comes_from_input_not_speed() {
        let limits = [1.5, 8.0, 10.0];
        // Jogging just under the walk cap still reads as jog.
        assert_eq!(
            gait_flags(KIND_JOG, true, 1.4, limits),
            [false, true, false]
        );
        assert_eq!(
            gait_flags(KIND_RUN, true, 9.0, limits),
            [false, false, true]
        );
        // Pinned against a wall: no measured speed, idle.
        assert_eq!(gait_flags(KIND_RUN, true, 0.0, limits), [false; 3]);
        // No input is idle whatever the platform carry left in the speed.
        assert_eq!(gait_flags(KIND_IDLE, false, 3.0, limits), [false; 3]);
    }

    #[test]
    fn downhill_tangent_points_down_the_slope() {
        let n = Vector3::new(0.0, 1.0, 1.0).normalized();
        let t = downhill_tangent(n);
        assert!(t.y < 0.0 && t.z > 0.0);
        assert!(t.dot(n).abs() < 1e-5);
    }
}
