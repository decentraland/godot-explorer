extends SceneTree

# Regression for #3080 (head sync points the wrong way on a peer). The wire
# carries the sender's aim in WORLD space as Unity Euler yaw/pitch
# (HeadIKComponent.GetHeadYawAndPitch over Camera.transform.forward), and Unity's
# remote IK compares it against the avatar's own head forward. Godot sent its
# body-relative yaw, so a peer applied a world heading: with the body facing
# anywhere but DCL +Z the head turned the wrong way, and past the 120 deg reset
# it stayed frontal.
#
# Pure math, no rig and no frames: head_ik.gd owns both conversions.
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_head_ik_wire.gd

const HeadIK = preload("res://src/decentraland_components/avatar/head_ik.gd")

const TOL := 0.01

# Avatar facing Godot +X: its local -Z (forward) maps to world +X, so its right
# (+X local) maps to world +Z.
const BODY_EAST := Basis(Vector3.UP, -PI / 2.0)

var _failures: Array[String] = []


func _initialize() -> void:
	# Wire = body heading + the aim the local path applied, wrapped to the range
	# Unity decodes. A body facing DCL +X broadcasts 90 for a frontal aim.
	_check("send north", HeadIK.body_to_wire(0.0, 0.0, Basis.IDENTITY), Vector2(0.0, 0.0))
	_check("send east", HeadIK.body_to_wire(0.0, 0.0, BODY_EAST), Vector2(90.0, 0.0))
	_check("send east right", HeadIK.body_to_wire(20.0, 0.0, BODY_EAST), Vector2(110.0, 0.0))
	_check("send east left", HeadIK.body_to_wire(-20.0, 0.0, BODY_EAST), Vector2(70.0, 0.0))
	_check("send pitch", HeadIK.body_to_wire(0.0, 45.0, BODY_EAST), Vector2(90.0, 45.0))
	_check("send wrapped", HeadIK.body_to_wire(170.0, 0.0, BODY_EAST), Vector2(-100.0, 0.0))

	# First person clamps pitch at +-90, where the horizontal projection vanishes
	# and re-deriving the heading from the camera flips it 180 deg: the peer would
	# read "behind the shoulders" and snap frontal. The heading must survive.
	_check("send straight up", HeadIK.body_to_wire(0.0, 90.0, BODY_EAST), Vector2(90.0, 90.0))
	_check("send straight down", HeadIK.body_to_wire(0.0, -90.0, BODY_EAST), Vector2(90.0, -90.0))

	# Unity's decoder rebuilds the direction from the pair and compares it with
	# the avatar's head forward: DCL +Z is Godot -Z, so the Godot direction
	# (x, y, z) arrives as (x, y, -z) and the running peer turns the head by the
	# delta. Facing +X and looking forward used to send 0 (body-relative) and
	# Unity aimed 90 deg to the side.
	_check_unity_agrees("unity sees east forward", BODY_EAST, Vector2(0.0, 0.0))
	_check_unity_agrees("unity sees east right", BODY_EAST, Vector2(35.0, 20.0))

	# A peer's angles, projected into this avatar's frame. Wire yaw 0 is DCL +Z,
	# which is Godot -Z: for a body facing Godot +X that heading sits on its left.
	_check("recv dcl +Z", HeadIK.wire_to_body(0.0, 0.0, BODY_EAST), Vector2(-90.0, 0.0))
	_check("recv body forward", HeadIK.wire_to_body(90.0, 0.0, BODY_EAST), Vector2(0.0, 0.0))
	_check("recv dcl -Z", HeadIK.wire_to_body(180.0, 0.0, BODY_EAST), Vector2(90.0, 0.0))
	_check("recv up", HeadIK.wire_to_body(90.0, 60.0, BODY_EAST), Vector2(0.0, 60.0))
	_check("recv identity", HeadIK.wire_to_body(30.0, 20.0, Basis.IDENTITY), Vector2(30.0, 20.0))
	# Unity sends the unsigned [0, 360) range; the decoder only wraps, so a small
	# left turn must not read as "behind the shoulders".
	_check("recv wrapped yaw", HeadIK.wire_to_body(350.0, 0.0, Basis.IDENTITY), Vector2(-10.0, 0.0))
	_check("recv behind", HeadIK.wire_to_body(180.0, 0.0, Basis.IDENTITY), Vector2(-180.0, 0.0))

	# Round trip through the wire recovers the body-relative aim the local IK
	# applied, for both directions and pitches.
	_round_trip("round trip forward", BODY_EAST, Vector2(0.0, 0.0))
	_round_trip("round trip right", BODY_EAST, Vector2(25.0, 15.0))
	_round_trip("round trip left", BODY_EAST, Vector2(-25.0, -15.0))
	_round_trip("round trip steep", BODY_EAST, Vector2(10.0, 80.0))
	_round_trip("round trip north body", Basis.IDENTITY, Vector2(-60.0, 30.0))

	if _failures.is_empty():
		print("[test_head_ik_wire] PASS")
		quit(0)
	else:
		for failure in _failures:
			printerr("  " + failure)
		printerr("[test_head_ik_wire] FAIL: %d case(s)" % _failures.size())
		quit(1)


func _round_trip(label: String, body: Basis, aim: Vector2) -> void:
	var wire := HeadIK.body_to_wire(aim.x, aim.y, body)
	_check(label, HeadIK.wire_to_body(wire.x, wire.y, body), aim)


func _check_unity_agrees(label: String, body: Basis, aim: Vector2) -> void:
	# Quaternion.Euler(pitch, yaw, 0) * Vector3.forward in Unity's left-handed
	# space, with the wire pitch positive down (what the encoder sends).
	var wire := HeadIK.body_to_wire(aim.x, aim.y, body)
	var yaw := deg_to_rad(wire.x)
	var pitch := deg_to_rad(-wire.y)
	var unity_dir := Vector3(sin(yaw) * cos(pitch), -sin(pitch), cos(yaw) * cos(pitch))
	# The same heading as a DCL-space direction (yaw 0 = +Z, pitch + = up).
	var dcl_yaw := deg_to_rad(wire.x)
	var dcl_pitch := deg_to_rad(wire.y)
	var dcl_dir := Vector3(
		sin(dcl_yaw) * cos(dcl_pitch), sin(dcl_pitch), cos(dcl_yaw) * cos(dcl_pitch)
	)
	if unity_dir.distance_to(dcl_dir) > TOL:
		_failures.append("%s: unity aims %v, camera is %v" % [label, unity_dir, dcl_dir])


func _check(label: String, got: Vector2, want: Vector2) -> void:
	if absf(got.x - want.x) > TOL or absf(got.y - want.y) > TOL:
		_failures.append("%s: got (%f, %f), want (%f, %f)" % [label, got.x, got.y, want.x, want.y])
