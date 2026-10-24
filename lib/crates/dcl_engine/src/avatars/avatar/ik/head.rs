//! audit #905 / Unity HeadIKSystem + ApplyHeadLookAt: while the avatar stands still it turns its
//! head toward the camera, clamped per axis, and eases back frontal when the direction falls
//! behind the shoulders. Bone-only, as a delta on top of the animated pose.

use godot::classes::{ISkeletonModifier3D, Skeleton3D, SkeletonModifier3D};
use godot::prelude::*;

use super::set_bone_world_rotation;
use crate::godot_classes::dcl_avatar::DclAvatar;

const HEAD: &str = "Avatar_Head";

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

/// Added last on the skeleton: rotates the head's world pose so the turn reads on top of whatever
/// the clip did with it.
#[derive(GodotClass)]
#[class(base=SkeletonModifier3D, init)]
pub struct HeadTurnModifier3D {
    base: Base<SkeletonModifier3D>,
    #[init(val = -1)]
    bone: i32,
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
        if self.bone < 0 || self.weight <= 0.0 || !skel.is_inside_tree() {
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
        let Some(cur_world) =
            super::orthonormal(skel_basis * skel.get_bone_global_pose(self.bone).basis)
        else {
            return;
        };
        let Some(want_world) = super::orthonormal(delta * cur_world) else {
            return;
        };
        let mixed = cur_world.slerp(&want_world, self.weight);
        set_bone_world_rotation(&mut skel, self.bone, skel_basis, mixed);
    }
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

#[derive(Default)]
pub struct HeadIk {
    modifier: Option<Gd<HeadTurnModifier3D>>,
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
            modifier.bind_mut().bone = skeleton.find_bone(HEAD);
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

    /// Remote avatar: the turn arrives over the wire as the sender's own head angles (Unity
    /// UpdateRemoteIK), already in the avatar's frame. Same limits and easing as the local path.
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
}
