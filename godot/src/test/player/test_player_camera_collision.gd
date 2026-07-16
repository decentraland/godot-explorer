extends SceneTree

# Regression test for the camera "see through walls" fix (issue #1814).
#
# Two root causes, both pinned here:
#
#  1. The camera SpringArm3D ("Mount") used the engine-default collision_mask = 1
#     (CL_POINTER). Solid world geometry is on CL_PHYSICS (layer 2) — many scene
#     colliders are physics-only — so the arm never shortened and the camera
#     passed straight through those walls. The fix sets mask = CL_PHYSICS (2),
#     matching the player body.
#
#  2. The over-shoulder offset (0.75) was applied by translating the SpringArm
#     pivot itself, moving it OUTSIDE the 0.25-radius collision capsule; the
#     lateral segment isn't collision-swept, so the cast could start behind a
#     wall. The fix keeps the pivot centered and applies the offset on the
#     Camera3D below the arm (via a CameraArm node), so the cast starts from a
#     protected point.
#
# The physics cases use synthetic Godot nodes only (no DclCamera3D / player.tscn),
# so they run headless without the Rust extension built:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/player/test_player_camera_collision.gd

const CameraRig := preload("res://src/logic/player/camera_rig_helpers.gd")

const SPRING_LENGTH := 3.0
# The arm extends its children BEHIND the pivot (+Z, since forward is -Z), so the
# camera sits behind the player; the wall must be on that +Z side to be cast onto.
const WALL_Z := 1.5

# Collision layer bit values.
const CL_POINTER := 1  # layer 1
const CL_PHYSICS := 2  # layer 2 — solid world geometry
const CL_POINTER_AND_PHYSICS := 3  # layer 1|2 — the DCL scene-collider default

var _failures: Array[String] = []


# gdlint:ignore = async-function-name
func _initialize() -> void:
	await _test_physics_wall_shortens_arm()
	await _test_old_default_mask_misses_physics_wall()
	await _test_pointer_only_wall_ignored()
	await _test_default_scene_collider_shortens_arm()
	_test_rig_targets_third_person()
	_test_rig_targets_first_person()
	_test_scene_pivot_centered_and_masked()
	_finish()


# Build a SpringArm3D with a wall at WALL_Z, step physics, and return the resolved
# arm length (a child marker is driven to (0,0,+length) behind the pivot).
# gdlint:ignore = async-function-name
func _resolve_spring(spring_mask: int, wall_layer: int) -> float:
	var world := Node3D.new()
	root.add_child(world)

	var wall := StaticBody3D.new()
	wall.collision_layer = wall_layer
	wall.collision_mask = 0
	var col := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(4, 4, 0.2)
	col.shape = box
	wall.add_child(col)
	wall.position = Vector3(0, 0, WALL_Z)
	world.add_child(wall)

	var arm := SpringArm3D.new()
	arm.spring_length = SPRING_LENGTH
	arm.margin = 0.0
	arm.collision_mask = spring_mask
	var marker := Node3D.new()
	arm.add_child(marker)
	world.add_child(arm)

	# The static body registers in the physics space on the next tick; the arm
	# queries that space when it updates. Step a couple of frames to settle.
	await physics_frame
	await physics_frame
	await process_frame

	var resolved: float = marker.position.z
	world.queue_free()
	return resolved


# The fix: with mask = CL_PHYSICS the arm shortens against a physics-only wall.
# gdlint:ignore = async-function-name
func _test_physics_wall_shortens_arm() -> void:
	var resolved := await _resolve_spring(CameraRig.CAMERA_COLLISION_MASK, CL_PHYSICS)
	if resolved >= SPRING_LENGTH - 0.01:
		_fail(
			(
				"physics wall: arm not shortened (resolved=%.2f, expected < %.2f)"
				% [resolved, SPRING_LENGTH]
			)
		)
	if resolved <= 0.0 or resolved > 1.7:
		_fail("physics wall: resolved=%.2f, expected near wall distance ~1.5" % resolved)


# The bug: the old engine default (mask 1 = CL_POINTER) does NOT see a physics
# wall, so the arm stays full length and the camera clips through.
# gdlint:ignore = async-function-name
func _test_old_default_mask_misses_physics_wall() -> void:
	var resolved := await _resolve_spring(CL_POINTER, CL_PHYSICS)
	if resolved < SPRING_LENGTH - 0.01:
		_fail(
			"old default mask=1 unexpectedly collided with physics wall (resolved=%.2f)" % resolved
		)


