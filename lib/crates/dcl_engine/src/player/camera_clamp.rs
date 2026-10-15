//! Secondary camera collision: sweeps a sphere from the SpringArm pivot to the ACTUAL camera
//! position (lateral offset included) every frame and pulls the camera in along that segment when
//! the sphere would hit solid geometry.
//!
//! The SpringArm3D only collision-checks a thin ray along its own axis, so geometry beside the
//! camera was invisible to it; a sphere big enough to cover the offset would over-shorten near
//! any geometry. Sweeping a modest sphere to the real camera position is precise: it only
//! shortens when the actual camera volume is blocked.
//!
//! A child of Mount (SpringArm3D), after CameraArm: children process after the parent's internal
//! physics step, so CameraArm.position.z already holds this frame's spring-resolved length.

use godot::classes::{
    Camera3D, INode3D, Node3D, PhysicsRayQueryParameters3D, PhysicsShapeQueryParameters3D,
    SphereShape3D, SpringArm3D,
};
use godot::prelude::*;

use super::rig::{
    CAMERA_COLLISION_MASK, CLAMP_CONTACT_RADIUS, CLAMP_EXTEND_SPEED, CLAMP_EXTRA_MARGIN,
    CLAMP_NEAR_CLEARANCE, CLAMP_SPHERE_RADIUS, FLOOR_CLEARANCE, THIRD_PERSON_CAMERA_Z,
};

/// The spring resolves its length during its first physics steps (CameraArm starts at z=0). Those
/// frames are skipped: clamping against the unresolved zero length would slam the camera to the
/// pivot and slowly recover for no reason.
const WARMUP_FRAMES: u32 = 3;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct CameraCollisionClamp {
    base: Base<Node3D>,
    /// Tweened by the player on camera-mode changes.
    #[var(pub)]
    lateral_offset: f32,
    params: Gd<PhysicsShapeQueryParameters3D>,
    ray_params: Gd<PhysicsRayQueryParameters3D>,
    // Static contact sphere at the camera position (depenetration).
    contact_params: Gd<PhysicsShapeQueryParameters3D>,
    /// The contact offset and the smoothed distance from the pivot along the camera segment:
    /// shortening is instant, extending is smoothed so geometry does not pop through.
    state: ClampState,
    warmup_frames: u32,
    mount: Option<Gd<SpringArm3D>>,
    camera_arm: Option<Gd<Node3D>>,
    camera: Option<Gd<Camera3D>>,
    player_body: Option<Gd<Node3D>>,
}

#[godot_api]
impl INode3D for CameraCollisionClamp {
    fn init(base: Base<Node3D>) -> Self {
        let mut sphere = SphereShape3D::new_gd();
        sphere.set_radius(CLAMP_SPHERE_RADIUS);
        let mut params = PhysicsShapeQueryParameters3D::new_gd();
        params.set_shape(&sphere);
        params.set_collision_mask(CAMERA_COLLISION_MASK);
        params.set_collide_with_bodies(true);
        params.set_collide_with_areas(false);
        // Inflates the swept shape so contact is reported with perpendicular clearance: the near
        // plane must never rest inside the wall.
        params.set_margin(CLAMP_NEAR_CLEARANCE);

        let mut ray_params = PhysicsRayQueryParameters3D::new_gd();
        ray_params.set_collision_mask(CAMERA_COLLISION_MASK);
        ray_params.set_collide_with_bodies(true);
        ray_params.set_collide_with_areas(false);

        let mut contact_sphere = SphereShape3D::new_gd();
        contact_sphere.set_radius(CLAMP_CONTACT_RADIUS);
        let mut contact_params = PhysicsShapeQueryParameters3D::new_gd();
        contact_params.set_shape(&contact_sphere);
        contact_params.set_collision_mask(CAMERA_COLLISION_MASK);
        contact_params.set_collide_with_bodies(true);
        contact_params.set_collide_with_areas(false);
        contact_params.set_margin(0.0);

        Self {
            base,
            lateral_offset: 0.0,
            params,
            ray_params,
            contact_params,
            // Start fully extended so there is no snap on the first frames.
            state: ClampState {
                contact_offset: Vector3::ZERO,
                smoothed_dist: THIRD_PERSON_CAMERA_Z,
            },
            warmup_frames: WARMUP_FRAMES,
            mount: None,
            camera_arm: None,
            camera: None,
            player_body: None,
        }
    }

