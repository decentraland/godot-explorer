extends SceneTree

# Regression for #3080 (head sync points the wrong way on a peer). The wire
# carries the sender's camera direction in WORLD space as Unity Euler yaw/pitch
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
	# Wire = world heading, independent of where the body faces.
	_check("send forward", HeadIK.world_dir_to_wire(Vector3(1.0, 0.0, 0.0)), Vector2(90.0, 0.0))
	_check("send up", HeadIK.world_dir_to_wire(Vector3(0.0, 0.8660254, -0.5)), Vector2(0.0, 60.0))
	_check("send zero", HeadIK.world_dir_to_wire(Vector3.ZERO), Vector2.ZERO)

	# Unity's decoder rebuilds the direction from the pair and compares it with
	# the avatar's head forward: DCL +Z is Godot -Z, so the Godot direction
	# (x, y, z) arrives as (x, y, -z) and the running peer turns the head by the
	# delta. Facing +X and looking forward used to send 0 (body-relative) and
	# Unity aimed 90 deg to the side.
	_check_unity_agrees("unity sees forward", Vector3(1.0, 0.0, 0.0), Vector2(90.0, 0.0))

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
	_round_trip("round trip forward", Vector3(1.0, 0.0, 0.0), BODY_EAST)
	_round_trip("round trip north", Vector3(0.0, 0.0, -1.0), BODY_EAST)
	_round_trip("round trip diagonal", Vector3(0.5, 0.5, -0.70710678), BODY_EAST)

	if _failures.is_empty():
		print("[test_head_ik_wire] PASS")
		quit(0)
	else:
		for failure in _failures:
			printerr("  " + failure)
		printerr("[test_head_ik_wire] FAIL: %d case(s)" % _failures.size())
		quit(1)


func _round_trip(label: String, look_dir: Vector3, body: Basis) -> void:
	var wire := HeadIK.world_dir_to_wire(look_dir)
	var body_aim := HeadIK.wire_to_body(wire.x, wire.y, body)
	var dir := body.orthonormalized().inverse() * look_dir.normalized()
	var horizontal := sqrt(dir.x * dir.x + dir.z * dir.z)
	var expected := Vector2(rad_to_deg(atan2(dir.x, -dir.z)), rad_to_deg(atan2(dir.y, horizontal)))
	_check(label, body_aim, expected)


func _check_unity_agrees(label: String, look_dir: Vector3, wire: Vector2) -> void:
	# Quaternion.Euler(pitch, yaw, 0) * Vector3.forward in Unity's left-handed
	# space, with the wire pitch positive down (what the encoder sends).
	var yaw := deg_to_rad(wire.x)
	var pitch := deg_to_rad(-wire.y)
	var unity_dir := Vector3(sin(yaw) * cos(pitch), -sin(pitch), cos(yaw) * cos(pitch))
	var dcl_camera_dir := Vector3(look_dir.x, look_dir.y, -look_dir.z).normalized()
	if unity_dir.distance_to(dcl_camera_dir) > TOL:
		_failures.append("%s: unity aims %v, camera is %v" % [label, unity_dir, dcl_camera_dir])


func _check(label: String, got: Vector2, want: Vector2) -> void:
	if absf(got.x - want.x) > TOL or absf(got.y - want.y) > TOL:
		_failures.append("%s: got (%f, %f), want (%f, %f)" % [label, got.x, got.y, want.x, want.y])
