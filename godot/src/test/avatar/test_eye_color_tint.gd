extends Node

# Base eyes ship a color texture and no _mask.png, so the facial material got a null mask and the
# eye color was never applied (#2439). Their bundled iris masks must be picked by texture content
# hash, and a texture with no mask of its own must stay untinted instead of tinting the sclera.

const EYES_02_HASH := "bafkreidbeafduyssl7y2gdjoxq3ouagn6beivty4v6eovpl5iz2z72jeqm"
const UNKNOWN_HASH := "bafyreyeinternalplaceholder"

var _failures := 0


func _ready() -> void:
	_check_base_eye_gets_its_bundled_mask()
	_check_unknown_maskless_texture_stays_untinted()
	_check_maskless_mouth_keeps_tinting_the_whole_texture()

	if _failures > 0:
		print("[test_eye_color_tint] FAIL (%d)" % _failures)
		get_tree().quit(1)
		return
	print("[test_eye_color_tint] PASS")
	get_tree().quit(0)


func _make_mesh() -> MeshInstance3D:
	var mesh := MeshInstance3D.new()
	mesh.mesh = QuadMesh.new()
	add_child(mesh)
	return mesh


func _apply(textures: Array, color: Color, mask_color: Color) -> Material:
	var mesh := _make_mesh()
	Avatar.apply_texture_and_mask(mesh, textures, color, mask_color)
	return mesh.get_surface_override_material(0)


func _expect(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		print("[test_eye_color_tint] FAIL: %s" % message)


func _check_base_eye_gets_its_bundled_mask() -> void:
	var material := _apply([EYES_02_HASH], Color.GREEN, Color.WHITE)
	var mask = material.get_shader_parameter("mask_texture")

	_expect(
		mask is Texture2D, "base eye has no bundled mask, so the eye color cannot apply (#2439)"
	)
	if mask is Texture2D:
		_expect(
			(mask as Texture2D).resource_path.ends_with("eyes_02_mask.png"),
			"base eye picked the wrong bundled mask: %s" % (mask as Texture2D).resource_path
		)
	_expect(
		material.get_shader_parameter("mask_color") == Color.WHITE,
		"eyes must keep the white mask_color so only the iris is tinted"
	)


func _check_unknown_maskless_texture_stays_untinted() -> void:
	var material := _apply([UNKNOWN_HASH], Color.GREEN, Color.WHITE)

	_expect(
		material.get_shader_parameter("mask_texture") == null,
		"a maskless texture with no bundled mask must stay untinted, not tint the sclera"
	)
	_expect(
		material.get_shader_parameter("mask_color") == Color.WHITE,
		"unknown maskless eyes must not fall back to a whole-texture tint"
	)


func _check_maskless_mouth_keeps_tinting_the_whole_texture() -> void:
	var skin := Color(0.8, 0.6, 0.5)
	var material := _apply([UNKNOWN_HASH], skin, Color.BLACK)

	_expect(
		material.get_shader_parameter("mask_texture") == null, "a maskless mouth keeps a null mask"
	)
	_expect(
		material.get_shader_parameter("mask_color") == Color.BLACK,
		"mouth and eyebrows still tint the whole texture through mask_color"
	)
	_expect(material.get_shader_parameter("material_color") == skin, "mouth keeps the skin color")