    fn ready(&mut self) {
        let mount = self
            .base()
            .get_parent()
            .and_then(|parent| parent.try_cast::<SpringArm3D>().ok());
        self.camera_arm = mount
            .as_ref()
            .and_then(|mount| mount.try_get_node_as::<Node3D>("CameraArm"));
        self.camera = self
            .camera_arm
            .as_ref()
            .and_then(|arm| arm.try_get_node_as::<Camera3D>("Camera3D"));
        // The player CharacterBody3D: its origin rests on the ground, so its Y is a
        // scene-collider-free floor reference for the floor guard.
        self.player_body = mount
            .as_ref()
            .and_then(|mount| mount.get_parent())
            .and_then(|parent| parent.try_cast::<Node3D>().ok());
        self.mount = mount;
    }

    /// In `process` (not `physics_process`): physics steps complete first, so CameraArm.position.z
    /// is fresh, and input-driven camera rotations applied between physics ticks are caught on
    /// the same rendered frame; a fast mouse flick can otherwise rotate the camera into a wall
    /// for a visible frame.
    fn process(&mut self, delta: f64) {
        if self.warmup_frames > 0 {
            self.warmup_frames -= 1;
            return;
        }
        self.clamp(delta as f32);
    }
}

struct RayHit {
    position: Vector3,
    normal: Vector3,
}

struct Contact {
    point: Vector3,
    normal: Vector3,
}

/// The physics queries the clamp needs, so the algorithm can run against a fake world in tests.
/// Implementations apply the rig's radii and margins (`rig::CLAMP_*`).
trait ClampWorld {
    /// First solid hit along `from -> to`.
    fn ray(&mut self, from: Vector3, to: Vector3) -> Option<RayHit>;
    /// Whether the sweep sphere (`CLAMP_SPHERE_RADIUS` plus `CLAMP_NEAR_CLEARANCE`) at `center`
    /// already overlaps something.
    fn sweep_sphere_blocked_at(&mut self, center: Vector3) -> bool;
    /// How far (0..1) the sweep sphere moves from `origin` along `motion` before touching
    /// something; `None` when the whole motion is clear.
    fn sweep_sphere(&mut self, origin: Vector3, motion: Vector3) -> Option<f32>;
    /// Deepest contact of the camera's own bubble (`CLAMP_CONTACT_RADIUS`) at `center`.
    fn contact(&mut self, center: Vector3) -> Option<Contact>;
}

#[derive(Default)]
struct ClampState {
    contact_offset: Vector3,
    smoothed_dist: f32,
}

