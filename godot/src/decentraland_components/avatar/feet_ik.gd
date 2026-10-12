class_name AvatarFeetIK
extends RefCounted
## audit #905 / Unity FeetIKSystem: plants each animated foot on the ground
## under THAT foot, so the feet follow slopes, steps and curbs instead of the
## flat plane the clip assumes. The IK weight is driven by the clip itself: a
## foot the animation lifted loses weight, so only the planted foot is solved.
##
## The probe is a ShapeCast3D (sphere, radius SPHERE_RADIUS) per foot: Unity
## sweeps a sphere of FeetIKSphereSize, and a fat probe rolls over a curb edge
## instead of flipping between the kerb's face and the street a step below,
## which stepped the hips by the whole drop in one frame. The engine updates the
## probe inside the physics step; the avatar never calls direct_space_state
## itself — queries issued from the avatar node disturbed is_on_floor() and the
## player's predictive step-up (the capsule stuck on steps and replayed the
## landing "bounce").

const THIGH := {"left": "Avatar_LeftUpLeg", "right": "Avatar_RightUpLeg"}
const SHIN := {"left": "Avatar_LeftLeg", "right": "Avatar_RightLeg"}
const FOOT := {"left": "Avatar_LeftFoot", "right": "Avatar_RightFoot"}
const SIDES := ["left", "right"]

const RAY_MASK := 2  # CL_PHYSICS — world geometry (the player is CL_PLAYER)
const PULL_MAX := 0.6  # FeetIKHipsPullMaxDistance: ray origin above the foot
const CAST_LENGTH := 1.2  # cast reach, Unity parity
const WEIGHT_SPEED := 10.0  # IKWeightSpeed (rise only; release is instant)
const POS_SPEED := 10.0  # IKPositionSpeed (m/s)
const HEIGHT_CORRECTION := 0.08  # FeetHeightCorrection — idle ankle height
const SPHERE_RADIUS := 0.08  # FeetIKSphereSize — probe radius
const DISABLE_DIST := 0.1  # FeetHeightDisableIkDistance — clip-driven fade span
const POLE_AHEAD := 0.5  # knee hint ahead of the foot
const POLE_UP := 0.2
const ALIGN_MIN_WEIGHT := 0.5  # plant weight that teaches the sole's up axis
# FeetIKLeft/RightOffset and FeetIKLeft/RightRotationOffset.y. The rotation
# offset is a small yaw on the planted foot; Unity's vertical/twist limits need
# no port here because only walkable normals reorient the foot (WALKABLE_NORMAL_Y).
const POS_OFFSET := {"left": Vector3(0.0, 0.0, 0.0), "right": Vector3(0.0, 0.0, 0.08)}
const YAW_OFFSET := {"left": -5.35, "right": 6.39}
const WALKABLE_NORMAL_Y := 0.695  # same walkable threshold the player uses

var _avatar: Node3D
var _skeleton: Skeleton3D
var _targets := {}  # side -> Marker3D
var _poles := {}  # side -> Marker3D
var _rays := {}  # side -> ShapeCast3D
var _chains := {}  # side -> TwoBoneIK3D
var _align: FootAlignModifier3D
var _hips: HipsPullModifier3D
var _feet := {}  # side -> bone idx
var _influence := {"left": 0.0, "right": 0.0}
var _distance := {"left": 0.0, "right": 0.0}  # how far each leg had to stretch
var _hips_pull := 0.0  # eased toward the most-stretched foot
var _built := false


