extends SceneTree

# Test for the avatar proximity fade (issue #1814). When the camera gets
# within arm's reach of an avatar's head — the local player pinned against a
# wall, or any avatar in the camera's personal space — the WHOLE avatar
# dissolves uniformly (object-level), driven by a per-instance `own_fade`
# shader param + dither discard in the avatar shaders. Attached to every
# avatar by avatar.gd; only the explorer's world camera drives it.
#
# Run headless (the Rust extension loads from lib/target, providing
# DclCamera3D for the camera gate):
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/player/test_avatar_proximity_fade.gd

const Fade := preload("res://src/logic/player/avatar_proximity_fade.gd")

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
	await _test_subviewport_avatar_does_not_fade()
	_test_consts_sane()
	_test_avatar_shaders_support_own_fade()
	_test_avatar_gd_wires_the_node()
	_finish()


# Head sits at avatar origin + HEAD_HEIGHT; rig at the origin keeps math trivial.
func _build_rig() -> Dictionary:
	var avatar := Node3D.new()
	avatar.name = "Avatar"
	root.add_child(avatar)

	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	var shader := Shader.new()
	shader.code = FADE_SHADER_CODE
	var mat := ShaderMaterial.new()
	mat.shader = shader
	mesh.material_override = mat
	avatar.add_child(mesh)

	var fade: AvatarProximityFade = Fade.new()
	avatar.add_child(fade)

	var camera := DclCamera3D.new()
	root.add_child(camera)
	camera.make_current()

	return {"avatar": avatar, "mesh": mesh, "camera": camera}


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
	var t := await _fade_at(rig["mesh"], rig["camera"], Vector3(0, Fade.HEAD_HEIGHT, 3.0))
	if t > 0.01:
		_fail("far camera: avatar fade = %.2f, expected 0 (fully opaque)" % t)
	rig["avatar"].queue_free()
	rig["camera"].queue_free()


# Camera pulled to the head (collision clamp against a wall): fully faded.
# gdlint:ignore = async-function-name
func _test_close_camera_fades_whole_avatar() -> void:
	var rig := _build_rig()
	var t := await _fade_at(rig["mesh"], rig["camera"], Vector3(0, Fade.HEAD_HEIGHT, 0.3))
	if t < 0.99:
		_fail("close camera: avatar fade = %.2f, expected 1 (fully faded)" % t)
	rig["avatar"].queue_free()
	rig["camera"].queue_free()


# gdlint:ignore = async-function-name
func _test_mid_distance_partial_fade() -> void:
	var rig := _build_rig()
	var mid: float = (Fade.FADE_START_DISTANCE + Fade.FADE_GONE_DISTANCE) * 0.5
	var t := await _fade_at(rig["mesh"], rig["camera"], Vector3(0, Fade.HEAD_HEIGHT, mid))
	if not is_equal_approx(t, 0.5):
		_fail("mid camera: avatar fade = %.2f, expected ~0.5" % t)
	rig["avatar"].queue_free()
	rig["camera"].queue_free()


# Preview avatars live in SubViewports (backpack, passport, impostor
# capture): they must NOT fade even with a close-up camera.
# gdlint:ignore = async-function-name
func _test_subviewport_avatar_does_not_fade() -> void:
	var sub := SubViewport.new()
	sub.size = Vector2i(64, 64)
	root.add_child(sub)

	var avatar := Node3D.new()
	sub.add_child(avatar)
	var mesh := MeshInstance3D.new()
	mesh.mesh = BoxMesh.new()
	var shader := Shader.new()
	shader.code = FADE_SHADER_CODE
	var mat := ShaderMaterial.new()
	mat.shader = shader
	mesh.material_override = mat
	avatar.add_child(mesh)
	var fade: AvatarProximityFade = Fade.new()
	avatar.add_child(fade)

	var cam := Camera3D.new()
	sub.add_child(cam)
	cam.make_current()
	cam.global_position = Vector3(0, Fade.HEAD_HEIGHT, 0.3)
	await process_frame
	await process_frame
	var value: Variant = mesh.get_instance_shader_parameter("own_fade")
	if value != null and value != 0.0:
		_fail("subviewport avatar: fade = %s, expected unset/0" % value)
	sub.queue_free()


func _test_consts_sane() -> void:
	if Fade.FADE_GONE_DISTANCE <= 0.0:
		_fail("FADE_GONE_DISTANCE must be > 0")
	if Fade.FADE_GONE_DISTANCE >= Fade.FADE_START_DISTANCE:
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


# Every avatar gets the node from avatar.gd (local player, remote, NPCs).
func _test_avatar_gd_wires_the_node() -> void:
	var code := FileAccess.get_file_as_string("res://src/decentraland_components/avatar/avatar.gd")
	if code.is_empty():
		_fail("could not read avatar.gd")
		return
	if not code.contains("AvatarProximityFade.new()"):
		_fail("avatar.gd does not attach AvatarProximityFade to every avatar")


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_proximity_fade] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_avatar_proximity_fade] FAIL: %d case(s)" % _failures.size())
	quit(1)