/// Where the camera goes: the arm's analytic `target` pulled in against the geometry between
/// `pivot` and it. `mount_back` is the mount's local +Z (the player's back).
fn resolve_camera(
    world: &mut dyn ClampWorld,
    state: &mut ClampState,
    pivot: Vector3,
    target: Vector3,
    mount_back: Vector3,
    delta: f32,
) -> Option<Vector3> {
    let segment = target - pivot;
    let dist = segment.length();
    if dist < 0.001 {
        return None;
    }
    let dir = segment / dist;

    // cast_motion IGNORES geometry the shape already overlaps at the start (it reports
    // safe=1.0, all clear). Two layers:
    //
    // 1) RAY WHISKERS from the pivot: three parallel rays along the segment (center +/- sphere
    //    radius, horizontally). A point can never start overlapped (the pivot lives inside the
    //    player capsule), so the camera CENTER can never cross a collider in the ribbon,
    //    whatever the orbit, wall thickness or shift alignment. The OUTER whiskers also catch
    //    walls BESIDE the center line: curved surfaces bulging into the camera.
    let mut allowed = dist;
    let mut perp = dir.cross(Vector3::UP);
    if perp.length_squared() < 0.01 {
        perp = dir.cross(Vector3::RIGHT);
    }
    let perp = perp.normalized();
    for k in [-CLAMP_SPHERE_RADIUS, 0.0, CLAMP_SPHERE_RADIUS] {
        let from = pivot + perp * k;
        if let Some(hit) = world.ray(from, from + segment) {
            // Back off along the segment enough that the PERPENDICULAR clearance to the wall
            // equals CLAMP_NEAR_CLEARANCE (capped for grazing rays).
            let cosang = dir.dot(hit.normal).abs().clamp(0.3, 1.0);
            let backoff = CLAMP_NEAR_CLEARANCE / cosang;
            let hit_allowed = (dir.dot(hit.position - pivot) - backoff).max(0.0);
            allowed = allowed.min(hit_allowed);
        }
    }

    // 2) SPHERE VOLUME from a point shifted along the mount's FORWARD (the player's front, always
    //    opposite to where the camera looks back from). Back against a wall, a sphere at the
    //    pivot would already penetrate it and the cast would go blind; shifted, it starts clear
    //    and detects it. But facing a THIN wall (e.g. a door box), the shifted origin can land
    //    INSIDE the wall and the cast is blind again (hugging a door and orbiting 360 put the
    //    camera on the far side). So the sphere is only trusted when its origin is NOT
    //    overlapped; the whiskers always apply regardless.
    let shift = CLAMP_SPHERE_RADIUS.min(dist);
    let origin = pivot - mount_back * shift;
    let motion = target - origin;
    if !world.sweep_sphere_blocked_at(origin) {
        if let Some(safe) = world.sweep_sphere(origin, motion) {
            // Contact position of the sphere center, projected back onto the camera segment so
            // framing stays on the arm's line. May end up in front of the pivot, which is
            // correct when hugging a wall (the own-avatar proximity fade hides the head; no
            // distance floor).
            let contact_pos = origin + motion * safe
                - super::rig::normalized_or_zero(motion) * CLAMP_EXTRA_MARGIN;
            allowed = allowed.min(dir.dot(contact_pos - pivot).clamp(-shift, dist));
        }
    }

    if allowed < state.smoothed_dist {
        state.smoothed_dist = allowed; // shorten instantly: never clip
    } else {
        state.smoothed_dist +=
            (allowed - state.smoothed_dist).clamp(0.0, CLAMP_EXTEND_SPEED * delta);
    }

    let base_pos = pivot + dir * state.smoothed_dist;

    // CONTACT SPHERE: a static sphere at the camera position, depenetrated from anything the
    // line clamp could not see. It covers the cases no longitudinal cast handles: walls PARALLEL
    // to the segment (pull-in can't help, clearance along them is constant), corners poking into
    // the frustum sides, and any residual poke while the sweep sphere is blind.
    //
    // The resolution is computed STATELESSLY from `base` every frame: while the contact
    // persists, the depenetrated position is a deterministic fixed point, so nothing oscillates.
    // (A stateful version started from `base + offset` and decayed the offset the moment contact
    // cleared, which pulled the camera straight back INTO contact: the frame-on/frame-off
    // flicker.) The stored offset only smooths the RETURN to the arm's line once the contact is
    // truly gone, and snaps instantly whenever a push is needed.
    let mut pos = base_pos;
    let mut pushed = false;
    for _ in 0..3 {
        let Some(contact) = world.contact(pos) else {
            break;
        };
        let depth = CLAMP_CONTACT_RADIUS - (pos - contact.point).dot(contact.normal);
        if depth <= 0.0 {
            break;
        }
        pushed = true;
        pos += contact.normal * (depth + CLAMP_EXTRA_MARGIN);
    }
    let target_offset = if pushed {
        pos - base_pos
    } else {
        Vector3::ZERO
    };
    if pushed && target_offset.length() >= state.contact_offset.length() {
        state.contact_offset = target_offset; // growing push: instant, never clip
    } else {
        state.contact_offset = state
            .contact_offset
            .move_toward(target_offset, CLAMP_EXTEND_SPEED * delta);
    }
    Some(base_pos + state.contact_offset)
}

