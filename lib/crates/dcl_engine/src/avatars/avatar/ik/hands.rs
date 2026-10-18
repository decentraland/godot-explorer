//! audit #905 / Unity HandsIKSystem: a hand reaching a wall plants on it (the player can lean on
//! geometry), elbow hinted so it bends outward. One RayCast3D per hand, swept on demand because
//! this runs in idle; never a direct space query from the avatar node (those break is_on_floor).

use godot::classes::skeleton_modifier_3d::BoneDirection;
use godot::classes::{
    ISkeletonModifier3D, Marker3D, RayCast3D, Skeleton3D, SkeletonModifier3D, TwoBoneIk3d,
};
use godot::prelude::*;

use super::{set_bone_world_rotation, SIDES};
use crate::avatars::collision_layers::CL_PHYSICS;
use crate::godot_classes::dcl_avatar::DclAvatar;

const UPPER_ARM: [&str; 2] = ["Avatar_LeftArm", "Avatar_RightArm"];
const FOREARM: [&str; 2] = ["Avatar_LeftForeArm", "Avatar_RightForeArm"];
const HAND: [&str; 2] = ["Avatar_LeftHand", "Avatar_RightHand"];
// The palm frame comes from the rig itself: the knuckle and thumb roots lie in the palm plane, so
// their cross product is the palm's normal, mirrored per side.
const KNUCKLE: [&str; 2] = ["Avatar_LeftHandIndex1", "Avatar_RightHandIndex1"];
const THUMB: [&str; 2] = ["Avatar_LeftHandThumb1", "Avatar_RightHandThumb1"];

const WALL_DISTANCE: f32 = 0.5; // HandsIKWallHitDistance
const WEIGHT_SPEED: f32 = 5.0; // HandsIKWeightSpeed
const POS_SPEED: f32 = 10.0; // IKPositionSpeed (m/s)
                             // Unity probes from a fixed point in avatar space (LeftHandWallRaycast at -0.205,
                             // RightHandWallRaycast at +0.205, both y=1.369) straight along the character's forward — NOT along
                             // the arm, which points at the floor whenever the arms hang.
const PROBE_ORIGIN: Vector3 = Vector3::new(0.205, 1.369, 0.0);
// HandsIKElbowOffset, x negated on the left. Unity is +Z forward, Godot -Z, so the depth flips sign.
const ELBOW_OFFSET: Vector3 = Vector3::new(0.256, 0.980_012_9, 0.461_001_8);

/// Unity's constraint gives the hand the target's rotation outright (`handIKTarget.forward =
/// -hit.normal`: the hand lies flat on the wall). TwoBoneIK3D never applies the target's rotation,
/// so this does, from each hand's own rest palm frame: the two bones carry mirrored local axes.
#[derive(GodotClass)]
#[class(base=SkeletonModifier3D, init)]
pub struct HandAlignModifier3D {
    base: Base<SkeletonModifier3D>,
    bones: [i32; 2],
    /// Bone-local palm frame (palm normal, knuckle axis, their cross).
    frames: [Option<Basis>; 2],
    /// ZERO = no wall.
    normals: [Vector3; 2],
    weights: [f32; 2],
}

#[godot_api]
impl ISkeletonModifier3D for HandAlignModifier3D {
    fn process_modification(&mut self) {
        let Some(mut skel) = self.base().get_skeleton() else {
            return;
        };
        let skel_basis = skel.get_global_transform().basis.orthonormalized();
        for side in SIDES {
            let (w, Some(frame)) = (self.weights[side], self.frames[side]) else {
                continue;
            };
            let Some(into) = (-self.normals[side]).try_normalized() else {
                continue;
            };
            if w <= 0.0 {
                continue;
            }
            // Knuckles run up the wall: the residual of world up across the wall plane, which is
            // also what stops the hand rolling.
            let residual = Vector3::UP - into * Vector3::UP.dot(into);
            if residual.length_squared() < 0.0001 {
                continue;
            }
            let knuckles = residual.normalized();
            let want_world =
                Basis::from_cols(into, knuckles, into.cross(knuckles)) * frame.inverse();
            let idx = self.bones[side];
            let cur_world = (skel_basis * skel.get_bone_global_pose(idx).basis).orthonormalized();
            let mixed = cur_world.slerp(&want_world.orthonormalized(), w);
            set_bone_world_rotation(&mut skel, idx, skel_basis, mixed);
        }
    }
}

