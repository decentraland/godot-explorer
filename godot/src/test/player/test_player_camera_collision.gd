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
const Clamp := preload("res://src/logic/player/camera_collision_clamp.gd")

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
	await _test_clamp_full_offset_when_clear()
	await _test_clamp_catches_lateral_wall()
	await _test_clamp_back_to_wall_overlap()
	await _test_clamp_corner_two_walls()
	await _test_clamp_thin_door_orbit()
	await _test_clamp_whiskers_catch_bulge_beside_line()
	await _test_contact_sphere_depenetrates_parallel_wall()
	await _test_clamp_fully_blocked_stays_out_of_wall()
	_test_rig_targets_third_person()
	_test_rig_targets_first_person()
	_test_scene_pivot_centered_and_masked()
	_test_scene_has_collision_clamp()
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


# --- CameraCollisionClamp: sphere swept to the ACTUAL offset camera position ---


# Mirror of the player.tscn rig: Mount (SpringArm3D) -> CameraArm -> Camera3D,
# plus the clamp node as a child of Mount. Returns the nodes for assertions.
func _build_clamp_rig() -> Dictionary:
	var world := Node3D.new()
	root.add_child(world)

	var mount := SpringArm3D.new()
	mount.spring_length = CameraRig.THIRD_PERSON_CAMERA.z
	mount.margin = 0.0
	mount.collision_mask = CameraRig.CAMERA_COLLISION_MASK
	world.add_child(mount)

	var arm := Node3D.new()
	arm.name = "CameraArm"
	mount.add_child(arm)

	var cam := Camera3D.new()
	cam.name = "Camera3D"
	arm.add_child(cam)

	var clamp_node: CameraCollisionClamp = Clamp.new()
	mount.add_child(clamp_node)
	clamp_node.lateral_offset = CameraRig.THIRD_PERSON_CAMERA.x

	return {"world": world, "mount": mount, "arm": arm, "cam": cam}


func _add_box(world: Node3D, center: Vector3, size: Vector3) -> void:
	var wall := StaticBody3D.new()
	wall.collision_layer = CL_PHYSICS
	wall.collision_mask = 0
	var col := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = size
	col.shape = box
	wall.add_child(col)
	wall.position = center
	world.add_child(wall)


# Physics frames settle the spring; process frames run the clamp (which lives
# in _process). Extend smoothing uses real-time delta, so give it several.
# gdlint:ignore = async-function-name
func _settle() -> void:
	await physics_frame
	await physics_frame
	await physics_frame
	for i in range(20):
		await process_frame


# No walls: the clamp must preserve the exact over-shoulder framing — camera at
# the full lateral offset + back distance.
# gdlint:ignore = async-function-name
func _test_clamp_full_offset_when_clear() -> void:
	var rig := _build_clamp_rig()
	await _settle()

	var expected := Vector3(CameraRig.THIRD_PERSON_CAMERA.x, 0, CameraRig.THIRD_PERSON_CAMERA.z)
	var cam: Camera3D = rig["cam"]
	if cam.global_position.distance_to(expected) > 0.05:
		_fail(
			(
				"clamp moved the camera with no walls around (pos=%s, expected ~%s)"
				% [cam.global_position, expected]
			)
		)
	rig["world"].queue_free()


# The reviewer case: a wall on the over-shoulder SIDE. The arm's centered ray
# passes clean (arm stays full length), but the offset camera would end up
# inside the wall — the clamp's sphere must pull it in front of the box.
# gdlint:ignore = async-function-name
func _test_clamp_catches_lateral_wall() -> void:
	var rig := _build_clamp_rig()
	# Box occupying x[0.5,1.5], z[2.5,3.5]: the unclamped camera (0.75,0,3) is
	# inside it, while the arm's centered ray at x=0 misses it completely.
	_add_box(rig["world"], Vector3(1.0, 0, 3.0), Vector3(1, 2, 1))
	await _settle()

	var arm: Node3D = rig["arm"]
	if arm.position.z < CameraRig.THIRD_PERSON_CAMERA.z - 0.01:
		_fail(
			(
				"setup broken: the arm's centered ray should NOT hit the lateral wall "
				+ "(arm z=%.2f, expected %.2f)" % [arm.position.z, CameraRig.THIRD_PERSON_CAMERA.z]
			)
		)

	var cam: Camera3D = rig["cam"]
	# Pulled in front of the box's near face (2.5) minus the sphere radius (0.4),
	# and clearly not collapsed to the pivot.
	if cam.global_position.z > 2.2:
		_fail(
			(
				"lateral wall: camera still inside the wall volume (pos=%s, z expected < 2.2)"
				% cam.global_position
			)
		)
	if cam.global_position.z < 1.0:
		_fail("lateral wall: camera over-shrunk (pos=%s)" % cam.global_position)
	rig["world"].queue_free()