/// The real physics space.
struct SpaceWorld<'a> {
    space: Gd<godot::classes::PhysicsDirectSpaceState3D>,
    ray_params: &'a mut Gd<PhysicsRayQueryParameters3D>,
    sweep_params: &'a mut Gd<PhysicsShapeQueryParameters3D>,
    contact_params: &'a mut Gd<PhysicsShapeQueryParameters3D>,
}

impl ClampWorld for SpaceWorld<'_> {
    fn ray(&mut self, from: Vector3, to: Vector3) -> Option<RayHit> {
        self.ray_params.set_from(from);
        self.ray_params.set_to(to);
        let hit = self.space.intersect_ray(&*self.ray_params);
        let position = hit.get("position")?.to::<Vector3>();
        Some(RayHit {
            position,
            normal: hit
                .get("normal")
                .map(|n| n.to::<Vector3>())
                .unwrap_or(Vector3::ZERO),
        })
    }

    fn sweep_sphere_blocked_at(&mut self, center: Vector3) -> bool {
        self.sweep_params
            .set_transform(Transform3D::new(Basis::IDENTITY, center));
        self.sweep_params.set_motion(Vector3::ZERO);
        !self.space.get_rest_info(&*self.sweep_params).is_empty()
    }

    fn sweep_sphere(&mut self, origin: Vector3, motion: Vector3) -> Option<f32> {
        self.sweep_params
            .set_transform(Transform3D::new(Basis::IDENTITY, origin));
        self.sweep_params.set_motion(motion);
        let result = self.space.cast_motion(&*self.sweep_params);
        let result = result.as_slice();
        // cast_motion always returns [safe, unsafe]; safe < 1.0 means blocked.
        (result.len() >= 2 && result[0] < 1.0).then(|| result[0])
    }

    fn contact(&mut self, center: Vector3) -> Option<Contact> {
        self.contact_params
            .set_transform(Transform3D::new(Basis::IDENTITY, center));
        let rest = self.space.get_rest_info(&*self.contact_params);
        if rest.is_empty() {
            return None;
        }
        Some(Contact {
            normal: rest
                .get("normal")
                .map(|n| n.to::<Vector3>())
                .unwrap_or(Vector3::UP),
            point: rest
                .get("point")
                .map(|p| p.to::<Vector3>())
                .unwrap_or(center),
        })
    }
}

