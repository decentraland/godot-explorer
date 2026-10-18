//! audit #905 / Unity FeetIKSystem: plants each animated foot on the ground under THAT foot, so
//! feet follow slopes, steps and curbs. The weight comes from the clip, so only the planted foot
//! is solved. The probe is a ShapeCast3D per foot, never a direct space query from the avatar
//! node (it breaks is_on_floor).

use godot::classes::skeleton_modifier_3d::BoneDirection;
use godot::classes::{
    ISkeletonModifier3D, Marker3D, Shape3D, ShapeCast3D, Skeleton3D, SkeletonModifier3D,
    SphereShape3D, TwoBoneIk3d,
};
use godot::prelude::*;

use super::{forward_xz, set_bone_world_rotation, SIDES};
use crate::avatars::collision_layers::CL_PHYSICS;
use crate::godot_classes::dcl_avatar::DclAvatar;

const THIGH: [&str; 2] = ["Avatar_LeftUpLeg", "Avatar_RightUpLeg"];
const SHIN: [&str; 2] = ["Avatar_LeftLeg", "Avatar_RightLeg"];
const FOOT: [&str; 2] = ["Avatar_LeftFoot", "Avatar_RightFoot"];

const PULL_MAX: f32 = 0.6; // FeetIKHipsPullMaxDistance: ray origin above the foot
const CAST_LENGTH: f32 = 1.2; // cast reach, Unity parity
const WEIGHT_SPEED: f32 = 10.0; // IKWeightSpeed (rise only; release is instant)
const POS_SPEED: f32 = 10.0; // IKPositionSpeed (m/s)
const HEIGHT_CORRECTION: f32 = 0.08; // FeetHeightCorrection — idle ankle height
const SPHERE_RADIUS: f32 = 0.08; // FeetIKSphereSize — probe radius
const DISABLE_DIST: f32 = 0.1; // FeetHeightDisableIkDistance — clip-driven fade span
const POLE_AHEAD: f32 = 0.5; // knee hint ahead of the foot
const POLE_UP: f32 = 0.2;
const ALIGN_MIN_WEIGHT: f32 = 0.5; // plant weight that teaches the sole's up axis
                                   // FeetIKLeft/RightOffset and FeetIKLeft/RightRotationOffset.y. Unity's vertical/twist limits need
                                   // no port: only walkable normals reorient the foot (WALKABLE_NORMAL_Y).
const POS_OFFSET: [Vector3; 2] = [Vector3::ZERO, Vector3::new(0.0, 0.0, 0.08)];
const YAW_OFFSET: [f32; 2] = [-5.35, 6.39];
const WALKABLE_NORMAL_Y: f32 = 0.695; // same walkable threshold the player uses

/// Sole alignment: TwoBoneIK3D positions the end bone but never applies the target's rotation,
/// so the foot keeps the clip's orientation and floats off a slope. This rotates the foot about
/// the minimal arc that puts its sole normal (learned once, while planted) onto the ground
/// normal, keeping the clip's yaw. Added after the chains, so it aligns the solved pose.
#[derive(GodotClass)]
#[class(base=SkeletonModifier3D, init)]
pub struct FootAlignModifier3D {
    base: Base<SkeletonModifier3D>,
    bones: [i32; 2],
    /// Bone-local axis that pointed world-up when planted.
    ups: [Option<Vector3>; 2],
    #[init(val = [Vector3::UP; 2])]
    normals: [Vector3; 2],
    weights: [f32; 2],
}

impl FootAlignModifier3D {
    fn learn(&mut self, side: usize, up_world: Vector3, weight: f32) {
        if self.ups[side].is_some() || weight < ALIGN_MIN_WEIGHT {
            return;
        }
        self.ups[side] = up_world.try_normalized();
    }
}

