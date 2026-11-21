extends Node

# Base eyes ship a colour texture and no _mask.png of their own, so the facial material had nothing
# to tint and the eye colour was dropped (#2439). The bundled iris masks are picked by the eye
# texture's content hash; a texture with no bundled mask must stay untinted instead of tinting the
# sclera, and the eyes whose iris is a solid black disc must replace it, since a multiply on black
# stays black.

const EYES_02_HASH := "bafkreidbeafduyssl7y2gdjoxq3ouagn6beivty4v6eovpl5iz2z72jeqm"
const EYES_03_HASH := "bafkreieo2imr2lvlfiesq4ollve3oflwwlmfmjg5pjytz46kc5gontdjxm"
const UNKNOWN_HASH := "bafyreyeinternalplaceholder"

var _failures := 0


func _ready() -> void:
	_check_base_eye_gets_its_bundled_mask()
	_check_every_bundled_mask_resolves()
	_check_solid_black_iris_replaces_instead_of_multiplying()
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
	_expect(
		material.get_shader_parameter("mask_replace") == 0.0,
		"a shaded iris must multiply, not replace, so its pupil and shading survive"
	)


func _check_every_bundled_mask_resolves() -> void:
	for hash_key in EyeMasks.BASE_EYE_MASKS:
		var path: String = EyeMasks.BASE_EYE_MASKS[hash_key]
		_expect(ResourceLoader.exists(path), "bundled mask missing for %s: %s" % [hash_key, path])
		_expect(
			(
				_apply([hash_key], Color.GREEN, Color.WHITE).get_shader_parameter("mask_texture")
				is Texture2D
			),
			"base eye %s resolved no mask, so its eye color does nothing (#2439)" % hash_key
		)


func _check_solid_black_iris_replaces_instead_of_multiplying() -> void:
	var material := _apply([EYES_03_HASH], Color.GREEN, Color.WHITE)

	_expect(
		material.get_shader_parameter("mask_texture") is Texture2D,
		"the solid black iris eye has no bundled mask"
	)
	_expect(
		material.get_shader_parameter("mask_replace") == 1.0,
		"a solid black iris stays black when multiplied, so it must replace (#2439)"
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