# The camera must ignore pointer-only (non-solid) colliders — no false shortening.
# gdlint:ignore = async-function-name
func _test_pointer_only_wall_ignored() -> void:
	var resolved := await _resolve_spring(CameraRig.CAMERA_COLLISION_MASK, CL_POINTER)
	if resolved < SPRING_LENGTH - 0.01:
		_fail("pointer-only wall wrongly collided (resolved=%.2f)" % resolved)


# The typical DCL scene collider (layer 1|2) is detected too.
# gdlint:ignore = async-function-name
func _test_default_scene_collider_shortens_arm() -> void:
	var resolved := await _resolve_spring(CameraRig.CAMERA_COLLISION_MASK, CL_POINTER_AND_PHYSICS)
	if resolved >= SPRING_LENGTH - 0.01:
		_fail("default scene collider (layer 1|2) not detected (resolved=%.2f)" % resolved)


func _test_rig_targets_third_person() -> void:
	var t := CameraRig.rig_targets(true)
	# Back distance is the pure Z, NOT the diagonal length — the lateral part is
	# handled by the camera offset, so baking it in would push the camera too far.
	_expect_eq("3rd person spring_length", CameraRig.THIRD_PERSON_CAMERA.z, t.spring_length)
	if is_equal_approx(t.spring_length, CameraRig.THIRD_PERSON_CAMERA.length()):
		_fail("3rd person spring_length must be the back distance (.z), not the diagonal")
	# Over-shoulder offset is a CAMERA offset (not applied to the pivot).
	_expect_eq("3rd person camera_offset_x", CameraRig.THIRD_PERSON_CAMERA.x, t.camera_offset_x)


func _test_rig_targets_first_person() -> void:
	var t := CameraRig.rig_targets(false)
	_expect_eq("1st person spring_length", CameraRig.FIRST_PERSON_SPRING_LENGTH, t.spring_length)
	_expect_eq("1st person camera_offset_x", 0.0, t.camera_offset_x)


# Guard the actual scene: the Mount pivot stays centered (no lateral X in its
# transform) and carries the CL_PHYSICS mask; the offset lives on a CameraArm
# node below it. Parsed from text so this needs no Rust extension.
func _test_scene_pivot_centered_and_masked() -> void:
	var text := FileAccess.get_file_as_string("res://src/logic/player/player.tscn")
	if text.is_empty():
		_fail("could not read player.tscn")
		return

	var mount_block := _node_block(text, "Mount")
	if mount_block.is_empty():
		_fail("Mount node not found in player.tscn")
		return

	if not mount_block.contains("collision_mask = %d" % CameraRig.CAMERA_COLLISION_MASK):
		_fail("Mount is missing collision_mask = %d (CL_PHYSICS)" % CameraRig.CAMERA_COLLISION_MASK)

	# Mount transform origin must be (x=0, y=1.71, z=0): pivot centered on the
	# player axis (no lateral over-shoulder offset baked into the pivot).
	if not mount_block.contains("0, 1.71, 0)"):
		_fail("Mount pivot is not centered on the player axis (expected origin x=0)")

	# The over-shoulder offset now lives below the arm on a CameraArm node.
	if not text.contains('name="CameraArm"') or not text.contains('parent="Mount"'):
		_fail("CameraArm node (parent=Mount) missing — offset relocation not applied")
	if not text.contains('parent="Mount/CameraArm"'):
		_fail("Camera3D is not parented under Mount/CameraArm")


# Return the .tscn text from `[node name="<name>" ...]` up to the next node header.
func _node_block(text: String, node_name: String) -> String:
	var marker := '[node name="%s"' % node_name
	var start := text.find(marker)
	if start == -1:
		return ""
	var next := text.find("\n[node ", start + marker.length())
	if next == -1:
		return text.substr(start)
	return text.substr(start, next - start)


func _expect_eq(ctx: String, expected: float, actual: float) -> void:
	if not is_equal_approx(expected, actual):
		_fail("%s: expected %s, got %s" % [ctx, expected, actual])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_player_camera_collision] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_player_camera_collision] FAIL: %d case(s)" % _failures.size())
	quit(1)
