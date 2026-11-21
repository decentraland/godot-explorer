# gdlint: disable=async-function-name
extends SceneTree

# Regression for #3080 (Head Aim IK snaps at extreme pitch). The aim has to be
# spread over the neck and the head, the way Unity's TwistChainConstraints do:
# rotating the head alone leaves the whole clamped angle at the neck joint and
# the avatar reads as a broken neck at +/-60 degrees of pitch.
#
# Only the rest transforms of the DCL rig are needed, lifted from avatar.tscn, so
# the test builds its own Spine2 > Neck > Head skeleton instead of loading an
# avatar (no body shape, no GDExtension, no content server).
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_head_ik_neck_split.gd

const HeadIK = preload("res://src/decentraland_components/avatar/head_ik.gd")

const NECK := 1
const HEAD := 2
# The DCL rig carries a -90 deg X rotation in the spine's rest pose and identity
# neck/head locals. An identity parent would hide a broken world->local
# conversion (right in the test, wrong on a real avatar), so keep the real one.
const SPINE2_REST := Transform3D(Basis(Quaternion(-0.707107, 0.0, 0.0, 0.707107)), Vector3.ZERO)
const NECK_REST := Transform3D(Basis.IDENTITY, Vector3(0.0, 14.0786, 0.0))
const HEAD_REST := Transform3D(Basis.IDENTITY, Vector3(0.0, 24.2357, 0.0))

# Enough frames for the weight (2/s) and the rotation easing (360 deg/s) to
# settle on the clamped target.
const FRAMES := 30

# The aim split, in degrees: the joint may bend a little (the head holds the
# full aim, the neck almost all of it) but never by the whole clamped angle.
const MAX_JOINT_BEND := 20.0
const MIN_NECK_TURN := 30.0
# The head keeps the rest of the aim over the neck, so its local twist stays
# small; Unity's chain gives the same residual at the head joint.
const MAX_HEAD_TURN := 15.0

const TOL := 1.0

var _failures: Array[String] = []


func _initialize() -> void:
	await process_frame
	await _check_remote("pitch down", 0.0, -60.0, 60.0)
	await _check_remote("pitch up", 0.0, 60.0, 60.0)
	await _check_remote("yaw", 50.0, 0.0, 50.0)
	await _check_remote("reset behind shoulders", 179.0, 0.0, 0.0)
	await _check_local("local pitch down", -60.0)

	if _failures.is_empty():
		print("[test_head_ik_neck_split] PASS")
		quit(0)
	else:
		for failure in _failures:
			printerr("  " + failure)
		printerr("[test_head_ik_neck_split] FAIL: %d case(s)" % _failures.size())
		quit(1)


func _check_remote(label: String, yaw: float, pitch: float, expected_total: float) -> void:
	var rig := _build_rig()
	var avatar: Node3D = rig[0]
	var skel: Skeleton3D = rig[1]
	await process_frame
	var ik = HeadIK.new(avatar, skel)
	var sample: Dictionary = await _settle(
		skel, func() -> void: ik.update_remote(0.05, true, yaw, pitch)
	)
	_expect(label, sample, expected_total)
	avatar.queue_free()
	await process_frame


func _check_local(label: String, pitch_deg: float) -> void:
	var rig := _build_rig()
	var avatar: Node3D = rig[0]
	var skel: Skeleton3D = rig[1]
	await process_frame
	var ik = HeadIK.new(avatar, skel)
	# Same direction the local path gets from the camera's forward: down is -Y.
	var pitch := deg_to_rad(pitch_deg)
	var look_dir := avatar.global_transform.basis * Vector3(0.0, sin(pitch), -cos(pitch))
	var sample: Dictionary = await _settle(skel, func() -> void: ik.update_ik(0.05, true, look_dir))
	_expect(label, sample, absf(pitch_deg))
	avatar.queue_free()
	await process_frame


func _expect(label: String, sample: Dictionary, expected_total: float) -> void:
	if sample.is_empty():
		_failures.append("%s: head modifier never ran" % label)
		return
	var total: float = sample["head_total"]
	if absf(total - expected_total) > TOL:
		_failures.append("%s: head aim is %.2f deg, expected %.2f" % [label, total, expected_total])
	if expected_total <= TOL:
		if sample["neck_vs_rest"] > TOL:
			_failures.append(
				(
					"%s: neck kept %.2f deg of aim after the frontal reset"
					% [label, sample["neck_vs_rest"]]
				)
			)
		return
	if sample["neck_vs_rest"] < MIN_NECK_TURN:
		_failures.append(
			(
				"%s: neck turned %.2f deg, expected at least %.2f"
				% [label, sample["neck_vs_rest"], MIN_NECK_TURN]
			)
		)
	if sample["rel"] > MAX_JOINT_BEND:
		_failures.append(
			(
				"%s: head-neck joint bent %.2f deg, expected at most %.2f"
				% [label, sample["rel"], MAX_JOINT_BEND]
			)
		)
	if sample["head_vs_rest"] > MAX_HEAD_TURN:
		_failures.append(
			(
				"%s: head turned %.2f deg over the neck, expected at most %.2f"
				% [label, sample["head_vs_rest"], MAX_HEAD_TURN]
			)
		)


func _build_rig() -> Array:
	var avatar := Node3D.new()
	avatar.rotation_degrees.y = 90.0
	root.add_child(avatar)
	var skel := Skeleton3D.new()
	avatar.add_child(skel)
	skel.add_bone("Avatar_Spine2")
	skel.set_bone_rest(0, SPINE2_REST)
	skel.add_bone("Avatar_Neck")
	skel.set_bone_parent(NECK, 0)
	skel.set_bone_rest(NECK, NECK_REST)
	skel.add_bone("Avatar_Head")
	skel.set_bone_parent(HEAD, NECK)
	skel.set_bone_rest(HEAD, HEAD_REST)
	skel.reset_bone_poses()
	return [avatar, skel]


## Drives `update` for FRAMES frames and returns the pose measured on
## `skeleton_updated`, the only point where the modifier's output is visible
## (Skeleton3D restores the animation pose right after skinning).
func _settle(skel: Skeleton3D, update: Callable) -> Dictionary:
	var sample := {}
	var rest_neck: Quaternion = skel.get_bone_rest(NECK).basis.get_rotation_quaternion()
	var rest_head: Quaternion = skel.get_bone_rest(HEAD).basis.get_rotation_quaternion()
	var rest_head_world: Basis = skel.get_bone_global_rest(HEAD).basis
	var on_updated := func() -> void:
		var neck_world: Basis = skel.get_bone_global_pose(NECK).basis
		var head_world: Basis = skel.get_bone_global_pose(HEAD).basis
		sample["head_total"] = rad_to_deg(
			(rest_head_world.inverse() * head_world).get_rotation_quaternion().get_angle()
		)
		sample["neck_vs_rest"] = rad_to_deg(
			(skel.get_bone_pose_rotation(NECK) * rest_neck.inverse()).get_angle()
		)
		sample["head_vs_rest"] = rad_to_deg(
			(skel.get_bone_pose_rotation(HEAD) * rest_head.inverse()).get_angle()
		)
		sample["rel"] = rad_to_deg(
			(neck_world.inverse() * head_world).get_rotation_quaternion().get_angle()
		)
	skel.skeleton_updated.connect(on_updated)
	for i in FRAMES:
		update.call()
		await process_frame
	skel.skeleton_updated.disconnect(on_updated)
	return sample
