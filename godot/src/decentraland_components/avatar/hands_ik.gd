class_name AvatarHandsIK
extends RefCounted
## audit #905 / Unity HandsIKSystem: a hand reaching a wall plants on it (the
## player can lean on geometry), elbow hinted so it bends outward. One RayCast3D
## per hand, swept on demand because this runs in idle; never a direct_space_state
## query from the avatar node (those break is_on_floor).

const UPPER_ARM := {"left": "Avatar_LeftArm", "right": "Avatar_RightArm"}
const FOREARM := {"left": "Avatar_LeftForeArm", "right": "Avatar_RightForeArm"}
const HAND := {"left": "Avatar_LeftHand", "right": "Avatar_RightHand"}
# Palm frame comes from the rig itself: the knuckle and thumb roots lie in the
# palm plane, so their cross product is the palm's normal, mirrored per side.
const KNUCKLE := {"left": "Avatar_LeftHandIndex1", "right": "Avatar_RightHandIndex1"}
const THUMB := {"left": "Avatar_LeftHandThumb1", "right": "Avatar_RightHandThumb1"}
const SIDES := ["left", "right"]

const RAY_MASK := AvatarCollisionLayers.CL_PHYSICS
const WALL_DISTANCE := 0.5  # HandsIKWallHitDistance
const WEIGHT_SPEED := 5.0  # HandsIKWeightSpeed
const POS_SPEED := 10.0  # IKPositionSpeed (m/s)
# Unity probes from a fixed point in avatar space (LeftHandWallRaycast at
# -0.205, RightHandWallRaycast at +0.205, both y=1.369) with identity rotation,
# i.e. straight along the character's forward — NOT along the arm, which points
# at the floor whenever the arms hang.
const PROBE_ORIGIN := Vector3(0.205, 1.369, 0.0)
# HandsIKElbowOffset, x negated on the left. Unity is +Z forward, Godot -Z, so
# the depth flips sign on the way over.
const ELBOW_OFFSET := Vector3(0.256, 0.9800129, 0.4610018)

var _avatar: Node3D
var _skeleton: Skeleton3D
var _targets := {}  # side -> Marker3D
var _poles := {}  # side -> Marker3D
var _rays := {}  # side -> RayCast3D
var _chains := {}  # side -> TwoBoneIK3D
var _upper := {}  # side -> bone idx
var _hands := {}  # side -> bone idx
var _influence := {"left": 0.0, "right": 0.0}
var _palm_frame := {}  # side -> bone-local frame (palm normal, knuckle, z)
var _align: HandAlignModifier3D
var _built := false


## Unity's constraint runs with TargetRotationWeight 1 / no rotation offset, so
## the hand bone takes the target's rotation outright: the code sets
## `handIKTarget.forward = -hit.normal`, i.e. the hand lies flat on the wall.
## TwoBoneIK3D positions the end bone but never applies the target's rotation,
## so this modifier does it — and it is what stops the hand sinking through.
##
## The orientation is built from each hand's own rest palm frame, never from one
## absolute basis shared by both: the left and right bones carry mirrored local
## axes, so a shared orientation twists one of them.
class HandAlignModifier3D:
	extends SkeletonModifier3D

	var bones := {}  # side -> bone idx
	var frames := {}  # side -> bone-local palm frame (Basis)
	var normals := {"left": Vector3.ZERO, "right": Vector3.ZERO}  # ZERO = no wall
	var weights := {"left": 0.0, "right": 0.0}
	var up_reference := Vector3.UP

	func _process_modification() -> void:
		var skel := get_skeleton()
		if skel == null:
			return
		var skel_basis := skel.global_transform.basis.orthonormalized()
		for side in AvatarHandsIK.SIDES:
			var w: float = weights[side]
			var n: Vector3 = normals[side]
			if w <= 0.0 or n == Vector3.ZERO or not frames.has(side):
				continue
			var into := -n.normalized()
			# Knuckles run up the wall: the residual of the up reference across
			# the wall plane, which is also what stops the hand rolling.
			var residual := up_reference - into * up_reference.dot(into)
			if residual.length_squared() < 0.0001:
				continue
			var knuckles := residual.normalized()
			var want_world := (
				Basis(into, knuckles, into.cross(knuckles)) * (frames[side] as Basis).inverse()
			)
			var idx: int = bones[side]
			var cur_world := (skel_basis * skel.get_bone_global_pose(idx).basis).orthonormalized()
			var mixed := cur_world.slerp(want_world.orthonormalized(), w)
			var parent_skel := Basis.IDENTITY
			var parent_idx := skel.get_bone_parent(idx)
			if parent_idx >= 0:
				parent_skel = skel.get_bone_global_pose(parent_idx).basis
			var local := parent_skel.inverse() * (skel_basis.inverse() * mixed)
			skel.set_bone_pose_rotation(idx, local.get_rotation_quaternion())