struct HandsRig {
    targets: [Gd<Marker3D>; 2],
    poles: [Gd<Marker3D>; 2],
    rays: [Gd<RayCast3D>; 2],
    chains: [Gd<TwoBoneIk3d>; 2],
    align: Gd<HandAlignModifier3D>,
}

#[derive(Default)]
pub struct HandsIk {
    rig: Option<HandsRig>,
    influence: [f32; 2],
}

fn mirrored(v: Vector3, side: usize) -> Vector3 {
    if side == 0 {
        Vector3::new(-v.x, v.y, v.z)
    } else {
        v
    }
}

impl HandsIk {
    /// Per-frame entry; `enabled` is false whenever IK must not apply at all.
    pub fn update(
        &mut self,
        avatar: &mut DclAvatar,
        skeleton: &Gd<Skeleton3D>,
        delta: f32,
        enabled: bool,
    ) {
        if self.rig.is_none() {
            if !enabled {
                return;
            }
            self.rig = Some(build(avatar, skeleton));
        }
        let Some(rig) = self.rig.as_mut() else {
            return;
        };
        let avatar_xf = avatar.base().get_global_transform();
        for side in SIDES {
            // Unity parks the elbow hint at a fixed point in the avatar's own space.
            rig.poles[side].set_global_position(avatar_xf * mirrored(ELBOW_OFFSET, side));
            // Probe straight ahead from the fixed hand-height point Unity uses, so the hand plants
            // on a wall the player walks into.
            let ray = &mut rig.rays[side];
            let origin = avatar_xf * mirrored(PROBE_ORIGIN, side);
            let forward = -avatar_xf.basis.col_c();
            ray.set_global_transform(Transform3D::new(Basis::IDENTITY, origin));
            ray.set_target_position(forward.normalized_or_zero() * WALL_DISTANCE);
            ray.force_raycast_update();
            let mut w = 0.0;
            let mut normal = Vector3::ZERO;
            if enabled && ray.is_colliding() {
                let hit = ray.get_collision_point();
                normal = ray.get_collision_normal();
                let target = &mut rig.targets[side];
                let current = target.get_global_position();
                target.set_global_position(current.move_toward(hit, POS_SPEED * delta));
                w = 1.0;
            }
            self.influence[side] = if enabled {
                let step = WEIGHT_SPEED * delta;
                let current = self.influence[side];
                if (w - current).abs() <= step {
                    w
                } else {
                    current + (w - current).signum() * step
                }
            } else {
                0.0
            };
            rig.chains[side].set_influence(self.influence[side]);
            let mut align = rig.align.bind_mut();
            align.normals[side] = normal;
            align.weights[side] = self.influence[side];
        }
    }
}

/// Rest position of `bone` in `ancestor`'s local space.
fn rest_origin_in(skeleton: &Gd<Skeleton3D>, ancestor: i32, bone: i32) -> Option<Vector3> {
    let mut chain = Vec::new();
    let mut i = bone;
    while i >= 0 && i != ancestor {
        chain.push(i);
        i = skeleton.get_bone_parent(i);
    }
    if i < 0 {
        return None;
    }
    let xform = chain.iter().rev().fold(Transform3D::IDENTITY, |acc, b| {
        acc * skeleton.get_bone_rest(*b)
    });
    Some(xform.origin)
}

/// The bone's rest orientation in world space: the frame the rest-derived signs are read in.
fn rest_world_basis(skeleton: &Gd<Skeleton3D>, bone: i32) -> Basis {
    let mut chain = Vec::new();
    let mut i = bone;
    while i >= 0 {
        chain.push(i);
        i = skeleton.get_bone_parent(i);
    }
    let rest = chain.iter().rev().fold(Basis::IDENTITY, |acc, b| {
        acc * skeleton.get_bone_rest(*b).basis
    });
    (skeleton.get_global_transform().basis * rest).orthonormalized()
}

