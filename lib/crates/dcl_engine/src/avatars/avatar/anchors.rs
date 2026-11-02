//! Skeletal anchor points for `AvatarAttach`: the bone poses of the anchors in use, cached per
//! skeleton update.

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use godot::classes::Skeleton3D;
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;

/// `AvatarAnchorPointType` ids that are not skeletal: resolved directly.
const ANCHOR_POSITION: i32 = 0;
const ANCHOR_NAME_TAG: i32 = 1;

/// `AvatarAnchorPointType` (avatar_attach.proto) -> skeleton bone name. Ids 0 (POSITION) and
/// 1 (NAME_TAG) are not skeletal.
fn anchor_bone_name(anchor: i32) -> Option<&'static str> {
    Some(match anchor {
        2 => "Avatar_LeftHand",
        3 => "Avatar_RightHand",
        4 => "Avatar_Head",
        5 => "Avatar_Neck",
        6 => "Avatar_Spine",
        7 => "Avatar_Spine1",
        8 => "Avatar_Spine2",
        9 => "Avatar_Hips",
        10 => "Avatar_LeftShoulder",
        11 => "Avatar_LeftArm",
        12 => "Avatar_LeftForeArm",
        13 => "Avatar_LeftHandIndex1",
        14 => "Avatar_RightShoulder",
        15 => "Avatar_RightArm",
        16 => "Avatar_RightForeArm",
        17 => "Avatar_RightHandIndex1",
        18 => "Avatar_LeftUpLeg",
        19 => "Avatar_LeftLeg",
        20 => "Avatar_LeftFoot",
        21 => "Avatar_LeftToeBase",
        22 => "Avatar_RightUpLeg",
        23 => "Avatar_RightLeg",
        24 => "Avatar_RightFoot",
        25 => "Avatar_RightToeBase",
        _ => return None,
    })
}

/// One per attached anchor, created lazily and dropped with its last user.
pub struct AnchorState {
    bone_name: &'static str,
    bone_idx: i32,
    /// The bone pose with its basis scaled by 100, cancelling the skeleton's 0.01 unit scale.
    cached_transform: Transform3D,
    users: Vec<InstanceId>,
}

impl AnchorState {
    fn resolve(&mut self, skeleton: &Gd<Skeleton3D>) {
        self.bone_idx = skeleton.find_bone(self.bone_name);
        if self.bone_idx != -1 {
            self.refresh(skeleton);
        }
    }

    fn refresh(&mut self, skeleton: &Gd<Skeleton3D>) {
        if self.bone_idx == -1 {
            return;
        }
        let mut pose = skeleton.get_bone_global_pose(self.bone_idx);
        pose.basis = pose.basis.scaled(Vector3::ONE * 100.0);
        self.cached_transform = pose;
    }
}

pub type Anchors = HashMap<i32, AnchorState>;

impl DclAvatar {
    /// An `AvatarAttach` starts following a skeletal anchor; attachments on the same anchor
    /// share one state.
    pub(crate) fn add_anchor_user(&mut self, anchor_id: i32, attachment: InstanceId) {
        let skeleton = self.nodes.as_ref().map(|n| n.skeleton.clone());
        let state = match self.anchors.entry(anchor_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let Some(bone_name) = anchor_bone_name(anchor_id) else {
                    return;
                };
                let mut state = AnchorState {
                    bone_name,
                    bone_idx: -1,
                    cached_transform: Transform3D::IDENTITY,
                    users: Vec::new(),
                };
                if let Some(skeleton) = skeleton.as_ref() {
                    state.resolve(skeleton);
                }
                entry.insert(state)
            }
        };
        if !state.users.contains(&attachment) {
            state.users.push(attachment);
        }
    }

    /// An `AvatarAttach` stops following (freed, detached, anchor or user changed).
    pub(crate) fn remove_anchor_user(&mut self, anchor_id: i32, attachment: InstanceId) {
        let Some(state) = self.anchors.get_mut(&anchor_id) else {
            return;
        };
        state.users.retain(|u| *u != attachment);
        if state.users.is_empty() {
            self.anchors.remove(&anchor_id);
        }
    }

    /// Refreshes the poses of the anchors in use; connected to `skeleton_updated`.
    pub(crate) fn refresh_anchor_poses(&mut self) {
        let Some(skeleton) = self.nodes.as_ref().map(|n| n.skeleton.clone()) else {
            return;
        };
        for state in self.anchors.values_mut() {
            state.refresh(&skeleton);
        }
    }

    /// A wearable merge rebuilt the skeleton: re-resolve the bone indices.
    pub(crate) fn reresolve_active_anchors(&mut self) {
        let Some(skeleton) = self.nodes.as_ref().map(|n| n.skeleton.clone()) else {
            return;
        };
        for state in self.anchors.values_mut() {
            state.resolve(&skeleton);
        }
    }

    /// World transform of an anchor point for `AvatarAttach`. Post-rotated 180° around Y to
    /// align with the Unity reference: Godot's glTF import flips the skeleton basis, leaving
    /// bone-derived +X and +Z inverted against the SDK convention (+Y stays correct). Applies to
    /// NAME_TAG too, whose quad hangs under a bone attachment. Unknown or unresolved ids fall
    /// back to the avatar root, so attached entities stay on the avatar.
    pub fn get_anchor_point_global_transform(&self, anchor_id: i32) -> Transform3D {
        let flip = |t: Transform3D| t.rotated_local(Vector3::UP, std::f32::consts::PI);
        if anchor_id == ANCHOR_NAME_TAG {
            if let Some(nodes) = self.nodes.as_ref() {
                return flip(nodes.nickname_quad.get_global_transform());
            }
        }
        if anchor_id != ANCHOR_POSITION {
            if let (Some(state), Some(nodes)) = (self.anchors.get(&anchor_id), self.nodes.as_ref())
            {
                if state.bone_idx != -1 {
                    return flip(nodes.skeleton.get_global_transform() * state.cached_transform);
                }
            }
        }
        self.base().get_global_transform()
    }
}

#[cfg(test)]
mod tests {
    use super::anchor_bone_name;

    #[test]
    fn skeletal_anchors_have_bones() {
        assert_eq!(anchor_bone_name(4), Some("Avatar_Head"));
        assert_eq!(anchor_bone_name(25), Some("Avatar_RightToeBase"));
        assert_eq!(anchor_bone_name(0), None);
        assert_eq!(anchor_bone_name(1), None);
        assert_eq!(anchor_bone_name(26), None);
    }
}