func _init(avatar: Node3D, skeleton: Skeleton3D) -> void:
	_avatar = avatar
	_skeleton = skeleton


## Per-frame entry; `enabled` is false whenever IK must not apply at all.
func update_ik(delta: float, enabled: bool) -> void:
	if not _built:
		if not enabled:
			return
		_build()
	for side in SIDES:
		_update_hand(side, delta, enabled)


func _build() -> void:
	_built = true
	for side in SIDES:
		var target := Marker3D.new()
		target.name = "IKHandTarget" + side.capitalize()
		_avatar.add_child(target)
		var pole := Marker3D.new()
		pole.name = "IKHandPole" + side.capitalize()
		_avatar.add_child(pole)
		var ray := RayCast3D.new()
		ray.name = "IKHandRay" + side.capitalize()
		ray.collision_mask = RAY_MASK
		_avatar.add_child(ray)
		var chain := TwoBoneIK3D.new()
		chain.name = "IKHand" + side.capitalize()
		_skeleton.add_child(chain)
		chain.setting_count = 1
		# Without mutable axes the solver only turns the arm around its primary
		# axis and cannot reach a wall beside it.
		chain.mutable_bone_axes = true
		chain.set_root_bone_name(0, UPPER_ARM[side])
		chain.set_middle_bone_name(0, FOREARM[side])
		chain.set_end_bone_name(0, HAND[side])
		chain.set_end_bone_direction(0, SkeletonModifier3D.BONE_DIRECTION_FROM_PARENT)
		chain.set_target_node(0, chain.get_path_to(target))
		chain.set_pole_node(0, chain.get_path_to(pole))
		chain.influence = 0.0
		_upper[side] = _skeleton.find_bone(UPPER_ARM[side])
		_hands[side] = _skeleton.find_bone(HAND[side])
		_targets[side] = target
		_poles[side] = pole
		_rays[side] = ray
		_chains[side] = chain
		_palm_frame[side] = _rest_palm_frame(side)
		# Seed the target at the hand so the first solved frame has no snap.
		target.global_position = _bone_world(_hands[side])
		_update_pole(side)
	# Added last so it runs after both chains and aligns the solved hands.
	_align = HandAlignModifier3D.new()
	_align.name = "IKHandAlign"
	for side in SIDES:
		_align.bones[side] = _hands[side]
		if _palm_frame.has(side):
			_align.frames[side] = _palm_frame[side]
	_skeleton.add_child(_align)


func _bone_world(idx: int) -> Vector3:
	return _skeleton.global_transform * _skeleton.get_bone_global_pose(idx).origin