#[godot_api]
impl ISkeletonModifier3D for FootAlignModifier3D {
    fn process_modification(&mut self) {
        let Some(mut skel) = self.base().get_skeleton() else {
            return;
        };
        let skel_basis = skel.get_global_transform().basis;
        for side in SIDES {
            let (w, Some(up)) = (self.weights[side], self.ups[side]) else {
                continue;
            };
            if w <= 0.0 {
                continue;
            }
            let idx = self.bones[side];
            let world_rot = skel_basis * skel.get_bone_global_pose(idx).basis;
            let (Some(up_now), Some(normal)) = (
                (world_rot * up).try_normalized(),
                self.normals[side].try_normalized(),
            ) else {
                continue;
            };
            // Antiparallel is the one arc a rotation arc cannot express.
            if up_now.dot(normal) < -0.999 {
                continue;
            }
            let arc = Quaternion::IDENTITY.slerp(Quaternion::from_rotation_arc(up_now, normal), w);
            // Godot's +Y turns left where Unity's turns right, so the ported yaw offset flips sign.
            let yaw = Basis::from_axis_angle(Vector3::UP, -YAW_OFFSET[side].to_radians());
            let want_world = yaw * (Basis::from_quaternion(arc) * world_rot);
            set_bone_world_rotation(&mut skel, idx, skel_basis, want_world);
        }
    }
}

/// Unity pulls the hips down so a leg can reach ground the clip left too high: that is what puts a
/// foot on a step without the pelvis floating. Added BEFORE the leg chains, which solve against
/// the lowered pelvis.
#[derive(GodotClass)]
#[class(base=SkeletonModifier3D, init)]
pub struct HipsPullModifier3D {
    base: Base<SkeletonModifier3D>,
    #[init(val = -1)]
    bone: i32,
    /// Metres the most-stretched leg had to reach.
    pull: f32,
    weight: f32,
}

#[godot_api]
impl ISkeletonModifier3D for HipsPullModifier3D {
    fn process_modification(&mut self) {
        let Some(mut skel) = self.base().get_skeleton() else {
            return;
        };
        if self.bone < 0 || self.weight <= 0.0 || self.pull <= 0.0 || !skel.is_inside_tree() {
            return;
        }
        let skel_basis = skel.get_global_transform().basis;
        if skel_basis.get_scale().y < 0.0001 {
            return;
        }
        // The skeleton runs in centimetres (scale 0.01) and inverse() already divides by that
        // scale: dividing again sent the hips 49 m down at any step edge.
        let delta = skel_basis.inverse() * (Vector3::DOWN * self.pull.min(PULL_MAX) * self.weight);
        if !delta.is_finite() {
            return;
        }
        // Position only: rewriting the whole pose replaces the animated rotation and scale.
        let position = skel.get_bone_pose_position(self.bone);
        skel.set_bone_pose_position(self.bone, position + delta);
    }
}

struct FeetRig {
    targets: [Gd<Marker3D>; 2],
    poles: [Gd<Marker3D>; 2],
    probes: [Gd<ShapeCast3D>; 2],
    chains: [Gd<TwoBoneIk3d>; 2],
    align: Gd<FootAlignModifier3D>,
    hips: Gd<HipsPullModifier3D>,
    feet: [i32; 2],
}

#[derive(Default)]
pub struct FeetIk {
    rig: Option<FeetRig>,
    influence: [f32; 2],
    /// How far each leg had to stretch.
    distance: [f32; 2],
    /// Eased toward the most-stretched foot.
    hips_pull: f32,
}

fn foot_world(skeleton: &Gd<Skeleton3D>, bone: i32) -> Vector3 {
    skeleton.get_global_transform() * skeleton.get_bone_global_pose(bone).origin
}

impl FeetIk {
    /// Per-frame entry; `enabled` is false whenever IK must not apply at all.
    pub fn update(
        &mut self,
        avatar: &mut DclAvatar,
        skeleton: &Gd<Skeleton3D>,
        delta: f32,
        enabled: bool,
    ) {
        if !enabled {
            self.release();
            return;
        }
        if self.rig.is_none() {
            self.rig = Some(build(avatar, skeleton));
        }
        for side in SIDES {
            self.update_foot(avatar, skeleton, side, delta);
        }
        // Second pass, as Unity does: pull the hips down by the most stretched leg, blended by how
        // planted the feet are. The blend is folded into the eased value: the plant weight flips
        // between 0 and 1 in one frame, and scaling the eased pull by it bounced the pelvis.
        let plant = (self.influence[0] + self.influence[1]) * 0.5;
        let stretched = self.distance[0].max(self.distance[1]) * plant;
        let target = stretched.clamp(0.0, PULL_MAX);
        let step = POS_SPEED * delta;
        self.hips_pull = if (target - self.hips_pull).abs() <= step {
            target
        } else {
            self.hips_pull + (target - self.hips_pull).signum() * step
        };
        if let Some(rig) = self.rig.as_mut() {
            let mut hips = rig.hips.bind_mut();
            hips.pull = self.hips_pull;
            hips.weight = 1.0;
        }
    }

