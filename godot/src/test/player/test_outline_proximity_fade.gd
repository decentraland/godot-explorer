extends SceneTree

# Test for the outline proximity-fade suppression (issue #1814 follow-up).
# Outlined meshes render in the outline SubViewport with their real materials,
# so the camera proximity fade dithers them there too — the Sobel edge
# detector then reads the dither holes as edges everywhere (rainbow flood
# artifact). The outline is suppressed while the target's surface is inside
# the fade band.
#
# Run headless (no Rust extension needed):
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/player/test_outline_proximity_fade.gd

const Outline := preload("res://src/logic/player/outline_system.gd")
const Fade := preload("res://src/decentraland_components/proximity_fade.gd")

var _failures: Array[String] = []


# gdlint:ignore = async-function-name
func _initialize() -> void:
	_test_aabb_surface_distance()
	_test_compute_local_aabb()
	_test_suppression_threshold_vs_fade()
	_test_script_wiring()
	_finish()


func _test_aabb_surface_distance() -> void:
	var box := AABB(Vector3(-1, -1, -1), Vector3(2, 2, 2))  # unit cube at origin

	# Point 1m outside the +Z face.
	_expect_eq("outside face", 1.0, Outline.aabb_surface_distance(Vector3(0, 0, 2), box))
	# Point inside the box: distance 0 (surface reached).
	_expect_eq("inside", 0.0, Outline.aabb_surface_distance(Vector3(0.5, 0.5, 0.5), box))
	# Corner: 3m past the corner on each axis -> sqrt(3^2 * 3) = ~5.196.
	var corner := Outline.aabb_surface_distance(Vector3(4, 4, 4), box)
	if absf(corner - sqrt(27.0)) > 0.01:
		_fail("corner distance: expected ~5.196, got %.3f" % corner)


# gdlint:ignore = async-function-name
func _test_compute_local_aabb() -> void:
	var entity := Node3D.new()
	root.add_child(entity)

	var mesh_instance := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(2, 2, 2)  # local aabb: (-1,-1,-1)..(1,1,1)
	mesh_instance.mesh = box
	mesh_instance.position = Vector3(5, 0, 0)
	entity.add_child(mesh_instance)

	# A frame so global transforms are valid.
	await process_frame

	var aabb := Outline.compute_local_aabb(entity)
	# Mesh at local x=5 with size 2 -> local aabb x in [4, 6].
	_expect_eq("aabb begin x", 4.0, aabb.position.x)
	_expect_eq("aabb end x", 6.0, aabb.end.x)
	_expect_eq("aabb begin y", -1.0, aabb.position.y)
	entity.queue_free()


# The suppression threshold must be >= the fade start: the outline may only
# vanish once the dither is actually active (never while the object is solid).
func _test_suppression_threshold_vs_fade() -> void:
	var threshold: float = Fade.FADE_START_DISTANCE + Outline.OUTLINE_SUPPRESS_MARGIN
	if threshold < Fade.FADE_START_DISTANCE:
		_fail("suppression threshold below fade start — outline would vanish early")
	if Outline.OUTLINE_SUPPRESS_MARGIN <= 0.0:
		_fail("OUTLINE_SUPPRESS_MARGIN must be > 0 (Sobel must never see dither)")


# Guard the wiring: the per-frame gate must reference the fade band and the
# entity AABB cache must be refreshed on target change.
func _test_script_wiring() -> void:
	var code := FileAccess.get_file_as_string("res://src/logic/player/outline_system.gd")
	if code.is_empty():
		_fail("could not read outline_system.gd")
		return
	if not code.contains("ProximityFade.FADE_START_DISTANCE"):
		_fail("outline_system.gd does not reference the fade band")
	if not code.contains("_is_entity_outline_suppressed"):
		_fail("outline_system.gd is missing the suppression check")
	if not code.contains("compute_local_aabb(entity)"):
		_fail("set_outlined_entity must cache the entity AABB")


func _expect_eq(ctx: String, expected: float, actual: float) -> void:
	if not is_equal_approx(expected, actual):
		_fail("%s: expected %s, got %s" % [ctx, expected, actual])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_outline_proximity_fade] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_outline_proximity_fade] FAIL: %d case(s)" % _failures.size())
	quit(1)