# THE probe case: player's back against a wall (pivot 0.25m from the wall
# face). An r=0.4 sphere cast FROM the pivot already overlaps the wall, and
# cast_motion ignores start overlaps (reports all-clear) — the camera used to
# sail through. The mount-forward shifted cast must catch it and keep the
# camera out of the wall (z must stay below the 0.25 wall face).
# gdlint:ignore = async-function-name
func _test_clamp_back_to_wall_overlap() -> void:
	var rig := _build_clamp_rig()
	# Wall face at z=0.25 (box z: 0.25..0.45), pivot at the origin.
	_add_box(rig["world"], Vector3(0, 0, 0.35), Vector3(4, 4, 0.2))
	await _settle()

	var cam: Camera3D = rig["cam"]
	if cam.global_position.z > 0.2:
		_fail(
			"back-to-wall: camera inside the wall (pos=%s, z expected <= 0.2)" % cam.global_position
		)
	if cam.global_position.z < -0.6:
		_fail("back-to-wall: camera shot off too far forward (pos=%s)" % cam.global_position)
	rig["world"].queue_free()


# Corner case (the reported door scenario): the mount-forward shifted origin
# lands INSIDE the thin wall in front (W1) — the sphere cast goes blind
# (start overlap is ignored) — and the camera would cross the thin wall
# BEHIND the player (W2). The ray backbone from the pivot must catch W2.
# gdlint:ignore = async-function-name
func _test_clamp_corner_two_walls() -> void:
	var rig := _build_clamp_rig()
	_add_box(rig["world"], Vector3(0, 0, -0.35), Vector3(4, 4, 0.2))  # W1 front: z -0.45..-0.25
	_add_box(rig["world"], Vector3(0, 0, 0.35), Vector3(4, 4, 0.2))  # W2 behind: z 0.25..0.45
	await _settle()

	var cam: Camera3D = rig["cam"]
	# Must stay out of W2's volume (face at 0.25) AND keep near-plane
	# clearance: pre-fix the camera rested at z=0.244 — 6mm from the face, so
	# the 0.05 near plane poked into the wall ("justo al borde" report).
	if cam.global_position.z > 0.25 - CameraRig.CLAMP_NEAR_CLEARANCE + 0.01:
		_fail(
			(
				"corner: camera rests at the wall face without near-plane clearance "
				+ (
					"(pos=%s, z expected <= %.2f)"
					% [cam.global_position, 0.25 - CameraRig.CLAMP_NEAR_CLEARANCE + 0.01]
				)
			)
		)
	rig["world"].queue_free()


# The reported scenario: hugging a THIN door box and orbiting 180° so the
# camera swings to the far side — the camera must never end up on the
# opposite side of the door from the avatar.
# gdlint:ignore = async-function-name
func _test_clamp_thin_door_orbit() -> void:
	var rig := _build_clamp_rig()
	# Thin door in front: z -0.45..-0.25.
	_add_box(rig["world"], Vector3(0, 0, -0.35), Vector3(4, 4, 0.2))
	# Camera orbited to the far side of the door.
	(rig["mount"] as SpringArm3D).rotation.y = PI
	await _settle()

	var cam: Camera3D = rig["cam"]
	if cam.global_position.z < -0.24:
		_fail(
			(
				"thin door orbit: camera crossed the door (pos=%s, z expected >= -0.24)"
				% cam.global_position
			)
		)
	rig["world"].queue_free()


func _add_cylinder(world: Node3D, center: Vector3, radius: float, height: float) -> void:
	var body := StaticBody3D.new()
	body.collision_layer = CL_PHYSICS
	body.collision_mask = 0
	var col := CollisionShape3D.new()
	var cyl := CylinderShape3D.new()
	cyl.radius = radius
	cyl.height = height
	col.shape = cyl
	body.add_child(col)
	body.position = center
	world.add_child(body)


# The live Genesis Plaza case: a curved column BESIDE the camera line. The
# shifted sphere is blind (its origin is overlapped by a wall in front) and
# the CENTER ray just misses the column — but the offset camera rests with
# the near plane on its curved surface. The outer whisker ray must catch it.
# gdlint:ignore = async-function-name
func _test_clamp_whiskers_catch_bulge_beside_line() -> void:
	var rig := _build_clamp_rig()
	# Wall in front: overlaps the shifted origin -> sphere untrusted.
	_add_box(rig["world"], Vector3(0, 0, -0.35), Vector3(4, 4, 0.2))
	# Column at (1.3, 0, 3.0) r=0.5: the segment's closest approach is ~0.53
	# (center ray misses by 3cm); the +0.4 whisker passes ~0.18 from the
	# center (hits). Without whiskers the camera ends at (0.75,0,3), 5cm from
	# the column surface — near plane on the wall.
	_add_cylinder(rig["world"], Vector3(1.3, 0, 3.0), 0.5, 4.0)
	await _settle()

	var cam: Camera3D = rig["cam"]
	# The camera must keep real clearance from the column's curved surface
	# (center (1.3,0,3), r=0.5): without whiskers it rests at (0.75,0,3) —
	# 0.55 from the center, i.e. 5cm from the surface, near plane on the wall.
	var dist_to_column := Vector2(cam.global_position.x - 1.3, cam.global_position.z - 3.0).length()
	if dist_to_column < 0.6:
		_fail(
			(
				"column bulge: camera too close to the curved surface (pos=%s, dist=%.2f < 0.6)"
				% [cam.global_position, dist_to_column]
			)
		)
	rig["world"].queue_free()


