//! Feet, hands and head IK: Unity's per-rig gates and the frame calls. Visual only, bone
//! modifiers. A remote avatar gets the head alone, since Unity's hands and feet need the sender's
//! rigid transform, which a remote entity does not carry. Head sync leaves from here too: Unity's
//! HeadSync sends yaw and pitch, not a look-at vector.

pub mod feet;
pub mod hands;
pub mod head;

use godot::classes::Skeleton3D;
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;

/// Unity HeadIKRemotePlayersDistance: a remote head turn is dropped past this, so a crowd costs
/// nothing at distance.
const REMOTE_DISTANCE: f32 = 10.0;

const SIDES: [usize; 2] = [0, 1];

#[derive(Default)]
pub struct AvatarIk {
    feet: feet::FeetIk,
    hands: hands::HandsIk,
    head: head::HeadIk,
}

/// Writes a bone's pose rotation from a wanted world basis: world -> skeleton -> parent bone.
fn set_bone_world_rotation(
    skel: &mut Gd<Skeleton3D>,
    idx: i32,
    skel_basis: Basis,
    want_world: Basis,
) {
    let parent = skel.get_bone_parent(idx);
    let parent_skel = if parent >= 0 {
        skel.get_bone_global_pose(parent).basis
    } else {
        Basis::IDENTITY
    };
    let local = parent_skel.inverse() * (skel_basis.inverse() * want_world);
    skel.set_bone_pose_rotation(idx, local.get_quaternion());
}

/// The avatar's horizontal forward.
fn forward_xz(avatar_xf: &Transform3D) -> Vector3 {
    let mut forward = -avatar_xf.basis.col_c();
    forward.y = 0.0;
    forward.try_normalized().unwrap_or(Vector3::FORWARD)
}

impl DclAvatar {
    /// `rig_free` is the avatar's own gate (full LOD, no spawn reveal, loaded world, no emote).
    pub(crate) fn ik_process(&mut self, delta: f64, rig_free: bool, grounded: bool, stunned: bool) {
        let Some(skeleton) = self.nodes.as_ref().map(|n| n.skeleton.clone()) else {
            return;
        };
        let delta = delta as f32;
        // The rigs add nodes under the avatar through `base_mut`, so they borrow the avatar.
        let mut ik = std::mem::take(&mut self.ik);
        if self.is_local_player {
            // Unity gates feet and hands on the same rig state.
            let enabled = rig_free && grounded && !stunned;
            ik.feet.update(self, &skeleton, delta, enabled);
            ik.hands.update(self, &skeleton, delta, enabled);
            // The head looks where the view camera looks, and only while nearly standing still.
            let camera = self.base().get_viewport().and_then(|v| v.get_camera_3d());
            let head_on = enabled && self.movement_speed < head::MAX_SPEED && camera.is_some();
            let look = camera
                .map(|c| -c.get_global_transform().basis.col_c())
                .unwrap_or(Vector3::ZERO);
            ik.head.update_local(self, &skeleton, delta, head_on, look);
            // Mirrored onto the avatar so the sender's dirty check sees the turn while it stands.
            self.head_ik_yaw = ik.head.head_yaw_deg;
            self.head_ik_pitch = ik.head.head_pitch_deg;
            self.head_ik_yaw_enabled = head_on;
            self.head_ik_pitch_enabled = head_on;
            globals::comms().bind_mut().set_head_ik(
                head_on,
                ik.head.head_yaw_deg,
                ik.head.head_pitch_deg,
            );
        } else {
            // A remote head comes over the wire, never from our camera.
            let head_on = (self.head_ik_yaw_enabled || self.head_ik_pitch_enabled)
                && rig_free
                && !stunned
                && self.lod.distance <= REMOTE_DISTANCE;
            let (yaw, pitch) = (self.head_ik_yaw, self.head_ik_pitch);
            ik.head
                .update_remote(self, &skeleton, delta, head_on, yaw, pitch);
        }
        self.ik = ik;
    }
}