    fn release(&mut self) {
        let Some(rig) = self.rig.as_mut() else {
            return;
        };
        for side in SIDES {
            self.influence[side] = 0.0;
            self.distance[side] = 0.0;
            rig.chains[side].set_influence(0.0);
            rig.align.bind_mut().weights[side] = 0.0;
        }
        self.hips_pull = 0.0;
        let mut hips = rig.hips.bind_mut();
        hips.pull = 0.0;
        hips.weight = 0.0;
    }

    fn update_foot(
        &mut self,
        avatar: &DclAvatar,
        skeleton: &Gd<Skeleton3D>,
        side: usize,
        delta: f32,
    ) {
        let Some(rig) = self.rig.as_mut() else {
            return;
        };
        let foot = foot_world(skeleton, rig.feet[side]);
        // Park the probe on the foot's column, PULL_MAX above it. The sphere and the motion vector
        // live in the node's local units, so both rescale with the rig.
        let probe = &mut rig.probes[side];
        probe.set_global_position(foot + Vector3::UP * PULL_MAX);
        let probe_scale = probe.get_global_transform().basis.get_scale().y.max(0.0001);
        probe.set_target_position(Vector3::new(0.0, -CAST_LENGTH / probe_scale, 0.0));
        if let Some(mut ball) = probe
            .get_shape()
            .and_then(|s: Gd<Shape3D>| s.try_cast::<SphereShape3D>().ok())
        {
            ball.set_radius(SPHERE_RADIUS / probe_scale);
        }
        probe.force_shapecast_update();
        let mut up = Vector3::UP;
        let mut travel = 0.0; // how far the sphere's centre moved before it touched
        let mut grounded = false;
        if probe.is_colliding() {
            // Unity keeps the hit only when its normal is walkable: a near-vertical normal is a
            // wall (the side of a raised parcel, a kerb face), not the ground under this foot.
            travel = probe.get_closest_collision_safe_fraction() * CAST_LENGTH;
            up = probe.get_collision_normal(0);
            grounded = up.y >= WALKABLE_NORMAL_Y;
        }
        if !grounded {
            // Unity kills the IK outright when the probe found no ground: the leg returns to the
            // clip. A fallback plane would snap the foot to a surface that is not there.
            self.influence[side] = 0.0;
            self.distance[side] = 0.0;
            rig.chains[side].set_influence(0.0);
            rig.align.bind_mut().weights[side] = 0.0;
            return;
        }
        // The swept fraction measures the centre's travel; the contact is that centre pushed back
        // out along the normal by the radius.
        let hit = probe.get_global_position() + Vector3::DOWN * travel - up * SPHERE_RADIUS;
        // Unity decides planting from the CLIP, not from the ground: the animated foot's height
        // in avatar space, rounded to a binary weight. The ground only decides WHERE it lands.
        let target_pos = hit + up * HEIGHT_CORRECTION + POS_OFFSET[side];
        let target = &mut rig.targets[side];
        let current = target.get_global_position();
        target.set_global_position(current.move_toward(target_pos, POS_SPEED * delta));
        let avatar_xf = avatar.base().get_global_transform();
        rig.poles[side]
            .set_global_position(foot + forward_xz(&avatar_xf) * POLE_AHEAD + up * POLE_UP);
        let local_y = foot.y - avatar_xf.origin.y;
        // Unity's Distance adds back the radius it sweeps, so the stretch left for the hips is the
        // surface the sphere reached minus the probe's own pull.
        self.distance[side] = travel + SPHERE_RADIUS - PULL_MAX - local_y;
        // IsInsideMesh: ground above the foot pulls it out whatever the clip says.
        let inside_mesh = target_pos.y > foot.y;
        let w = if inside_mesh || 1.0 - (local_y - HEIGHT_CORRECTION) / DISABLE_DIST > 0.5 {
            1.0
        } else {
            0.0
        };
        self.influence[side] = if w > self.influence[side] {
            w.min(self.influence[side] + WEIGHT_SPEED * delta)
        } else {
            w // release is instant: a jump must not drag the feet
        };
        let influence = self.influence[side];
        rig.chains[side].set_influence(influence);
        // Teach the sole's up axis while the foot is genuinely planted flat.
        let world_rot = skeleton.get_global_transform().basis
            * skeleton.get_bone_global_pose(rig.feet[side]).basis;
        let mut align = rig.align.bind_mut();
        align.normals[side] = up;
        align.weights[side] = influence;
        align.learn(side, world_rot.inverse() * Vector3::UP, influence);
    }
}