/// The hand's rest frame in the hand bone's own space: X = palm normal (out of the palm), Y =
/// knuckle axis, Z = their cross product.
fn rest_palm_frame(skeleton: &Gd<Skeleton3D>, side: usize, hand: i32) -> Option<Basis> {
    let knuckle = skeleton.find_bone(KNUCKLE[side]);
    let thumb = skeleton.find_bone(THUMB[side]);
    if hand < 0 || knuckle < 0 || thumb < 0 {
        return None;
    }
    let to_knuckle = rest_origin_in(skeleton, hand, knuckle)?.try_normalized()?;
    let to_thumb = rest_origin_in(skeleton, hand, thumb)?.try_normalized()?;
    let mut palm = to_knuckle.cross(to_thumb).try_normalized()?;
    let knuckles = (to_knuckle - palm * to_knuckle.dot(palm)).try_normalized()?;
    // The cross product flips sense on the mirrored side, so pin the sign to the bind pose's
    // convention (both palms face down there) instead of to a side.
    if (rest_world_basis(skeleton, hand) * palm).dot(Vector3::DOWN) < 0.0 {
        palm = -palm;
    }
    Some(Basis::from_cols(palm, knuckles, palm.cross(knuckles)))
}

fn build(avatar: &mut DclAvatar, skeleton: &Gd<Skeleton3D>) -> HandsRig {
    let mut skeleton = skeleton.clone();
    let names = ["Left", "Right"];
    let mut hands = [-1; 2];
    let mut frames = [None; 2];
    let mut make = |side: usize| {
        let name = names[side];
        let mut target = Marker3D::new_alloc();
        target.set_name(&format!("IKHandTarget{name}"));
        avatar.base_mut().add_child(&target);
        let mut pole = Marker3D::new_alloc();
        pole.set_name(&format!("IKHandPole{name}"));
        avatar.base_mut().add_child(&pole);
        let mut ray = RayCast3D::new_alloc();
        ray.set_name(&format!("IKHandRay{name}"));
        ray.set_collision_mask(CL_PHYSICS);
        // Every read follows a forced update, so the engine never needs to sweep it.
        ray.set_enabled(false);
        avatar.base_mut().add_child(&ray);
        let mut chain = TwoBoneIk3d::new_alloc();
        chain.set_name(&format!("IKHand{name}"));
        skeleton.add_child(&chain);
        chain.set_setting_count(1);
        // Without mutable axes the solver only turns the arm around its primary axis and cannot
        // reach a wall beside it.
        chain.set_mutable_bone_axes(true);
        chain.set_root_bone_name(0, UPPER_ARM[side]);
        chain.set_middle_bone_name(0, FOREARM[side]);
        chain.set_end_bone_name(0, HAND[side]);
        chain.set_end_bone_direction(0, BoneDirection::FROM_PARENT);
        let target_path = chain.get_path_to(&target);
        chain.set_target_node(0, &target_path);
        let pole_path = chain.get_path_to(&pole);
        chain.set_pole_node(0, &pole_path);
        chain.set_influence(0.0);
        let hand = skeleton.find_bone(HAND[side]);
        hands[side] = hand;
        frames[side] = rest_palm_frame(&skeleton, side, hand);
        // Seed the target at the hand so the first solved frame has no snap.
        target.set_global_position(
            skeleton.get_global_transform() * skeleton.get_bone_global_pose(hand).origin,
        );
        (target, pole, ray, chain)
    };
    let (t0, p0, r0, c0) = make(0);
    let (t1, p1, r1, c1) = make(1);
    // Added last so it runs after both chains and aligns the solved hands.
    let mut align = HandAlignModifier3D::new_alloc();
    align.set_name("IKHandAlign");
    {
        let mut a = align.bind_mut();
        a.bones = hands;
        a.frames = frames;
    }
    skeleton.add_child(&align);
    HandsRig {
        targets: [t0, t1],
        poles: [p0, p1],
        rays: [r0, r1],
        chains: [c0, c1],
        align,
    }
}