# The live Genesis Plaza case (system 2): a wall PARALLEL to the camera
# segment on the over-shoulder side. The whiskers and rays never cross it
# (the camera path stays x < 0.8), the sweep sphere is blind (origin
# overlapped by W1), and pull-in along the segment can't help — clearance to
# a parallel wall is constant. The contact sphere must depenetrate the
# camera LATERALLY, without pulling it in.
# gdlint:ignore = async-function-name
func _test_contact_sphere_depenetrates_parallel_wall() -> void:
	var rig := _build_clamp_rig()
	# W1 in front: blinds the sweep sphere (start overlap).
	_add_box(rig["world"], Vector3(0, 0, -0.35), Vector3(4, 4, 0.2))
	# Thin parallel wall: face at x=0.92, z in [2.5, 6] — placed so every
	# longitudinal cast slips past it (ray paths cross neither its side nor
	# its front face), while the offset camera rests ~0.17 from the face.
	_add_box(rig["world"], Vector3(0.945, 0, 4.25), Vector3(0.05, 4, 3.5))
	await _settle()

	var cam: Camera3D = rig["cam"]
	if cam.global_position.x > 0.74:
		_fail(
			(
				"parallel wall: camera rests at the wall (pos=%s, x expected <= 0.74)"
				% cam.global_position
			)
		)
	if cam.global_position.z < 2.5:
		_fail(
			(
				"parallel wall: camera wrongly pulled in (pos=%s, z expected > 2.5)"
				% cam.global_position
			)
		)

	# Anti-flicker: while the contact persists the depenetrated position must
	# be a stable fixed point — the previous stateful offset decayed each
	# frame and swung the camera back INTO the wall every other frame.
	var pos_a := cam.global_position
	for i in range(5):
		await process_frame
	var pos_b := cam.global_position
	if pos_a.distance_to(pos_b) > 0.05:
		_fail("parallel wall: camera oscillates between frames (%s -> %s)" % [pos_a, pos_b])
	rig["world"].queue_free()


# Fully blocked (wall right behind the pivot): the camera pulls in front of
# the wall face keeping the sphere's standoff — no artificial distance floor.
# gdlint:ignore = async-function-name
func _test_clamp_fully_blocked_stays_out_of_wall() -> void:
	var rig := _build_clamp_rig()
	_add_box(rig["world"], Vector3(0, 0, 1.0), Vector3(4, 4, 1))  # face at z=0.5
	await _settle()

	var cam: Camera3D = rig["cam"]
	# Must stop clearly in front of the 0.5 wall face (the camera is projected
	# back onto the arm's line, so it stands a bit closer than the sphere's
	# center does — what matters is it never enters the wall).
	if cam.global_position.z > 0.35:
		_fail(
			(
				"blocked wall: camera too close to/inside the wall (pos=%s, z expected <= 0.35)"
				% cam.global_position
			)
		)
	rig["world"].queue_free()


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


# Guard the clamp wiring: the node must exist under Mount in player.tscn, and
# player.gd must tween its lateral_offset (not the camera position directly).
func _test_scene_has_collision_clamp() -> void:
	var tscn := FileAccess.get_file_as_string("res://src/logic/player/player.tscn")
	if tscn.is_empty():
		_fail("could not read player.tscn")
		return
	if not tscn.contains('name="CameraCollisionClamp"'):
		_fail("CameraCollisionClamp node missing in player.tscn")
	var clamp_block := _node_block(tscn, "CameraCollisionClamp")
	if not clamp_block.contains('parent="Mount"'):
		_fail("CameraCollisionClamp must be a child of Mount")

	var gd := FileAccess.get_file_as_string("res://src/logic/player/player.gd")
	if gd.is_empty():
		_fail("could not read player.gd")
		return
	if not gd.contains('"lateral_offset"'):
		_fail("player.gd must tween camera_collision_clamp lateral_offset")
	if gd.contains('tween_property(camera, "position:x"'):
		_fail("player.gd still tweens camera position:x — must go through the clamp")


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
