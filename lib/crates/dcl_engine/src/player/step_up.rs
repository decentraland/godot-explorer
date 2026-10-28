//! Custom step offset: CharacterBody3D has none. The step-up is a PhysX CCT relocation test: the
//! margin-less capsule clone is moved up (the lift cap is the climbable band), forward past the
//! face and down; point rays then read the landing, measured from the resting contact.

use godot::classes::{PhysicsRayQueryParameters3D, PhysicsShapeQueryParameters3D};
use godot::prelude::*;

use super::body::{
    Player, CAPSULE_CENTER_Y, CAPSULE_RADIUS, REALM_FLOOR_EPS, SLOPE_WALK_NORMAL_Y,
    STEP_MAX_HEIGHT, STEP_MIN_RISE, STEP_PENDING_RISE, STEP_TALL_RISE, WALKABLE_NORMAL_Y,
};

fn ray(
    from: Vector3,
    to: Vector3,
    mask: u32,
    exclude: &Array<Rid>,
) -> Option<Gd<PhysicsRayQueryParameters3D>> {
    let mut query = PhysicsRayQueryParameters3D::create(from, to)?;
    query.set_collision_mask(mask);
    query.set_exclude(exclude);
    Some(query)
}

impl Player {
    fn shape_query(
        &self,
        transform: Transform3D,
        motion: Vector3,
    ) -> Option<Gd<PhysicsShapeQueryParameters3D>> {
        let mut query = PhysicsShapeQueryParameters3D::new_gd();
        // The margin-less clone: the skin width would eat the step band.
        query.set_shape(self.step_test_shape.as_ref()?);
        query.set_collision_mask(self.base().get_collision_mask());
        query.set_exclude(&self.raycast_exclude);
        query.set_transform(transform);
        query.set_motion(motion);
        Some(query)
    }

    /// Predictive step (PhysX CCT parity: the step happens inside the move, not after a blocked
    /// frame). The intended motion is probed before `move_and_slide`; on a step, rise first and
    /// let the horizontal motion flow unobstructed.
    pub(super) fn try_step_up_predictive(&mut self, intent: Vector3, dt: f32) {
        let horiz = Vector3::new(intent.x, 0.0, intent.z);
        if horiz.length_squared() < 0.25 {
            return;
        }
        // Already walking a climbable slope: the contact ahead is the slope itself, not a riser,
        // and stepping here turns a smooth incline into a stuttered staircase. The post-move
        // fallback still covers blocked cases.
        if self.base().is_on_floor() && self.base().get_floor_normal().y < SLOPE_WALK_NORMAL_Y {
            return;
        }
        let Some(world) = self.base().get_world_3d() else {
            return;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return;
        };
        let origin = self.base().get_global_position() + Vector3::new(0.0, CAPSULE_CENTER_Y, 0.0);
        let motion = horiz.normalized() * (horiz.length() * dt + 0.05);
        let Some(query) = self.shape_query(Transform3D::new(Basis::IDENTITY, origin), motion)
        else {
            return;
        };
        let contact = space.cast_motion(&query);
        if contact.as_slice().first().copied().unwrap_or(1.0) >= 1.0 {
            return; // nothing in the way this frame
        }
        self.step_up(intent);
    }

    /// Post-move fallback: the predictive pass covers the approach; this catches direction
    /// changes mid-contact. Trigger: wall contact OR blocked horizontal motion. The rounded
    /// capsule bottom meets a riser with a diagonal normal that classifies as FLOOR for the first
    /// frames (< 46 deg); waiting for `is_on_wall()` alone adds a visible hitch.
    pub(super) fn try_step_up(&mut self, intent: Vector3, moved_xz: f32) {
        let expected = Vector3::new(intent.x, 0.0, intent.z).length()
            * self.base().get_physics_process_delta_time() as f32;
        let blocked = expected > 0.008 && moved_xz < expected * 0.3;
        if !self.base().is_on_wall() && !blocked {
            return;
        }
        self.step_up(intent);
    }

