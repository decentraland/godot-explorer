//! audit #905 / Unity HeadIKSystem + ApplyHeadLookAt: while the avatar stands still it turns its
//! head toward the camera, clamped per axis, and eases back frontal when the direction falls
//! behind the shoulders. Bone-only, as a delta on top of the animated pose.

use godot::classes::{ISkeletonModifier3D, Skeleton3D, SkeletonModifier3D};
use godot::prelude::*;

use super::set_bone_world_rotation;
use crate::godot_classes::dcl_avatar::DclAvatar;

const HEAD: &str = "Avatar_Head";
const NECK: &str = "Avatar_Neck";

const WEIGHT_SPEED: f32 = 2.0; // HeadIKWeightChangeSpeed
const ROTATION_SPEED: f32 = 360.0; // HeadIKRotationSpeed (deg/s)
const HORIZONTAL_LIMIT: f32 = 50.0; // HeadIKHorizontalAngleLimit
const VERTICAL_LIMIT: f32 = 60.0; // HeadIKVerticalAngleLimit
                                  // Beyond this the direction is behind the shoulders: look frontal instead, at a third of the
                                  // speed (Unity scales maxDeltaDegrees by 1/3).
const HORIZONTAL_RESET: f32 = 120.0; // HeadIKHorizontalAngleReset
const RESET_SPEED_SCALE: f32 = 0.333_333_3;
/// Unity gates on MoveVelocity.sqrMagnitude > 0.5 while running, so the speed that turns head IK
/// off is its square root.
pub const MAX_SPEED: f32 = std::f32::consts::FRAC_1_SQRT_2;
const YAW_DEAD_ZONE: f32 = 0.05;
/// Unity spreads the turn with TwistChainConstraints: the neck sits at ~0.89 of the vertical chain
/// and ~0.94 of the horizontal one, and the head holds the tip, i.e. the full aim. Rotating the
/// head alone leaves the whole angle at one joint and the neck snaps.
const NECK_SHARE: f32 = 0.89;

/// Added last on the skeleton: reshapes the animated pose so the aim is spread over Neck > Head
/// instead of piling it all on the head.
#[derive(GodotClass)]
#[class(base=SkeletonModifier3D, init)]
pub struct HeadTurnModifier3D {
    base: Base<SkeletonModifier3D>,
    #[init(val = -1)]
    neck: i32,
    #[init(val = -1)]
    head: i32,
    /// Radians, + = the character turns to its own right.
    yaw: f32,
    /// Radians, + = up.
    pitch: f32,
    weight: f32,
    /// The avatar's own frame: Unity builds this rotation on a rig node that inherits the avatar,
    /// so its axes are the body's. The world right pitches the head sideways facing east/west.
    #[init(val = Basis::IDENTITY)]
    frame: Basis,
}

#[godot_api]
impl ISkeletonModifier3D for HeadTurnModifier3D {
    fn process_modification(&mut self) {
        let Some(mut skel) = self.base().get_skeleton() else {
            return;
        };
        if self.head < 0 || self.weight <= 0.0 || !skel.is_inside_tree() {
            return;
        }
        let Some(skel_basis) = super::orthonormal(skel.get_global_transform().basis) else {
            return;
        };
        // Yaw about the body's up, then pitch about the body's right, the order Unity composes
        // (horizontalTargetRotation * AxisAngle(right, ..)).
        let delta_local = Basis::from_axis_angle(Vector3::UP, -self.yaw)
            * Basis::from_axis_angle(Vector3::RIGHT, self.pitch);
        let delta = self.frame * delta_local * self.frame.inverse();
        let Some(head_base) =
            super::orthonormal(skel_basis * skel.get_bone_global_pose(self.head).basis)
        else {
            return;
        };
        let has_neck = self.neck >= 0 && skel.get_bone_parent(self.head) == self.neck;
        let neck_base = if has_neck {
            super::orthonormal(skel_basis * skel.get_bone_global_pose(self.neck).basis)
        } else {
            None
        };
        let (neck_world, head_world) = spread_aim(neck_base, head_base, delta, self.weight);
        // The head's parent must already carry the neck's new pose.
        if let Some(neck_world) = neck_world {
            set_bone_world_rotation(&mut skel, self.neck, skel_basis, neck_world);
        }
        set_bone_world_rotation(&mut skel, self.head, skel_basis, head_world);
    }
}

/// The world poses after applying `delta` at `weight`: the head takes the full aim, the neck
/// (when the head hangs off it) `NECK_SHARE` of it.
fn spread_aim(
    neck_base: Option<Basis>,
    head_base: Basis,
    delta: Basis,
    weight: f32,
) -> (Option<Basis>, Basis) {
    let head = head_base.slerp(&(delta * head_base), weight);
    let neck = neck_base.map(|base| {
        let share =
            Basis::from_quaternion(Quaternion::IDENTITY.slerp(delta.get_quaternion(), NECK_SHARE));
        base.slerp(&(share * base), weight)
    });
    (neck, head)
}