fn build(avatar: &mut DclAvatar, skeleton: &Gd<Skeleton3D>) -> FeetRig {
    let mut skeleton = skeleton.clone();
    let mut hips = HipsPullModifier3D::new_alloc();
    hips.set_name("IKHipsPull");
    hips.bind_mut().bone = skeleton.find_bone("Avatar_Hips");
    skeleton.add_child(&hips);
    let avatar_xf = avatar.base().get_global_transform();
    let names = ["Left", "Right"];
    let feet = [skeleton.find_bone(FOOT[0]), skeleton.find_bone(FOOT[1])];
    let mut make = |side: usize| {
        let name = names[side];
        let mut target = Marker3D::new_alloc();
        target.set_name(&format!("IKFootTarget{name}"));
        avatar.base_mut().add_child(&target);
        let mut pole = Marker3D::new_alloc();
        pole.set_name(&format!("IKFootPole{name}"));
        avatar.base_mut().add_child(&pole);
        let mut ball = SphereShape3D::new_gd();
        ball.set_radius(SPHERE_RADIUS);
        let mut probe = ShapeCast3D::new_alloc();
        probe.set_name(&format!("IKFootProbe{name}"));
        probe.set_shape(&ball);
        probe.set_collision_mask(CL_PHYSICS);
        probe.set_target_position(Vector3::new(0.0, -CAST_LENGTH, 0.0));
        // Every read follows a forced update, so the engine never needs to sweep it.
        probe.set_enabled(false);
        avatar.base_mut().add_child(&probe);
        let mut chain = TwoBoneIk3d::new_alloc();
        chain.set_name(&format!("IKFoot{name}"));
        skeleton.add_child(&chain);
        chain.set_setting_count(1);
        // Without mutable axes the solver only turns the leg around its primary axis and cannot
        // reach a target below the animated foot.
        chain.set_mutable_bone_axes(true);
        chain.set_root_bone_name(0, THIGH[side]);
        chain.set_middle_bone_name(0, SHIN[side]);
        chain.set_end_bone_name(0, FOOT[side]);
        chain.set_end_bone_direction(0, BoneDirection::FROM_PARENT);
        let target_path = chain.get_path_to(&target);
        chain.set_target_node(0, &target_path);
        let pole_path = chain.get_path_to(&pole);
        chain.set_pole_node(0, &pole_path);
        chain.set_influence(0.0);
        // Seed both hints at the rest foot so the first solved frame has no snap.
        let rest = foot_world(&skeleton, feet[side]);
        target.set_global_position(rest);
        pole.set_global_position(
            rest + forward_xz(&avatar_xf) * POLE_AHEAD + Vector3::UP * POLE_UP,
        );
        (target, pole, probe, chain)
    };
    let (t0, p0, r0, c0) = make(0);
    let (t1, p1, r1, c1) = make(1);
    // Added last so it runs after both chains and aligns the solved pose.
    let mut align = FootAlignModifier3D::new_alloc();
    align.set_name("IKFootAlign");
    align.bind_mut().bones = feet;
    skeleton.add_child(&align);
    FeetRig {
        targets: [t0, t1],
        poles: [p0, p1],
        probes: [r0, r1],
        chains: [c0, c1],
        align,
        hips,
        feet,
    }
}