    fn step_up(&mut self, intent: Vector3) {
        if !self.step_armed {
            return;
        }
        let horiz = Vector3::new(intent.x, 0.0, intent.z);
        let Some(world) = self.base().get_world_3d() else {
            return;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return;
        };
        if horiz.length_squared() < 0.25 {
            return;
        }
        let Some(shape) = self.step_test_shape.clone() else {
            return;
        };
        let dir = horiz.normalized();
        let position = self.base().get_global_position();
        let origin = position + Vector3::new(0.0, CAPSULE_CENTER_Y, 0.0);
        let mask = self.base().get_collision_mask();
        let exclude = self.raycast_exclude.clone();

        // PhysX CCT relocation test: instead of measuring the obstacle with point probes, move
        // the (margin-less) capsule itself. The test uses the same shape the solver moves, so
        // seam lips, stacked colliders and trimesh edges need no special cases, and the lift cap
        // IS the climbable band (nothing measured, nothing to flake at the boundary).
        let cast = |space: &mut Gd<godot::classes::PhysicsDirectSpaceState3D>,
                    transform: Transform3D,
                    motion: Vector3|
         -> Option<f32> {
            let query = self.shape_query(transform, motion)?;
            space.cast_motion(&query).as_slice().first().copied()
        };

        // a) face distance (margin-less clone travel + radius): needed to PLACE the down phase
        //    with the pole just past the edge, so the capsule's own down sweep reads the top
        //    exactly, with no hemisphere graze.
        let low = self
            .shape_query(Transform3D::new(Basis::IDENTITY, origin), dir * 0.5)
            .map(|query| space.cast_motion(&query))
            .unwrap_or_default();
        // A miss means the cast starts overlapped (pole wedged in a ground-level seam) or the
        // blocker sits below the pole: proceed as if flush; the rest of the pipeline validates.
        let low_unsafe = low.as_slice().get(1).copied().unwrap_or(1.0);
        let d_face = if low_unsafe < 1.0 {
            low_unsafe * 0.5 + CAPSULE_RADIUS
        } else {
            CAPSULE_RADIUS
        };

        // b) up: how far the capsule can be lifted (doubles as the headroom check).
        let Some(up) = cast(
            &mut space,
            Transform3D::new(Basis::IDENTITY, origin),
            Vector3::new(0.0, STEP_MAX_HEIGHT, 0.0),
        ) else {
            return;
        };
        let lift = up * STEP_MAX_HEIGHT;
        if lift < STEP_MIN_RISE {
            return; // ceiling
        }

        // c) forward at lifted height until the pole sits just past the face: a riser taller than
        //    the lift blocks this, a wall and not a step.
        let over_face = d_face + 0.005;
        let lifted = origin + Vector3::new(0.0, lift, 0.0);
        let Some(forward) = cast(
            &mut space,
            Transform3D::new(Basis::IDENTITY, lifted),
            dir * over_face,
        ) else {
            return;
        };
        if forward < 1.0 {
            return;
        }

        // d) down with the pole directly over the edge. Void past the band means a gap, not a
        //    step (the capsule steps UP only).
        let landing = lifted + dir * over_face;
        let Some(down) = cast(
            &mut space,
            Transform3D::new(Basis::IDENTITY, landing),
            Vector3::new(0.0, -(lift + 0.02), 0.0),
        ) else {
            return;
        };
        if !(0.005..1.0).contains(&down) {
            // >= 1: void past the band, a gap. ~0: the down phase starts already touching,
            // squeezed against the face/edge at lifted height, a wall (reading that contact as a
            // landing reports floor_y = lifted height and teleports up).
            return;
        }

        let mut floor_y = landing.y - down * (lift + 0.02) - shape.get_height() * 0.5;
        if floor_y < position.y + STEP_MIN_RISE {
            return; // flat or lower: nothing to step onto
        }
        // Never commit an overlapping relocation (CCT invariant): diagonal corner approaches can
        // thread the lifted forward cast past the block's edge and end with the pole inside the
        // top slab.
        if let Some(query) = self.shape_query(
            Transform3D::new(
                Basis::IDENTITY,
                Vector3::new(landing.x, floor_y + CAPSULE_CENTER_Y, landing.z),
            ),
            Vector3::ZERO,
        ) {
            let overlaps = space.intersect_shape_ex(&query).max_results(1).done();
            if !overlaps.is_empty() {
                return;
            }
        }

        // The band is measured from the LOWEST walkable contact, the real ground under the feet:
        // the higher edge/corner contacts would inflate the band and let a 0.44 top pass. A ray
        // under the axis misses edge rests (the center hangs past the edge) and the pole
        // smuggles +1cm when the hemisphere rides an edge.
        let mut support_y = f32::INFINITY;
        let mut has_contact = false;
        for i in 0..self.base().get_slide_collision_count() {
            let Some(contact) = self.base().get_slide_collision(i) else {
                continue;
            };
            has_contact = true;
            if contact.get_normal().y >= WALKABLE_NORMAL_Y {
                support_y = support_y.min(contact.get_position().y);
            }
        }
        if support_y == f32::INFINITY {
            if position.y <= REALM_FLOOR_EPS {
                support_y = 0.0; // clamp-held realm floor
            } else if has_contact {
                support_y = position.y; // wedged/hanging: from the pole
            }
        }
        // The ground under the feet is a candidate too: a pole hovering on an edge registers no
        // ground contact, and the edge contact would inflate the band.
        if let Some(query) = ray(
            position + Vector3::new(0.0, 0.05, 0.0),
            position + Vector3::new(0.0, -0.15, 0.0),
            mask,
            &exclude,
        ) {
            let hit = space.intersect_ray(&query);
            if let Some(hit_position) = hit.get("position") {
                support_y = support_y.min(hit_position.to::<Vector3>().y);
            }
        }

        // Probe the landing footprint near-to-far; the FIRST in-band read is the landing: a low
        // lip in front of a walkable incline is what you step on, not the higher surface behind
        // it. Nothing in-band: an over-band read blocks (wall tops), void/below reads miss (gaps).
        let mut found = false;
        let mut won_ny = 1.0;
        for dist in [d_face + 0.005, d_face + 0.15, d_face + 0.3] {
            let px = position.x + dir.x * dist;
            let pz = position.z + dir.z * dist;
            let Some(query) = ray(
                Vector3::new(px, position.y + lift + 0.1, pz),
                Vector3::new(px, position.y - 0.05, pz),
                mask,
                &exclude,
            ) else {
                continue;
            };
            let hit = space.intersect_ray(&query);
            let Some(hit_position) = hit.get("position") else {
                continue;
            };
            let hy = hit_position.to::<Vector3>().y;
            if hy > support_y + STEP_MAX_HEIGHT || hy < position.y + STEP_MIN_RISE {
                continue;
            }
            floor_y = hy;
            found = true;
            won_ny = hit
                .get("normal")
                .map(|n| n.to::<Vector3>().y)
                .unwrap_or(1.0);
            break;
        }
        if !found || support_y == f32::INFINITY {
            return;
        }

        // A continuous ramp never lets go: the landing AND the surface a radius past it both
        // read un-walkable slope. Rounded curbs, bevels and staircase noses all read slope at the
        // landing but have flat ground (or a drop) behind: those climb. A ray miss accepts
        // (trimesh tri edges).
        let nx = position.x + dir.x * (d_face + 0.3);
        let nz = position.z + dir.z * (d_face + 0.3);
        if let Some(query) = ray(
            Vector3::new(nx, position.y + lift + 0.5, nz),
            Vector3::new(nx, position.y - 0.05, nz),
            mask,
            &exclude,
        ) {
            let hit = space.intersect_ray(&query);
            if let Some(normal) = hit.get("normal") {
                if won_ny < WALKABLE_NORMAL_Y && normal.to::<Vector3>().y < WALKABLE_NORMAL_Y {
                    return; // the slope continues past the landing: a ramp, not a step
                }
            }
        }

        // Rise in place; the horizontal motion flows via move_and_slide itself, so there is no
        // blocked frame and no forward teleport pop. Tall rises disarm until the capsule rests:
        // re-triggering on the same ramp face is what stair-climbs steep inclines. Small rises
        // (bumps, treads) don't disarm, but two in a row without a rest in between do: that
        // chain is how a 60-65 deg ramp climbs in sub-0.2 bands. Snap off for this frame's move:
        // the lower floor is within snap reach and would re-glue the capsule mid-step.
        let rise = floor_y - position.y;
        if rise > STEP_TALL_RISE || (rise >= STEP_PENDING_RISE && self.step_pending) {
            self.step_armed = false;
        } else if rise >= STEP_PENDING_RISE {
            self.step_pending = true;
        }
        self.base_mut().set_floor_snap_length(0.0);
        let mut raised = self.base().get_global_position();
        raised.y = floor_y + 0.001;
        self.base_mut().set_global_position(raised);
    }
}
