extends SceneTree

# Test for the own-avatar proximity fade (issue #1814). When the camera is
# forced into the local avatar's head, the WHOLE avatar dissolves uniformly
# (object-level) — not per-fragment, which would show a cross-section of the
# skull. Mechanism: per-instance shader param `own_fade` + dither discard in
# the avatar shaders (GeometryInstance3D.transparency is ignored by the mobile
# renderer for opaque materials — verified empirically with a rendered probe).
# Remote avatars never get the node, so they're unaffected.
#
# Run headless (no Rust extension needed):
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/player/test_own_avatar_proximity_fade.gd

const OwnFade := preload("res://src/logic/player/own_avatar_proximity_fade.gd")

# Mirrors the own_fade contract of the dcl_toon / mask shaders.
const FADE_SHADER_CODE := """
shader_type spatial;
instance uniform float own_fade : hint_range(0.0, 1.0) = 0.0;
void fragment() { ALBEDO = vec3(1.0); }
"""

var _failures: Array[String] = []


# gdlint:ignore = async-function-name
func _initialize() -> void:
	await _test_far_camera_keeps_avatar_opaque()
	await _test_close_camera_fades_whole_avatar()
	await _test_mid_distance_partial_fade()
	_test_consts_sane()
	_test_avatar_shaders_support_own_fade()
	_test_scene_guard()
	_finish()


# Mirror of the player.tscn structure: Player > Mount + Avatar > meshes + fade.
func _build_rig() -> Dictionary:
	var player := Node3D.new()
	player.name = "Player"
	root.add_child(player)

	var mount := Node3D.new()
	mount.name = "Mount"
	mount.position = Vector3(0, 1.71, 0)
	player.add_child(mount)

	var avatar := Node3D.new()
	avatar.name = "Avatar"
	player.add_child(avatar)

	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	var shader := Shader.new()
	shader.code = FADE_SHADER_CODE
	var mat := ShaderMaterial.new()
	mat.shader = shader
	mesh.material_override = mat
	avatar.add_child(mesh)

	var fade: OwnAvatarProximityFade = OwnFade.new()
	avatar.add_child(fade)

	var camera := Camera3D.new()
	root.add_child(camera)
	camera.make_current()

	return {"player": player, "mesh": mesh, "camera": camera}


# gdlint:ignore = async-function-name
func _fade_at(mesh: MeshInstance3D, camera: Camera3D, pos: Vector3) -> float:
	camera.global_position = pos
	await process_frame
	await process_frame
	var value: Variant = mesh.get_instance_shader_parameter("own_fade")
	if value == null:
		_fail("own_fade instance param was never set on the mesh")
		return -1.0
	return value


# gdlint:ignore = async-function-name
func _test_far_camera_keeps_avatar_opaque() -> void:
	var rig := _build_rig()
	var t := await _fade_at(rig["mesh"], rig["camera"], Vector3(0, 1.71, 3.0))
	if t > 0.01:
		_fail("far camera: avatar fade = %.2f, expected 0 (fully opaque)" % t)
	rig["player"].queue_free()
	rig["camera"].queue_free()


# The reported case: camera pulled to the clamp's minimum distance (0.3) sits
# right at the head — the whole avatar must be fully transparent.
# gdlint:ignore = async-function-name
func _test_close_camera_fades_whole_avatar() -> void:
	var rig := _build_rig()
	var t := await _fade_at(rig["mesh"], rig["camera"], Vector3(0, 1.71, 0.3))
	if t < 0.99:
		_fail("close camera: avatar fade = %.2f, expected 1 (fully faded)" % t)
	rig["player"].queue_free()
	rig["camera"].queue_free()


# gdlint:ignore = async-function-name
func _test_mid_distance_partial_fade() -> void:
	var rig := _build_rig()
	var mid: float = (OwnFade.FADE_START_DISTANCE + OwnFade.FADE_GONE_DISTANCE) * 0.5
	var t := await _fade_at(rig["mesh"], rig["camera"], Vector3(0, 1.71, mid))
	if not is_equal_approx(t, 0.5):
		_fail("mid camera: avatar fade = %.2f, expected ~0.5" % t)
	rig["player"].queue_free()
	rig["camera"].queue_free()


func _test_consts_sane() -> void:
	if OwnFade.FADE_GONE_DISTANCE <= 0.0:
		_fail("FADE_GONE_DISTANCE must be > 0")
	if OwnFade.FADE_GONE_DISTANCE >= OwnFade.FADE_START_DISTANCE:
		_fail("FADE_GONE_DISTANCE must be < FADE_START_DISTANCE")


# The fade only acts on meshes whose shaders declare the own_fade uniform —
# pin that the avatar shaders actually carry it.
func _test_avatar_shaders_support_own_fade() -> void:
	var shaders := [
		"res://assets/avatar/dcl_toon.gdshader",
		"res://assets/avatar/dcl_toon_double.gdshader",
		"res://assets/avatar/dcl_toon_alpha_clip.gdshader",
		"res://assets/avatar/dcl_toon_alpha_blend.gdshader",
		"res://assets/avatar/mask_material.gdshader",
	]
	for path in shaders:
		var code := FileAccess.get_file_as_string(path)
		if code.is_empty():
			_fail("could not read " + path)
			continue
		if not code.contains("instance uniform float own_fade"):
			_fail(path + " is missing the own_fade instance uniform")
		if not code.contains("discard"):
			_fail(path + " is missing the own_fade dither discard")


# The node must live under the LOCAL player's Avatar in player.tscn — that is
# what scopes the effect to the own avatar only.
func _test_scene_guard() -> void:
	var text := FileAccess.get_file_as_string("res://src/logic/player/player.tscn")
	if text.is_empty():
		_fail("could not read player.tscn")
		return
	if not text.contains('name="OwnAvatarProximityFade"'):
		_fail("OwnAvatarProximityFade node missing in player.tscn")
	if not text.contains('parent="Avatar"'):
		_fail("OwnAvatarProximityFade must be a child of the Avatar node")


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_own_avatar_proximity_fade] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_own_avatar_proximity_fade] FAIL: %d case(s)" % _failures.size())
	quit(1)