## Sole alignment: TwoBoneIK3D can position the end bone but never applies the
## target's rotation, so the foot keeps the clip's orientation and floats off a
## slope. This rotates the foot about the minimal arc that puts its sole normal
## (learned once, while the foot is planted) onto the ground normal, keeping the
## clip's yaw. Runs after the IK chains, so it aligns the solved pose.
class FootAlignModifier3D:
	extends SkeletonModifier3D

	var bones := {}  # side -> bone idx
	var ups := {}  # side -> bone-local axis that pointed world-up when planted
	var normals := {"left": Vector3.UP, "right": Vector3.UP}
	var weights := {"left": 0.0, "right": 0.0}

	func learn(side: String, up_world: Vector3, weight: float) -> void:
		if ups.has(side) or weight < AvatarFeetIK.ALIGN_MIN_WEIGHT:
			return
		ups[side] = up_world.normalized()

	func _process_modification() -> void:
		var skel := get_skeleton()
		if skel == null:
			return
		var skel_basis := skel.global_transform.basis
		for side in AvatarFeetIK.SIDES:
			var w: float = weights[side]
			if w <= 0.0 or not ups.has(side):
				continue
			var idx: int = bones[side]
			var world_rot := skel_basis * skel.get_bone_global_pose(idx).basis
			var up_now: Vector3 = (world_rot * (ups[side] as Vector3)).normalized()
			var normal: Vector3 = normals[side]
			normal = normal.normalized()
			# Antiparallel is the one arc Quaternion(from, to) cannot express.
			if up_now.dot(normal) < -0.999:
				continue
			var arc := Quaternion.IDENTITY.slerp(Quaternion(up_now, normal), w)
			# Godot's +Y turns left where Unity's turns right, so the ported yaw
			# offset flips sign on the way over.
			var yaw := Basis(Vector3.UP, -deg_to_rad(float(AvatarFeetIK.YAW_OFFSET[side])))
			var want_world := yaw * (Basis(arc) * world_rot)
			# Back to a bone-local rotation: world -> skeleton -> parent bone.
			var parent_skel := Basis.IDENTITY
			var parent_idx := skel.get_bone_parent(idx)
			if parent_idx >= 0:
				parent_skel = skel.get_bone_global_pose(parent_idx).basis
			var local := parent_skel.inverse() * (skel_basis.inverse() * want_world)
			skel.set_bone_pose_rotation(idx, local.get_rotation_quaternion())


## Unity pulls the hips down so a leg can reach ground the clip left too high —
## that is what puts a foot on a step without the pelvis floating. Runs BEFORE
## the leg chains: the legs have to solve against the lowered pelvis.
class HipsPullModifier3D:
	extends SkeletonModifier3D

	var bone := -1
	var pull := 0.0  # metres the most-stretched leg had to reach
	var weight := 0.0  # average plant weight of the two feet

	func _process_modification() -> void:
		var skel := get_skeleton()
		if skel == null or bone < 0 or weight <= 0.0 or pull <= 0.0 or not skel.is_inside_tree():
			return
		var skel_basis := skel.global_transform.basis
		var scale := skel_basis.get_scale().y
		if scale < 0.0001:
			return
		# The skeleton runs in centimetres (scale 0.01), and inverse() already
		# divides by that scale: converting the world delta with it alone yields
		# local units. Dividing by the scale a second time sent the hips 49 m down
		# and stretched the whole avatar to a point at any step edge.
		var delta := (
			skel_basis.inverse() * (Vector3.DOWN * minf(pull, AvatarFeetIK.PULL_MAX) * weight)
		)
		if not (is_finite(delta.x) and is_finite(delta.y) and is_finite(delta.z)):
			return
		# Position only: rewriting the whole pose replaces the animated rotation
		# and scale with whatever a failed read returned, which drops the entire
		# body onto the skeleton origin (it collapsed to a point in QA).
		var pos := skel.get_bone_pose_position(bone)
		pos += delta
		skel.set_bone_pose_position(bone, pos)


func _init(avatar: Node3D, skeleton: Skeleton3D) -> void:
	_avatar = avatar
	_skeleton = skeleton