fn move_toward(from: f32, to: f32, delta: f32) -> f32 {
    if (to - from).abs() <= delta {
        to
    } else {
        from + (to - from).signum() * delta
    }
}

/// The aim for a requested yaw/pitch (degrees): clamped, or frontal at a third of the speed when
/// the direction is behind the shoulders. Returns `(yaw, pitch, speed)`.
fn clamp_aim(yaw: f32, pitch: f32) -> (f32, f32, f32) {
    if yaw.abs() > HORIZONTAL_RESET {
        (0.0, 0.0, ROTATION_SPEED * RESET_SPEED_SCALE)
    } else {
        (
            yaw.clamp(-HORIZONTAL_LIMIT, HORIZONTAL_LIMIT),
            pitch.clamp(-VERTICAL_LIMIT, VERTICAL_LIMIT),
            ROTATION_SPEED,
        )
    }
}

/// The wire carries the sender's head aim in world space as Unity Euler yaw/pitch
/// (HeadIKComponent.GetHeadYawAndPitch over Camera.transform.forward), which Unity's remote IK
/// compares against the avatar's own head forward. The heading is the body's plus the aim the
/// local path already froze, never re-derived from the camera: first person clamps pitch at
/// +-90, where the horizontal projection vanishes and atan2 flips 180 degrees.
pub fn body_to_wire(yaw_deg: f32, pitch_deg: f32, avatar_basis: Basis) -> (f32, f32) {
    let body_yaw = (-avatar_basis.col_c().x)
        .atan2(avatar_basis.col_c().z)
        .to_degrees();
    (wrap_degrees(body_yaw + yaw_deg), pitch_deg)
}

/// Inverse of `body_to_wire`: wire angles to the avatar's own frame, which the modifier and the
/// per-axis limits use.
pub fn wire_to_body(yaw_deg: f32, pitch_deg: f32, avatar_basis: Basis) -> (f32, f32) {
    let yaw = wrap_degrees(yaw_deg).to_radians();
    let pitch = wrap_degrees(pitch_deg).clamp(-90.0, 90.0).to_radians();
    // DCL +Z is Godot -Z (to_godot_transform_3d flips z), hence the negated z.
    let world_dir = Vector3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );
    let basis = super::orthonormal(avatar_basis).unwrap_or(Basis::IDENTITY);
    let dir = basis.inverse() * world_dir;
    let horizontal = (dir.x * dir.x + dir.z * dir.z).sqrt();
    (
        dir.x.atan2(-dir.z).to_degrees(),
        dir.y.atan2(horizontal).to_degrees(),
    )
}

/// Godot's `wrapf(x, -180, 180)`: the range is half-open, +180 reads as -180.
fn wrap_degrees(deg: f32) -> f32 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}

#[derive(Default)]
pub struct HeadIk {
    modifier: Option<Gd<HeadTurnModifier3D>>,
    /// The last heading a remote head was aimed at, held while the wire's pitch is vertical.
    remote_yaw_deg: f32,
    /// The angles the local player last AIMED at, in degrees, before the limits and the easing:
    /// what Unity broadcasts (HeadIKComponent.GetHeadYawAndPitch).
    pub head_yaw_deg: f32,
    pub head_pitch_deg: f32,
}

impl HeadIk {
    fn modifier(
        &mut self,
        skeleton: &Gd<Skeleton3D>,
        enabled: bool,
    ) -> Option<Gd<HeadTurnModifier3D>> {
        if self.modifier.is_none() {
            if !enabled || !super::skeleton_usable(skeleton) {
                return None;
            }
            let mut modifier = HeadTurnModifier3D::new_alloc();
            modifier.set_name("IKHeadTurn");
            {
                let mut m = modifier.bind_mut();
                m.neck = skeleton.find_bone(NECK);
                m.head = skeleton.find_bone(HEAD);
            }
            skeleton.clone().add_child(&modifier);
            self.modifier = Some(modifier);
        }
        self.modifier.clone()
    }

