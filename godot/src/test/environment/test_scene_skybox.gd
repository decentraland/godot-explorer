extends RefCounted

# `SceneSkybox` mirrors the proto's piecewise-linear gradient rules in GDScript (the
# Rust side only sorts and clamps the keys) and builds the time-of-day LUTs the sky
# shader samples. Both are pure functions, so they are tested here without a GPU.

const SceneSkyboxScript = preload("res://assets/environment/scene_skybox.gd")

var suite_name := "scene_skybox"
var method_name := "test_gradients_and_luts"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	_test_sample()
	_test_make_lut()
	_test_equirectangular_check()
	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return errors.is_empty()


func _test_sample() -> void:
	var keys := [[0.0, 1.0, 0.0, 0.0, 1.0], [0.5, 0.0, 1.0, 0.0, 0.5], [1.0, 0.0, 0.0, 1.0, 1.0]]

	var midnight: Color = SceneSkyboxScript._sample(keys, 0.0)
	_expect(midnight.is_equal_approx(Color(1.0, 0.0, 0.0, 1.0)), "first key at t=0")

	var midday: Color = SceneSkyboxScript._sample(keys, 0.5)
	_expect(midday.is_equal_approx(Color(0.0, 1.0, 0.0, 0.5)), "key at t=0.5")

	var quarter: Color = SceneSkyboxScript._sample(keys, 0.25)
	_expect(
		quarter.is_equal_approx(Color(0.5, 0.5, 0.0, 0.75)),
		"interpolates linearly between keys (%s)" % quarter
	)

	# Positions outside the key range take the nearest key's color.
	_expect(
		SceneSkyboxScript._sample(keys, -1.0).is_equal_approx(midnight),
		"before the first key takes its color"
	)
	_expect(
		SceneSkyboxScript._sample(keys, 2.0).is_equal_approx(Color(0.0, 0.0, 1.0, 1.0)),
		"after the last key takes its color"
	)

	# A single key is a flat gradient.
	var single := [[0.3, 0.25, 0.5, 0.75, 1.0]]
	_expect(
		SceneSkyboxScript._sample(single, 0.0).is_equal_approx(Color(0.25, 0.5, 0.75, 1.0)),
		"single key is flat"
	)
	_expect(
		SceneSkyboxScript._sample(single, 1.0).is_equal_approx(Color(0.25, 0.5, 0.75, 1.0)),
		"single key is flat at the end too"
	)


func _test_make_lut() -> void:
	var keys := [[0.0, 0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0, 1.0]]

	var linear_lut: GradientTexture1D = SceneSkyboxScript._make_lut(keys, false)
	_expect(linear_lut != null, "builds a LUT for two keys")
	_expect(linear_lut.use_hdr, "LUT keeps HDR values")
	var gradient: Gradient = linear_lut.gradient
	_expect(gradient.get_point_count() == 2, "LUT keeps both keys")
	_expect(
		gradient.sample(0.0).is_equal_approx(Color(0.0, 0.0, 0.0, 1.0)),
		"linear LUT stores the color as authored"
	)

	# The three tint LUTs are declared `source_color`, so their values are written
	# sRGB-encoded and the shader decodes them back to the same linear color.
	var srgb_lut: GradientTexture1D = SceneSkyboxScript._make_lut([[0.0, 0.5, 0.5, 0.5, 1.0]], true)
	var srgb_color: Color = srgb_lut.gradient.sample(0.5)
	_expect_near(srgb_color.r, 0.7354, "linear 0.5 is stored sRGB-encoded")
	_expect(
		srgb_color.srgb_to_linear().is_equal_approx(Color(0.5, 0.5, 0.5, 1.0)),
		"and the shader's sRGB decode lands back on the proto's color"
	)

	var single_lut: GradientTexture1D = SceneSkyboxScript._make_lut(
		[[0.4, 0.2, 0.4, 0.6, 1.0]], false
	)
	_expect(
		single_lut.gradient.sample(0.9).is_equal_approx(Color(0.2, 0.4, 0.6, 1.0)),
		"single-key LUT is flat everywhere"
	)


func _test_equirectangular_check() -> void:
	var skybox = SceneSkyboxScript.new(null)

	_expect(skybox._is_equirectangular(_texture_of(1024, 512)), "2:1 is equirectangular")
	_expect(not skybox._is_equirectangular(_texture_of(1024, 1024)), "1:1 is rejected")
	_expect(not skybox._is_equirectangular(_texture_of(1024, 341)), "3:1 is rejected")

	# An empty config must never leave the sky overridden.
	skybox.apply_config("", -1)
	_expect(not skybox._active, "empty config leaves the sky alone")
	skybox.apply_config("not json", 1)
	_expect(not skybox._active, "unparseable config leaves the sky alone")


func _texture_of(width: int, height: int) -> Texture2D:
	var image := Image.create_empty(width, height, false, Image.FORMAT_RGBA8)
	return ImageTexture.create_from_image(image)


func _expect_near(actual: float, expected: float, label: String) -> void:
	if absf(actual - expected) > 0.001:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])


func _expect(condition: bool, label: String) -> void:
	if not condition:
		errors.append(label)
