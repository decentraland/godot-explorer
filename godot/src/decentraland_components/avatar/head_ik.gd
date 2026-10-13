class_name AvatarHeadIK
extends RefCounted
## audit #905 / Unity HeadIKSystem + ApplyHeadLookAt: while the avatar stands
## still it turns its head toward the camera, clamped per axis, and eases back
## frontal when the direction falls behind the shoulders. Bone-only, as a delta
## on top of the animated pose.

const HEAD := "Avatar_Head"

const WEIGHT_SPEED := 2.0  # HeadIKWeightChangeSpeed
const ROTATION_SPEED := 360.0  # HeadIKRotationSpeed (deg/s)
const HORIZONTAL_LIMIT := 50.0  # HeadIKHorizontalAngleLimit
const VERTICAL_LIMIT := 60.0  # HeadIKVerticalAngleLimit
# Beyond this the direction is behind the shoulders: look frontal instead, at a
# third of the speed (Unity scales maxDeltaDegrees by 1/3).
const HORIZONTAL_RESET := 120.0  # HeadIKHorizontalAngleReset
const RESET_SPEED_SCALE := 0.3333333
# Unity gates on MoveVelocity.sqrMagnitude > 0.5 while it is running, so the
# speed that turns head IK off is its square root.
const MAX_SPEED := 0.7071068


## Applied last on the skeleton: rotates the head's world pose so the turn reads
## on top of whatever the clip did with it.
class HeadTurnModifier3D:
	extends SkeletonModifier3D

	const YAW_DEAD_ZONE := 0.05

	var bone := -1
	var yaw := 0.0  # radians, + = character turns to its own right
	var pitch := 0.0  # radians, + = up
	var weight := 0.0
	# The avatar's own frame: Unity builds this rotation on a rig node that
	# inherits the avatar, so its axes are the body's, not the world's. Using the
	# world right pitches the head sideways as soon as the body faces east/west.
	var frame := Basis.IDENTITY

	func _process_modification() -> void:
		var skel := get_skeleton()
		if skel == null or bone < 0 or weight <= 0.0 or not skel.is_inside_tree():
			return
		var skel_basis := skel.global_transform.basis.orthonormalized()
		# Yaw about the body's up, then pitch about the body's right, the order
		# Unity composes (horizontalTargetRotation * AxisAngle(right, ..)).
		var delta_local := Basis(Vector3.UP, -yaw) * Basis(Vector3.RIGHT, pitch)
		var delta := frame * delta_local * frame.inverse()
		var cur_world := (skel_basis * skel.get_bone_global_pose(bone).basis).orthonormalized()
		var mixed := cur_world.slerp((delta * cur_world).orthonormalized(), weight)
		var parent_skel := Basis.IDENTITY
		var parent_idx := skel.get_bone_parent(bone)
		if parent_idx >= 0:
			parent_skel = skel.get_bone_global_pose(parent_idx).basis
		var local := parent_skel.inverse() * (skel_basis.inverse() * mixed)
		skel.set_bone_pose_rotation(bone, local.get_rotation_quaternion())


# The angles the local player last AIMED at, in degrees, before the limits and
# the easing — what Unity broadcasts (HeadIKComponent.GetHeadYawAndPitch).
var head_yaw_deg := 0.0
var head_pitch_deg := 0.0

var _avatar: Node3D
var _skeleton: Skeleton3D
var _modifier: HeadTurnModifier3D
var _built := false


func _init(avatar: Node3D, skeleton: Skeleton3D) -> void:
	_avatar = avatar
	_skeleton = skeleton


## Per-frame entry. `look_dir` is the direction the head should aim at (the view
## camera's forward for the local player).
func update_ik(delta: float, enabled: bool, look_dir: Vector3) -> void:
	if not _built:
		if not enabled:
			return
		_build()
	var target_weight := 1.0 if enabled else 0.0
	_modifier.weight = move_toward(_modifier.weight, target_weight, WEIGHT_SPEED * delta)
	if _modifier.weight <= 0.0:
		return
	if not enabled:
		# Weight is on its way out: keep looking where we were, the blend fades.
		return
	var basis := _avatar.global_transform.basis.orthonormalized()
	_modifier.frame = basis
	var dir := (basis.inverse() * look_dir).normalized()
	var horizontal := sqrt(dir.x * dir.x + dir.z * dir.z)
	var yaw := rad_to_deg(atan2(dir.x, -dir.z))
	var pitch := rad_to_deg(atan2(dir.y, horizontal))
	if horizontal < HeadTurnModifier3D.YAW_DEAD_ZONE:
		# Camera pitched near vertical: keep the heading, track the pitch only.
		yaw = rad_to_deg(_modifier.yaw)
	head_yaw_deg = yaw
	head_pitch_deg = pitch
	_aim(delta, yaw, pitch)


## Per-frame entry for a remote avatar: the turn arrives over the wire as the
## sender's own head angles (Unity UpdateRemoteIK). They are
## already in the avatar's frame, so there is no direction to project. Same
## limits and easing as the local path, so a remote head reads like a local one.
func update_remote(delta: float, enabled: bool, yaw_deg: float, pitch_deg: float) -> void:
	if not _built:
		if not enabled:
			return
		_build()
	_modifier.weight = move_toward(_modifier.weight, 1.0 if enabled else 0.0, WEIGHT_SPEED * delta)
	if _modifier.weight <= 0.0 or not enabled:
		return
	_modifier.frame = _avatar.global_transform.basis.orthonormalized()
	_aim(delta, yaw_deg, pitch_deg)


## Shared tail: clamp the aim, then ease the applied turn toward it. The weight
## is already eased in by the caller.
func _aim(delta: float, yaw_deg: float, pitch_deg: float) -> void:
	var yaw := yaw_deg
	var pitch := pitch_deg
	var speed := ROTATION_SPEED
	if absf(yaw) > HORIZONTAL_RESET:
		# Behind the shoulders: drop both axes and ease frontal, slower.
		yaw = 0.0
		pitch = 0.0
		speed *= RESET_SPEED_SCALE
	else:
		yaw = clampf(yaw, -HORIZONTAL_LIMIT, HORIZONTAL_LIMIT)
		pitch = clampf(pitch, -VERTICAL_LIMIT, VERTICAL_LIMIT)
	_modifier.yaw = move_toward(_modifier.yaw, deg_to_rad(yaw), deg_to_rad(speed) * delta)
	_modifier.pitch = move_toward(_modifier.pitch, deg_to_rad(pitch), deg_to_rad(speed) * delta)


func _build() -> void:
	_built = true
	_modifier = HeadTurnModifier3D.new()
	_modifier.name = "IKHeadTurn"
	_modifier.bone = _skeleton.find_bone(HEAD)
	_skeleton.add_child(_modifier)