    /// Local player: `look_dir` is the view camera's forward.
    pub fn update_local(
        &mut self,
        avatar: &DclAvatar,
        skeleton: &Gd<Skeleton3D>,
        delta: f32,
        enabled: bool,
        look_dir: Vector3,
    ) {
        let Some(mut modifier) = self.modifier(skeleton, enabled) else {
            return;
        };
        let mut m = modifier.bind_mut();
        m.weight = move_toward(
            m.weight,
            if enabled { 1.0 } else { 0.0 },
            WEIGHT_SPEED * delta,
        );
        // While the weight is on its way out, keep looking where we were: the blend fades.
        if m.weight <= 0.0 || !enabled {
            return;
        }
        let Some(basis) = super::orthonormal(avatar.base().get_global_transform().basis) else {
            return;
        };
        m.frame = basis;
        let dir = (basis.inverse() * look_dir).normalized_or_zero();
        let horizontal = (dir.x * dir.x + dir.z * dir.z).sqrt();
        let mut yaw = dir.x.atan2(-dir.z).to_degrees();
        let pitch = dir.y.atan2(horizontal).to_degrees();
        if horizontal < YAW_DEAD_ZONE {
            // Camera pitched near vertical: keep the heading, track the pitch only.
            yaw = m.yaw.to_degrees();
        }
        self.head_yaw_deg = yaw;
        self.head_pitch_deg = pitch;
        let (yaw, pitch, speed) = clamp_aim(yaw, pitch);
        let step = speed.to_radians() * delta;
        m.yaw = move_toward(m.yaw, yaw.to_radians(), step);
        m.pitch = move_toward(m.pitch, pitch.to_radians(), step);
    }