## Per-frame entry; `enabled` is false whenever IK must not apply at all.
func update_ik(delta: float, enabled: bool) -> void:
	if not enabled:
		_release()
		return
	_build()
	for side in SIDES:
		_update_foot(side, delta)
	# Second pass, as Unity does: pull the hips down by the most stretched leg,
	# blended by how planted the feet are. That blend is folded into the eased
	# value: the plant weight is clip-driven and flips between 0 and 1 in one
	# frame, so scaling the eased pull by it stepped the pelvis by the whole
	# stretch at once (the hard bounce felt when walking over a kerb).
	var plant := (float(_influence["left"]) + float(_influence["right"])) * 0.5
	var stretched: float = maxf(float(_distance["left"]), float(_distance["right"])) * plant
	_hips_pull = move_toward(_hips_pull, clampf(stretched, 0.0, PULL_MAX), POS_SPEED * delta)
	_hips.pull = _hips_pull
	_hips.weight = 1.0


func _release() -> void:
	if not _built:
		return
	for side in SIDES:
		_influence[side] = 0.0
		_chains[side].influence = 0.0
		_align.weights[side] = 0.0
		_distance[side] = 0.0
	_hips_pull = 0.0
	_hips.pull = 0.0
	_hips.weight = 0.0


func _build() -> void:
	if _built:
		return
	_built = true
	_hips = HipsPullModifier3D.new()
	_hips.name = "IKHipsPull"
	_hips.bone = _skeleton.find_bone("Avatar_Hips")
	_skeleton.add_child(_hips)
	for side in SIDES:
		var target := Marker3D.new()
		target.name = "IKFootTarget" + side.capitalize()
		_avatar.add_child(target)
		var pole := Marker3D.new()
		pole.name = "IKFootPole" + side.capitalize()
		_avatar.add_child(pole)
		var ball := SphereShape3D.new()
		ball.radius = SPHERE_RADIUS
		var probe := ShapeCast3D.new()
		probe.name = "IKFootProbe" + side.capitalize()
		probe.shape = ball
		probe.collision_mask = RAY_MASK
		probe.target_position = Vector3(0.0, -CAST_LENGTH, 0.0)
		_avatar.add_child(probe)
		var chain := TwoBoneIK3D.new()
		chain.name = "IKFoot" + side.capitalize()
		_skeleton.add_child(chain)
		chain.setting_count = 1
		# Without mutable axes the solver only turns the leg around its primary
		# axis and cannot reach a target below the animated foot.
		chain.mutable_bone_axes = true
		chain.set_root_bone_name(0, THIGH[side])
		chain.set_middle_bone_name(0, SHIN[side])
		chain.set_end_bone_name(0, FOOT[side])
		chain.set_end_bone_direction(0, SkeletonModifier3D.BONE_DIRECTION_FROM_PARENT)
		chain.set_target_node(0, chain.get_path_to(target))
		chain.set_pole_node(0, chain.get_path_to(pole))
		chain.influence = 0.0
		_feet[side] = _skeleton.find_bone(FOOT[side])
		_targets[side] = target
		_poles[side] = pole
		_rays[side] = probe
		_chains[side] = chain
		# Seed both hints at the rest foot so the first solved frame has no snap.
		var rest := _foot_world(side)
		target.global_position = rest
		pole.global_position = rest + _forward_xz() * POLE_AHEAD + Vector3.UP * POLE_UP
	# Added last so it runs after both chains and aligns the solved pose.
	_align = FootAlignModifier3D.new()
	_align.name = "IKFootAlign"
	for side in SIDES:
		_align.bones[side] = _feet[side]
	_skeleton.add_child(_align)


func _foot_world(side: String) -> Vector3:
	var pose := _skeleton.get_bone_global_pose(_feet[side])
	return _skeleton.global_transform * pose.origin


func _forward_xz() -> Vector3:
	var fwd := -_avatar.global_transform.basis.z
	fwd.y = 0.0
	return fwd.normalized() if fwd.length_squared() > 0.0001 else Vector3.FORWARD