impl CameraCollisionClamp {
    fn clamp(&mut self, delta: f32) {
        let (Some(mount), Some(camera_arm), Some(camera)) =
            (&self.mount, &self.camera_arm, &mut self.camera)
        else {
            return;
        };
        let Some(world) = mount.get_world_3d() else {
            return;
        };
        let Some(space) = world.get_direct_space_state() else {
            return;
        };
        let pivot = mount.get_global_transform().origin;
        // Analytic target: spring-resolved length behind the pivot plus the lateral offset.
        // (Computed, not read from the camera, because the camera position is overwritten below
        // and must not feed back into the next frame.)
        let target = mount.get_global_transform()
            * Vector3::new(self.lateral_offset, 0.0, camera_arm.get_position().z);
        let mut physics = SpaceWorld {
            space,
            ray_params: &mut self.ray_params,
            sweep_params: &mut self.params,
            contact_params: &mut self.contact_params,
        };
        let Some(mut position) = resolve_camera(
            &mut physics,
            &mut self.state,
            pivot,
            target,
            mount.get_global_transform().basis.col_c(),
            delta,
        ) else {
            return;
        };

        // Floor guard: keep the camera above the player's own ground contact so it cannot dip
        // below scene floors that have no usable collider (the sweep casts slip through
        // single-sided / cmask=0 meshes). Additive: only ever raises the camera.
        if let Some(body) = &self.player_body {
            if body.is_instance_valid() {
                position.y = position
                    .y
                    .max(body.get_global_position().y + FLOOR_CLEARANCE);
            }
        }
        camera.set_global_position(position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATERAL_OFFSET: f32 = 0.75;

    /// Solid geometry for the fake world: axis-aligned boxes and vertical cylinders.
    enum Solid {
        Box {
            center: Vector3,
            size: Vector3,
        },
        Cylinder {
            center: Vector3,
            radius: f32,
            height: f32,
        },
    }

    impl Solid {
        /// The point on the solid closest to `p` (`p` itself when inside).
        fn closest(&self, p: Vector3) -> Vector3 {
            match self {
                Solid::Box { center, size } => {
                    let half = *size * 0.5;
                    let (min, max) = (*center - half, *center + half);
                    Vector3::new(
                        p.x.clamp(min.x, max.x),
                        p.y.clamp(min.y, max.y),
                        p.z.clamp(min.z, max.z),
                    )
                }
                Solid::Cylinder {
                    center,
                    radius,
                    height,
                } => {
                    let flat = Vector2::new(p.x - center.x, p.z - center.z);
                    let flat = if flat.length() > *radius {
                        flat.normalized() * *radius
                    } else {
                        flat
                    };
                    Vector3::new(
                        center.x + flat.x,
                        p.y.clamp(center.y - height * 0.5, center.y + height * 0.5),
                        center.z + flat.y,
                    )
                }
            }
        }

        fn contains(&self, p: Vector3) -> bool {
            self.closest(p) == p
        }

        /// First hit of the segment, found by marching (fine enough for 5mm assertions).
        fn ray(&self, from: Vector3, to: Vector3) -> Option<RayHit> {
            let segment = to - from;
            let steps = (segment.length() / 0.002).ceil() as usize;
            let mut previous = from;
            if self.contains(from) {
                return None;
            }
            for i in 1..=steps {
                let point = from + segment * (i as f32 / steps as f32);
                if self.contains(point) {
                    return Some(RayHit {
                        position: point,
                        normal: self.face_normal(previous, point),
                    });
                }
                previous = point;
            }
            None
        }

        /// Normal of the face crossed between an outside point and an inside one.
        fn face_normal(&self, outside: Vector3, _inside: Vector3) -> Vector3 {
            let closest = self.closest(outside);
            let away = outside - closest;
            if away.length() > 1e-6 {
                away.normalized()
            } else {
                Vector3::UP
            }
        }
    }

    #[derive(Default)]
    struct FakeWorld {
        solids: Vec<Solid>,
    }

    impl FakeWorld {
        fn penetration(&self, center: Vector3, radius: f32) -> Option<Contact> {
            let mut deepest: Option<(f32, Contact)> = None;
            for solid in &self.solids {
                let point = solid.closest(center);
                let away = center - point;
                let distance = away.length();
                if distance >= radius {
                    continue;
                }
                let normal = if distance > 1e-6 {
                    away / distance
                } else {
                    Vector3::UP
                };
                let depth = radius - distance;
                if deepest.as_ref().is_none_or(|(d, _)| depth > *d) {
                    deepest = Some((depth, Contact { point, normal }));
                }
            }
            deepest.map(|(_, contact)| contact)
        }
    }

    impl ClampWorld for FakeWorld {
        fn ray(&mut self, from: Vector3, to: Vector3) -> Option<RayHit> {
            self.solids
                .iter()
                .filter_map(|solid| solid.ray(from, to))
                .min_by(|a, b| {
                    a.position
                        .distance_to(from)
                        .total_cmp(&b.position.distance_to(from))
                })
        }

        fn sweep_sphere_blocked_at(&mut self, center: Vector3) -> bool {
            self.penetration(center, CLAMP_SPHERE_RADIUS + CLAMP_NEAR_CLEARANCE)
                .is_some()
        }

        fn sweep_sphere(&mut self, origin: Vector3, motion: Vector3) -> Option<f32> {
            let steps = (motion.length() / 0.005).ceil() as usize;
            for i in 1..=steps {
                let fraction = i as f32 / steps as f32;
                if self
                    .penetration(
                        origin + motion * fraction,
                        CLAMP_SPHERE_RADIUS + CLAMP_NEAR_CLEARANCE,
                    )
                    .is_some()
                {
                    return Some((i - 1) as f32 / steps as f32);
                }
            }
            None
        }

        fn contact(&mut self, center: Vector3) -> Option<Contact> {
            self.penetration(center, CLAMP_CONTACT_RADIUS)
        }
    }

    fn boxed(center: Vector3, size: Vector3) -> Solid {
        Solid::Box { center, size }
    }

    /// The camera after the rig settles: the mount at the origin looking down -Z (rotated by
    /// `orbit_y`), the arm resolved against the same world along its own axis.
    fn settle(solids: Vec<Solid>, orbit_y: f32) -> Vector3 {
        let mut world = FakeWorld { solids };
        let mut state = ClampState {
            contact_offset: Vector3::ZERO,
            smoothed_dist: THIRD_PERSON_CAMERA_Z,
        };
        let basis = Basis::from_axis_angle(Vector3::UP, orbit_y);
        let back = basis.col_c();
        // The spring arm: its centered ray along the arm's axis shortens it.
        let arm_end = back * THIRD_PERSON_CAMERA_Z;
        let arm_z = match world.ray(Vector3::ZERO, arm_end) {
            Some(hit) => hit.position.length().min(THIRD_PERSON_CAMERA_Z),
            None => THIRD_PERSON_CAMERA_Z,
        };
        let target = basis * Vector3::new(LATERAL_OFFSET, 0.0, arm_z);
        let mut camera = Vector3::ZERO;
        for _ in 0..60 {
            camera = resolve_camera(
                &mut world,
                &mut state,
                Vector3::ZERO,
                target,
                back,
                1.0 / 60.0,
            )
            .expect("a segment");
        }
        camera
    }

    #[test]
    fn with_no_walls_the_full_offset_framing_is_kept() {
        let camera = settle(vec![], 0.0);
        assert!(
            camera.distance_to(Vector3::new(LATERAL_OFFSET, 0.0, 3.0)) < 0.05,
            "{camera:?}"
        );
    }

    #[test]
    fn a_wall_on_the_over_shoulder_side_pulls_the_camera_in_front_of_it() {
        // The arm's centered ray at x=0 misses the box x[0.5,1.5], z[2.5,3.5]; the unclamped
        // camera (0.75, 0, 3) is inside it.
        let camera = settle(
            vec![boxed(
                Vector3::new(1.0, 0.0, 3.0),
                Vector3::new(1.0, 2.0, 1.0),
            )],
            0.0,
        );
        assert!(camera.z < 2.2, "still inside the wall volume: {camera:?}");
        assert!(camera.z > 1.0, "over-shrunk: {camera:?}");
    }

    #[test]
    fn with_the_back_against_a_wall_the_camera_stays_out_of_it() {
        // Wall face at z=0.25 behind the pivot: a sphere cast from the pivot already overlaps it
        // and cast_motion would report all-clear; the shifted cast and the whiskers catch it.
        let camera = settle(
            vec![boxed(
                Vector3::new(0.0, 0.0, 0.35),
                Vector3::new(4.0, 4.0, 0.2),
            )],
            0.0,
        );
        assert!(camera.z <= 0.2, "inside the wall: {camera:?}");
        assert!(camera.z >= -0.6, "shot off too far forward: {camera:?}");
    }

    #[test]
    fn between_two_thin_walls_the_whiskers_keep_near_plane_clearance() {
        // W1 in front blinds the shifted sphere; W2 behind is the one the camera would cross.
        let camera = settle(
            vec![
                boxed(Vector3::new(0.0, 0.0, -0.35), Vector3::new(4.0, 4.0, 0.2)),
                boxed(Vector3::new(0.0, 0.0, 0.35), Vector3::new(4.0, 4.0, 0.2)),
            ],
            0.0,
        );
        let limit = 0.25 - CLAMP_NEAR_CLEARANCE + 0.01;
        assert!(
            camera.z <= limit,
            "no near-plane clearance: {camera:?} (z <= {limit})"
        );
    }

    #[test]
    fn orbiting_a_thin_door_never_puts_the_camera_on_its_far_side() {
        let camera = settle(
            vec![boxed(
                Vector3::new(0.0, 0.0, -0.35),
                Vector3::new(4.0, 4.0, 0.2),
            )],
            std::f32::consts::PI,
        );
        assert!(camera.z >= -0.24, "crossed the door: {camera:?}");
    }

    #[test]
    fn the_outer_whisker_catches_a_column_bulging_beside_the_line() {
        let camera = settle(
            vec![
                boxed(Vector3::new(0.0, 0.0, -0.35), Vector3::new(4.0, 4.0, 0.2)),
                Solid::Cylinder {
                    center: Vector3::new(1.3, 0.0, 3.0),
                    radius: 0.5,
                    height: 4.0,
                },
            ],
            0.0,
        );
        let to_column = Vector2::new(camera.x - 1.3, camera.z - 3.0).length();
        assert!(
            to_column >= 0.6,
            "too close to the curved surface: {camera:?} ({to_column})"
        );
    }

    #[test]
    fn the_contact_sphere_pushes_the_camera_off_a_parallel_wall_without_pulling_it_in() {
        let world = vec![
            // Blinds the sweep sphere.
            boxed(Vector3::new(0.0, 0.0, -0.35), Vector3::new(4.0, 4.0, 0.2)),
            // Thin parallel wall: every longitudinal cast slips past it.
            boxed(Vector3::new(0.945, 0.0, 4.25), Vector3::new(0.05, 4.0, 3.5)),
        ];
        let camera = settle(world, 0.0);
        assert!(camera.x <= 0.74, "rests at the wall: {camera:?}");
        assert!(camera.z >= 2.5, "wrongly pulled in: {camera:?}");
    }

    #[test]
    fn the_depenetrated_position_is_a_stable_fixed_point() {
        let mut world = FakeWorld {
            solids: vec![
                boxed(Vector3::new(0.0, 0.0, -0.35), Vector3::new(4.0, 4.0, 0.2)),
                boxed(Vector3::new(0.945, 0.0, 4.25), Vector3::new(0.05, 4.0, 3.5)),
            ],
        };
        let mut state = ClampState {
            contact_offset: Vector3::ZERO,
            smoothed_dist: THIRD_PERSON_CAMERA_Z,
        };
        let target = Vector3::new(LATERAL_OFFSET, 0.0, 3.0);
        let mut previous = Vector3::ZERO;
        let mut worst: f32 = 0.0;
        for frame in 0..80 {
            let camera = resolve_camera(
                &mut world,
                &mut state,
                Vector3::ZERO,
                target,
                Vector3::BACK,
                1.0 / 60.0,
            )
            .unwrap();
            if frame >= 60 {
                worst = worst.max(camera.distance_to(previous));
            }
            previous = camera;
        }
        assert!(worst < 0.05, "oscillates between frames: {worst}");
    }

    #[test]
    fn a_wall_right_behind_the_pivot_stops_the_camera_in_front_of_it() {
        // Face at z=0.5.
        let camera = settle(
            vec![boxed(
                Vector3::new(0.0, 0.0, 1.0),
                Vector3::new(4.0, 4.0, 1.0),
            )],
            0.0,
        );
        assert!(
            camera.z <= 0.35,
            "too close to or inside the wall: {camera:?}"
        );
    }

    // The scene wiring the physics depends on, read from the scene text.
    const PLAYER_SCENE: &str = include_str!("../../../../../godot/src/logic/player/player.tscn");

    fn node_block<'a>(text: &'a str, name: &str) -> &'a str {
        let marker = format!("[node name=\"{name}\"");
        let start = text.find(&marker).expect("node in the scene");
        let end = text[start + marker.len()..]
            .find("\n[node ")
            .map_or(text.len(), |at| start + marker.len() + at);
        &text[start..end]
    }

    #[test]
    fn the_scene_pivot_is_centered_and_masked_to_solid_geometry() {
        let mount = node_block(PLAYER_SCENE, "Mount");
        assert!(mount.contains(&format!("collision_mask = {CAMERA_COLLISION_MASK}")));
        // Pivot on the player axis: no lateral offset baked into it.
        assert!(mount.contains("0, 1.71, 0)"));
        assert!(
            PLAYER_SCENE.contains("name=\"CameraArm\"")
                && PLAYER_SCENE.contains("parent=\"Mount\"")
        );
        assert!(PLAYER_SCENE.contains("parent=\"Mount/CameraArm\""));
    }

    #[test]
    fn the_scene_has_the_clamp_under_the_mount() {
        assert!(node_block(PLAYER_SCENE, "CameraCollisionClamp").contains("parent=\"Mount\""));
    }
}