## The hand's rest frame in the hand bone's own space: X = palm normal (out of
## the palm), Y = knuckle axis, Z = their cross product. Rest is what makes it
## per-side correct — the two hands carry mirrored local axes.
func _rest_palm_frame(side: String) -> Basis:
	var hand: int = _hands[side]
	var knuckle := _skeleton.find_bone(KNUCKLE[side])
	var thumb := _skeleton.find_bone(THUMB[side])
	if hand < 0 or knuckle < 0 or thumb < 0:
		return Basis.IDENTITY
	var to_knuckle := _rest_origin_in(hand, knuckle)
	var to_thumb := _rest_origin_in(hand, thumb)
	if to_knuckle.length_squared() < 0.000001 or to_thumb.length_squared() < 0.000001:
		return Basis.IDENTITY
	var palm := to_knuckle.normalized().cross(to_thumb.normalized()).normalized()
	var knuckles := to_knuckle.normalized()
	knuckles = (knuckles - palm * knuckles.dot(palm)).normalized()
	# The cross product flips sense on the mirrored side, so pin the sign to the
	# bind pose's convention (both palms face down there) instead of to a side.
	if (_rest_world_basis(hand) * palm).dot(Vector3.DOWN) < 0.0:
		palm = -palm
	return Basis(palm, knuckles, palm.cross(knuckles))


## Rest position of `bone` expressed in `ancestor`'s local space.
func _rest_origin_in(ancestor: int, bone: int) -> Vector3:
	var chain: Array[int] = []
	var i := bone
	while i >= 0 and i != ancestor:
		chain.append(i)
		i = _skeleton.get_bone_parent(i)
	if i < 0:
		return Vector3.ZERO
	chain.reverse()
	var xform := Transform3D.IDENTITY
	for b in chain:
		xform = xform * _skeleton.get_bone_rest(b)
	return xform.origin


## The bone's rest orientation in world space (rest pose, so it carries no
## animation) — the frame the rest-derived axes and signs are read in.
func _rest_world_basis(bone: int) -> Basis:
	var chain: Array[int] = []
	var i := bone
	while i >= 0:
		chain.append(i)
		i = _skeleton.get_bone_parent(i)
	chain.reverse()
	var rest := Basis.IDENTITY
	for b in chain:
		rest = rest * _skeleton.get_bone_rest(b).basis
	return (_skeleton.global_transform.basis * rest).orthonormalized()


func _update_pole(side: String) -> void:
	# Unity parks the elbow hint at a fixed point in the avatar's own space.
	var offset := ELBOW_OFFSET
	if side == "left":
		offset.x = -offset.x
	(_poles[side] as Marker3D).global_position = _avatar.global_transform * offset


func _update_hand(side: String, delta: float, enabled: bool) -> void:
	var chain: TwoBoneIK3D = _chains[side]
	_update_pole(side)
	# Probe straight ahead from the fixed hand-height point Unity uses, so the
	# hand plants on a wall the player walks into.
	var ray: RayCast3D = _rays[side]
	var probe := PROBE_ORIGIN
	if side == "left":
		probe.x = -probe.x
	var origin := _avatar.global_transform * probe
	var fwd := -_avatar.global_transform.basis.z
	var up := Vector3.UP
	if absf(fwd.dot(Vector3.UP)) > 0.99:
		up = Vector3.FORWARD
	ray.global_transform = Transform3D(Basis.looking_at(fwd, up), origin)
	var ray_scale := ray.global_transform.basis.get_scale().x
	ray.target_position = Vector3(0.0, 0.0, -WALL_DISTANCE / maxf(ray_scale, 0.0001))
	ray.force_raycast_update()
	var w := 0.0
	if enabled and ray.is_colliding():
		var hit := ray.get_collision_point()
		var target: Marker3D = _targets[side]
		target.global_position = target.global_position.move_toward(hit, POS_SPEED * delta)
		_align.normals[side] = ray.get_collision_normal()
		w = 1.0
	else:
		_align.normals[side] = Vector3.ZERO
	if not enabled:
		_influence[side] = 0.0
	else:
		_influence[side] = move_toward(_influence[side], w, WEIGHT_SPEED * delta)
	chain.influence = _influence[side]
	_align.weights[side] = _influence[side]