func _update_foot(side: String, delta: float) -> void:
	var chain: TwoBoneIK3D = _chains[side]
	var foot := _foot_world(side)
	var probe: ShapeCast3D = _rays[side]
	# Park the probe on the foot's column, PULL_MAX above it; the engine sweeps
	# it during the physics step. The sphere and the motion vector live in the
	# node's local units, so both rescale with the rig (which is cm).
	probe.global_position = foot + Vector3.UP * PULL_MAX
	var probe_scale := maxf(probe.global_transform.basis.get_scale().y, 0.0001)
	probe.target_position = Vector3(0.0, -CAST_LENGTH / probe_scale, 0.0)
	(probe.shape as SphereShape3D).radius = SPHERE_RADIUS / probe_scale
	var hit := Vector3.ZERO
	var up := Vector3.UP
	var travel := 0.0  # how far the sphere's centre moved before it touched
	var grounded := false
	if probe.is_colliding():
		# Unity probes with a SphereCast(FeetIKSphereSize) and only keeps the hit
		# when its normal is walkable: a near-vertical normal is a wall — the side
		# of a raised parcel, a kerb face — not the ground under this foot. The
		# base parcels carry no collider, so the sphere passes through the floor
		# and catches the neighbouring face at whatever depth it happens to be.
		travel = probe.get_closest_collision_safe_fraction() * CAST_LENGTH
		up = probe.get_collision_normal(0)
		grounded = up.y >= WALKABLE_NORMAL_Y
	if not grounded:
		# Unity kills the IK outright when the probe found no ground: the target
		# stays where it is and the leg goes back to the clip. The old fallback
		# plane sat ABOVE the foot, which the IsInsideMesh rule below read as
		# ground through the mesh and turned the IK fully on — so every time the
		# probe lost the ground the foot snapped to a plane that was not there,
		# and the leg shook. Distance also goes to 0, so a foot with no ground
		# under it stops dragging the hips down.
		_influence[side] = 0.0
		chain.influence = 0.0
		_align.weights[side] = 0.0
		_distance[side] = 0.0
		return
	# The swept fraction measures the centre's travel; the contact is that centre
	# pushed back out along the normal by the radius.
	hit = probe.global_position + Vector3.DOWN * travel - up * SPHERE_RADIUS
	# Unity decides planting from the CLIP, not from the ground: the animated
	# foot's height in avatar space, rounded to a binary weight — a foot the clip
	# lifted gets no IK, however far below the ground under it is. The ground only
	# decides WHERE the foot is placed. This is what lets a foot follow a step or
	# slope down; a ground-relative gate killed exactly those cases.
	var target_pos: Vector3 = hit + up * HEIGHT_CORRECTION + (POS_OFFSET[side] as Vector3)
	var target: Marker3D = _targets[side]
	target.global_position = target.global_position.move_toward(target_pos, POS_SPEED * delta)
	var pole: Marker3D = _poles[side]
	pole.global_position = foot + _forward_xz() * POLE_AHEAD + up * POLE_UP
	var local_y := foot.y - _avatar.global_position.y
	# Unity's Distance adds back the radius it sweeps, so the stretch left for the
	# hips is the surface the sphere reached minus the probe's own pull.
	_distance[side] = travel + SPHERE_RADIUS - PULL_MAX - local_y
	var w := 0.0
	if target_pos.y > foot.y:
		w = 1.0  # IsInsideMesh: ground above the foot — pull it out
	else:
		w = 1.0 - (local_y - HEIGHT_CORRECTION) / DISABLE_DIST
		w = 1.0 if w > 0.5 else 0.0
	if w > _influence[side]:
		_influence[side] = minf(w, _influence[side] + WEIGHT_SPEED * delta)
	else:
		_influence[side] = w  # release is instant (a jump must not drag the feet)
	chain.influence = _influence[side]
	_align.normals[side] = up
	_align.weights[side] = _influence[side]
	# Teach the sole's up axis while the foot is genuinely planted flat.
	var world_rot := (
		_skeleton.global_transform.basis * _skeleton.get_bone_global_pose(_feet[side]).basis
	)
	_align.learn(side, world_rot.inverse() * Vector3.UP, _influence[side])
