extends SceneTree

# Regression test for the camera proximity fade — the visual safety net of the
# see-through-walls fix (issue #1814). When the camera still ends up inside a
# mesh whose collider doesn't match its visuals (or doesn't exist), geometry
# within FADE_GONE_DISTANCE of the camera is fully dithered out, fading in up
# to FADE_START_DISTANCE — a soft dissolve instead of black inside-out faces.
#
# Run headless (no Rust extension needed):
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/decentraland_components/test_proximity_fade.gd

const Fade := preload("res://src/decentraland_components/proximity_fade.gd")
const CameraRig := preload("res://src/logic/player/camera_rig_helpers.gd")

var _failures: Array[String] = []


# gdlint:ignore = async-function-name
func _initialize() -> void:
	_test_apply_material_sets_distance_fade()
	_test_fade_band_is_sane()
	_test_gltf_container_hook()
	_test_rust_material_hook()
	_test_spring_margin_keeps_pressed_wall_visible()
	_test_clamp_radius_keeps_pressed_wall_visible()
	_finish()


func _test_apply_material_sets_distance_fade() -> void:
	var mat := StandardMaterial3D.new()
	Fade.apply_material(mat)
	if mat.distance_fade_mode != BaseMaterial3D.DISTANCE_FADE_PIXEL_DITHER:
		_fail("distance_fade_mode: expected PIXEL_DITHER, got %s" % mat.distance_fade_mode)
	_expect_eq("min_distance", Fade.FADE_GONE_DISTANCE, mat.distance_fade_min_distance)
	_expect_eq("max_distance", Fade.FADE_START_DISTANCE, mat.distance_fade_max_distance)


func _test_fade_band_is_sane() -> void:
	if Fade.FADE_GONE_DISTANCE <= 0.0:
		_fail("FADE_GONE_DISTANCE must be > 0 (got %s)" % Fade.FADE_GONE_DISTANCE)
	if Fade.FADE_GONE_DISTANCE >= Fade.FADE_START_DISTANCE:
		_fail(
			(
				"FADE_GONE_DISTANCE (%s) must be < FADE_START_DISTANCE (%s)"
				% [Fade.FADE_GONE_DISTANCE, Fade.FADE_START_DISTANCE]
			)
		)


# The GLTF chokepoint must apply the fade to every imported scene material.
func _test_gltf_container_hook() -> void:
	var text := FileAccess.get_file_as_string("res://src/decentraland_components/gltf_container.gd")
	if text.is_empty():
		_fail("could not read gltf_container.gd")
		return
	if not text.contains("ProximityFade.apply_material"):
		_fail("gltf_container.gd does not call ProximityFade.apply_material (hook missing)")


# The SDK Material chokepoint lives in Rust (lib/, outside res://). Read it via
# the absolute path so this stays a real guard.
func _test_rust_material_hook() -> void:
	var path := ProjectSettings.globalize_path(
		"res://../lib/src/scene_runner/components/material.rs"
	)
	var text := FileAccess.get_file_as_string(path)
	if text.is_empty():
		_fail("could not read %s" % path)
		return
	if not text.contains("PIXEL_DITHER"):
		_fail("material.rs is missing the distance fade (PIXEL_DITHER) hook")
	if not text.contains("proximity_fade.gd"):
		_fail("material.rs must reference proximity_fade.gd (values must stay in sync)")


# The SpringArm stops the camera `margin` meters from a collider face. A wall
# the camera is legitimately pressed against must sit at/above the start of the
# fade band — otherwise the fade would dissolve walls that are NOT clipped.
func _test_spring_margin_keeps_pressed_wall_visible() -> void:
	var text := FileAccess.get_file_as_string("res://src/logic/player/player.tscn")
	if text.is_empty():
		_fail("could not read player.tscn")
		return
	var mount_block := _node_block(text, "Mount")
	if mount_block.is_empty():
		_fail("Mount node not found in player.tscn")
		return

	var margin := _extract_margin(mount_block)
	if margin < 0.0:
		_fail("Mount has no margin property")
		return
	if margin < Fade.FADE_START_DISTANCE:
		_fail(
			(
				"Mount margin (%.2f) < FADE_START_DISTANCE (%.2f): a pressed wall would dither"
				% [margin, Fade.FADE_START_DISTANCE]
			)
		)


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


func _extract_margin(block: String) -> float:
	for line in block.split("\n"):
		var stripped := line.strip_edges()
		if stripped.begins_with("margin"):
			var parts := stripped.split("=")
			if parts.size() == 2:
				return parts[1].strip_edges().to_float()
	return -1.0


# Same invariant for the CameraCollisionClamp: its sphere radius is the
# standoff distance, so a wall pressed against it must stay at/above fade start.
func _test_clamp_radius_keeps_pressed_wall_visible() -> void:
	if CameraRig.CLAMP_SPHERE_RADIUS < Fade.FADE_START_DISTANCE:
		_fail(
			(
				"CLAMP_SPHERE_RADIUS (%.2f) < FADE_START_DISTANCE (%.2f): a pressed wall would dither"
				% [CameraRig.CLAMP_SPHERE_RADIUS, Fade.FADE_START_DISTANCE]
			)
		)


func _expect_eq(ctx: String, expected: float, actual: float) -> void:
	if not is_equal_approx(expected, actual):
		_fail("%s: expected %s, got %s" % [ctx, expected, actual])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_proximity_fade] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_proximity_fade] FAIL: %d case(s)" % _failures.size())
	quit(1)