    /// Remote avatar: the turn arrives over the wire as the sender's head angles (Unity
    /// UpdateRemoteIK), already converted to this avatar's frame by `wire_to_body`. Same limits
    /// and easing as the local path.
    pub fn update_remote(
        &mut self,
        avatar: &DclAvatar,
        skeleton: &Gd<Skeleton3D>,
        delta: f32,
        enabled: bool,
        yaw_deg: f32,
        pitch_deg: f32,
    ) {
        let Some(mut modifier) = self.modifier(skeleton, enabled) else {
            return;
        };
        let mut m = modifier.bind_mut();
        m.weight = move_toward(
            m.weight,
            if enabled { 1.0 } else { 0.0 },
            WEIGHT_SPEED * delta,
        );
        if m.weight <= 0.0 || !enabled {
            return;
        }
        let Some(frame) = super::orthonormal(avatar.base().get_global_transform().basis) else {
            return;
        };
        m.frame = frame;
        // At a vertical aim the wire's heading is not recoverable: hold the last one instead of
        // snapping frontal.
        let yaw_deg = if pitch_deg.to_radians().cos().abs() < YAW_DEAD_ZONE {
            self.remote_yaw_deg
        } else {
            self.remote_yaw_deg = yaw_deg;
            yaw_deg
        };
        let (yaw, pitch, speed) = clamp_aim(yaw_deg, pitch_deg);
        let step = speed.to_radians() * delta;
        m.yaw = move_toward(m.yaw, yaw.to_radians(), step);
        m.pitch = move_toward(m.pitch, pitch.to_radians(), step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aim_is_clamped_per_axis() {
        assert_eq!(clamp_aim(70.0, -80.0), (50.0, -60.0, ROTATION_SPEED));
        assert_eq!(clamp_aim(-30.0, 10.0), (-30.0, 10.0, ROTATION_SPEED));
    }

    #[test]
    fn behind_the_shoulders_eases_frontal_slower() {
        let (yaw, pitch, speed) = clamp_aim(150.0, 20.0);
        assert_eq!((yaw, pitch), (0.0, 0.0));
        assert!((speed - 120.0).abs() < 1e-3);
    }

    const TOL: f32 = 0.01;
    // Faces Godot +X: its forward (-Z) maps to world +X.
    fn body_east() -> Basis {
        Basis::from_axis_angle(Vector3::UP, -std::f32::consts::FRAC_PI_2)
    }

    fn near(got: (f32, f32), want: (f32, f32)) {
        assert!(
            (got.0 - want.0).abs() < TOL && (got.1 - want.1).abs() < TOL,
            "got {got:?}, want {want:?}"
        );
    }

    #[test]
    fn send_adds_the_body_heading() {
        near(body_to_wire(0.0, 0.0, Basis::IDENTITY), (0.0, 0.0));
        near(body_to_wire(0.0, 0.0, body_east()), (90.0, 0.0));
        near(body_to_wire(20.0, 0.0, body_east()), (110.0, 0.0));
        near(body_to_wire(-20.0, 0.0, body_east()), (70.0, 0.0));
        near(body_to_wire(0.0, 45.0, body_east()), (90.0, 45.0));
        near(body_to_wire(170.0, 0.0, body_east()), (-100.0, 0.0));
    }

    #[test]
    fn send_keeps_the_heading_at_a_vertical_aim() {
        near(body_to_wire(0.0, 90.0, body_east()), (90.0, 90.0));
        near(body_to_wire(0.0, -90.0, body_east()), (90.0, -90.0));
    }

    /// Unity rebuilds the direction from the wire pair (Euler(pitch, yaw, 0) * forward in its
    /// left-handed space, wire pitch positive down); it must be where the sender really looks.
    fn assert_unity_agrees(body: Basis, aim: (f32, f32)) {
        let (yaw, pitch) = (aim.0.to_radians(), aim.1.to_radians());
        let local = Vector3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let world = body * local;
        let dcl_dir = Vector3::new(world.x, world.y, -world.z).normalized();
        let wire = body_to_wire(aim.0, aim.1, body);
        let (wire_yaw, wire_pitch) = (wire.0.to_radians(), (-wire.1).to_radians());
        let unity_dir = Vector3::new(
            wire_yaw.sin() * wire_pitch.cos(),
            -wire_pitch.sin(),
            wire_yaw.cos() * wire_pitch.cos(),
        );
        assert!(
            (unity_dir - dcl_dir).length() < TOL,
            "{unity_dir:?} vs {dcl_dir:?}"
        );
    }

    #[test]
    fn unity_decodes_the_camera_direction() {
        assert_unity_agrees(body_east(), (0.0, 0.0));
        assert_unity_agrees(body_east(), (35.0, 20.0));
        assert_unity_agrees(body_east(), (37.0, 83.0));
        assert_unity_agrees(
            Basis::from_axis_angle(Vector3::UP, std::f32::consts::PI),
            (-45.0, -30.0),
        );
    }

    #[test]
    fn receive_projects_into_the_avatar_frame() {
        near(wire_to_body(0.0, 0.0, body_east()), (-90.0, 0.0));
        near(wire_to_body(90.0, 0.0, body_east()), (0.0, 0.0));
        near(wire_to_body(180.0, 0.0, body_east()), (90.0, 0.0));
        near(wire_to_body(90.0, 60.0, body_east()), (0.0, 60.0));
        near(wire_to_body(30.0, 20.0, Basis::IDENTITY), (30.0, 20.0));
        // Unity sends [0, 360): a small left turn must not read as behind the shoulders.
        near(wire_to_body(350.0, 0.0, Basis::IDENTITY), (-10.0, 0.0));
        near(wire_to_body(180.0, 0.0, Basis::IDENTITY), (180.0, 0.0));
    }

    #[test]
    fn wire_round_trip_recovers_the_local_aim() {
        for (body, aim) in [
            (body_east(), (0.0, 0.0)),
            (body_east(), (25.0, 15.0)),
            (body_east(), (-25.0, -15.0)),
            (body_east(), (10.0, 80.0)),
            (Basis::IDENTITY, (-60.0, 30.0)),
        ] {
            let wire = body_to_wire(aim.0, aim.1, body);
            near(wire_to_body(wire.0, wire.1, body), aim);
        }
    }
}

/// Quaternion and slerp math go through the engine, so these run in the Godot-hosted `--itest`.
mod itests {
    use super::*;
    use crate::framework::TestContext;

    fn angle_deg(b: Basis) -> f32 {
        Quaternion::IDENTITY
            .angle_to(b.get_quaternion())
            .to_degrees()
    }

    fn settled_split(yaw_deg: f32, pitch_deg: f32) -> (f32, f32, f32) {
        let avatar = Basis::from_axis_angle(Vector3::UP, std::f32::consts::FRAC_PI_2);
        // The DCL rig: Spine2 carries a -90 deg X rest, Neck and Head are identity locals.
        let neck_rest =
            avatar * Basis::from_axis_angle(Vector3::RIGHT, -std::f32::consts::FRAC_PI_2);
        let head_rest = neck_rest;
        let (yaw, pitch, _) = clamp_aim(yaw_deg, pitch_deg);
        let local = Basis::from_axis_angle(Vector3::UP, -yaw.to_radians())
            * Basis::from_axis_angle(Vector3::RIGHT, pitch.to_radians());
        let delta = avatar * local * avatar.inverse();
        let (neck, head) = spread_aim(Some(neck_rest), head_rest, delta, 1.0);
        let neck = neck.expect("the head hangs off the neck");
        (
            angle_deg(neck_rest.inverse() * neck),
            angle_deg(head_rest.inverse() * head),
            angle_deg(neck.inverse() * head),
        )
    }

    #[godot::test::itest]
    fn extreme_pitch_bends_the_neck_not_just_the_joint(_ctx: &TestContext) {
        for pitch in [-60.0, 60.0] {
            let (neck, head, joint) = settled_split(0.0, pitch);
            assert!((head - 60.0).abs() < 1.0, "head aim {head}");
            assert!(neck > 30.0, "neck turned {neck}");
            assert!(joint < 20.0, "joint bent {joint}");
        }
        let (neck, head, joint) = settled_split(50.0, 0.0);
        assert!((head - 50.0).abs() < 1.0 && neck > 30.0 && joint < 20.0);
    }

    #[godot::test::itest]
    fn behind_the_shoulders_leaves_the_neck_at_rest(_ctx: &TestContext) {
        let (neck, head, _) = settled_split(179.0, 0.0);
        assert!(neck < 1.0 && head < 1.0);
    }
}
