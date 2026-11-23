class_name SceneSkybox
extends RefCounted

## Applies the scene sky overrides (`PBSkybox` / `PBSun` / `PBEnvironmentFog`) that
## Rust resolves and publishes as JSON over `Global.sdk_skybox_changed`.
##
## Procedural sky colors, cloud color, sun color and the scalars ride on the existing
## sky shader uniforms (the same per-time-of-day LUTs the baked day/night cycle uses),
## so no shader change is needed. A `skybox_texture` swaps the sky material for a
## panorama; the rest of the time-of-day pipeline stays as is.

const LUT_WIDTH := 256
const EQUIRECTANGULAR_TOLERANCE := 0.02

# sky_colors key → shader LUT uniform it drives.
const SKY_COLOR_LUTS := {
	"zenith": "tint_color_cycle_bottom",
	"horizon": "tint_color_cycle_top",
	"nadir": "tint_color_cycle_nadir",
	"rim": "rim_color_cycle",
}

# These LUTs are declared `source_color`, so the shader decodes sRGB on sample:
# write sRGB-encoded values to land the proto's linear color in the sky.
const SRGB_ENCODED_LUTS := [
	"tint_color_cycle_bottom", "tint_color_cycle_top", "tint_color_cycle_nadir"
]

var _sky: Node
var _active := false
var _config := {}
var _scene_id := -1
var _texture_token := 0
var _pano_material: PanoramaSkyMaterial = null
var _saved_material: Material = null
var _params_material: Material = null
var _saved_params := {}
var _saved_fog := {}


func _init(sky: Node) -> void:
	_sky = sky


## Replaces every override with the ones in `config_json` (empty = back to defaults).
func apply_config(config_json: String, scene_id: int) -> void:
	restore()
	_scene_id = scene_id

	if config_json.is_empty():
		return

	# `JSON.parse` rather than `JSON.parse_string`: the latter logs an engine error for
	# a payload that is not JSON, which is a scene bug the log already covers.
	var json := JSON.new()
	if json.parse(config_json) != OK or not json.data is Dictionary:
		return

	_config = json.data
	if _config.is_empty():
		return

	_active = true
	_apply_static()


## Per-frame part of the overrides, called after the day/night cycle wrote the sky.
func apply_frame(time_of_day: float) -> void:
	if not _active or not is_instance_valid(_sky):
		return

	var sun: Dictionary = _config.get("sun", {})
	if not sun.is_empty():
		_apply_sun(sun, time_of_day)

	var sky_colors: Dictionary = _config.get("sky_colors", {})
	if sky_colors.has("horizon"):
		_apply_ambient(sky_colors["horizon"], time_of_day)

	var fog: Dictionary = _config.get("fog", {})
	if fog.has("color"):
		_apply_fog_color(fog["color"], time_of_day)


## Drops every override and hands the sky back to the day/night cycle.
func restore() -> void:
	_texture_token += 1
	if is_instance_valid(_sky):
		var material := _params_material if _params_material != null else _material()
		if material is ShaderMaterial:
			for param in _saved_params:
				material.set_shader_parameter(param, _saved_params[param])
		var environment := _environment()
		if environment != null and not _saved_fog.is_empty():
			environment.fog_mode = _saved_fog["mode"]
			environment.fog_density = _saved_fog["density"]
			environment.fog_depth_begin = _saved_fog["begin"]
			environment.fog_depth_end = _saved_fog["end"]
			environment.fog_depth_curve = _saved_fog["curve"]
		var sky_resource := _sky_resource()
		if _pano_material != null and sky_resource != null:
			if sky_resource.sky_material == _pano_material:
				sky_resource.sky_material = _saved_material
	_saved_params.clear()
	_saved_fog.clear()
	_params_material = null
	_pano_material = null
	_saved_material = null
	_active = false
	_config = {}
	_scene_id = -1


func _apply_static() -> void:
	var texture_src: String = _config.get("skybox_texture", "")
	if texture_src.is_empty():
		_apply_procedural_overrides()
	else:
		# Deferred: the config can arrive from inside the scene tick, where resolving the
		# content mapping would re-enter the scene runner that is already borrowed.
		call_deferred("_async_load_texture", texture_src)
	_apply_fog_density()


func _apply_procedural_overrides() -> void:
	var sky_colors: Dictionary = _config.get("sky_colors", {})
	for key in SKY_COLOR_LUTS:
		if sky_colors.has(key):
			_set_param(SKY_COLOR_LUTS[key], _make_lut(sky_colors[key], SRGB_ENCODED_LUTS.has(key)))

	var clouds: Dictionary = _config.get("clouds", {})
	if clouds.has("color"):
		_set_param("clouds_color_cycle", _make_lut(clouds["color"], false))
	if clouds.has("opacity"):
		_set_param("cloud_opacity", float(clouds["opacity"]))
	if clouds.has("speed"):
		# The shader advances clouds by turns per second (TIME * speed * TAU); the
		# proto's speed is radians per second.
		_set_param("clouds_scroll_speed", float(clouds["speed"]) / TAU)

	var stars: Dictionary = _config.get("stars", {})
	if stars.has("brightness"):
		_set_param("stars_energy", float(stars["brightness"]))

	var sun: Dictionary = _config.get("sun", {})
	if sun.has("color"):
		_set_param("sun_color_cycle", _make_lut(sun["color"], false))


func _async_load_texture(src: String) -> void:
	if not is_instance_valid(_sky):
		return

	var scene_runner: Node = Global.get_scene_runner()
	if scene_runner == null:
		return

	var content_mapping: RefCounted = scene_runner.get_scene_content_mapping(_scene_id)
	if content_mapping == null:
		push_warning("PBSkybox.skybox_texture: no content mapping for scene %d" % _scene_id)
		return

	_texture_token += 1
	var token := _texture_token

	var promise: Promise = Global.content_provider.fetch_texture(src, content_mapping)
	var result: Variant = await PromiseUtils.async_awaiter(promise)

	# Superseded by a newer config, or the sky was replaced while the texture loaded.
	if token != _texture_token or not is_instance_valid(_sky):
		return

	if result is PromiseError:
		push_warning(
			(
				"PBSkybox.skybox_texture could not be loaded (%s): %s; keeping the default sky"
				% [src, result.get_error()]
			)
		)
		_apply_procedural_overrides()
		return

	var texture: Texture2D = result.texture
	if texture == null:
		push_warning(
			"PBSkybox.skybox_texture loaded no texture (%s); keeping the default sky" % src
		)
		_apply_procedural_overrides()
		return

	if not _is_equirectangular(texture):
		# The proto asks for 2:1, but the Unity client stretches whatever arrives over the
		# dome, so rejecting it here leaves the two clients showing different skies.
		var size := texture.get_size()
		push_warning(
			(
				(
					"PBSkybox.skybox_texture is not equirectangular 2:1, got %dx%d (%s); "
					+ "rendering it stretched"
				)
				% [size.x, size.y, src]
			)
		)

	_apply_texture(texture)


func _apply_texture(texture: Texture2D) -> void:
	var sky_resource := _sky_resource()
	if sky_resource == null:
		return

	# Reflections come from the sky itself (`Environment.reflected_light_source` is
	# SKY), so a panorama sky also feeds the reflection map — no separate probe.
	if sky_resource.sky_material == null:
		push_warning("PBSkybox.skybox_texture: the sky has no material to replace")
		return

	_saved_material = sky_resource.sky_material
	_pano_material = PanoramaSkyMaterial.new()
	_pano_material.panorama = texture
	_pano_material.filter = true
	sky_resource.sky_material = _pano_material


func _apply_sun(sun: Dictionary, time_of_day: float) -> void:
	var main_light: DirectionalLight3D = _sky.main_light
	if main_light == null:
		return

	if sun.has("color"):
		var color := _sample(sun["color"], time_of_day)
		main_light.light_color = color
		RenderingServer.global_shader_parameter_set("sun_color", Vector3(color.r, color.g, color.b))

	if sun.has("intensity"):
		main_light.light_energy *= float(sun["intensity"])

	if sun.has("direction"):
		var direction_value: Array = sun["direction"]
		var direction := Vector3(direction_value[0], direction_value[1], direction_value[2])
		# `looking_at` puts -Z along the light's travel direction, so the sky draws the
		# disc at basis.z — opposite the light, which is where the sun is.
		var basis := Basis.looking_at(direction, Vector3.UP)
		main_light.global_transform.basis = basis
		var moon_light: DirectionalLight3D = _sky.moon_light
		if moon_light != null:
			moon_light.global_transform.basis = basis
		RenderingServer.global_shader_parameter_set("sun_direction", basis.z)

	if sun.has("visible") and not bool(sun["visible"]):
		# Hides the discs and the flare; the light itself stays on (proto contract).
		RenderingServer.global_shader_parameter_set("sun_opacity", 0.0)


func _apply_ambient(keys: Array, time_of_day: float) -> void:
	var environment := _environment()
	if environment == null:
		return

	# The proto says the sky colors drive the ambient light without saying which band,
	# so the horizon is used: it is the broad band the eye reads as "the sky".
	var color := _sample(keys, time_of_day)
	environment.ambient_light_color = color
	RenderingServer.global_shader_parameter_set("ambient_color", Vector3(color.r, color.g, color.b))


func _apply_fog_color(keys: Array, time_of_day: float) -> void:
	var environment := _environment()
	if environment == null:
		return
	environment.fog_light_color = _sample(keys, time_of_day)


func _apply_fog_density() -> void:
	var fog: Dictionary = _config.get("fog", {})
	if not fog.has("density"):
		return
	var environment := _environment()
	if environment == null:
		return

	# The player's view-distance setting owns depth fog (environment_selector.gd), so save
	# that state once: a scene density must not survive into the default sky after Reset.
	if _saved_fog.is_empty():
		_saved_fog = {
			"mode": environment.fog_mode,
			"density": environment.fog_density,
			"begin": environment.fog_depth_begin,
			"end": environment.fog_depth_end,
			"curve": environment.fog_depth_curve,
		}

	# The proto's density is per meter: fog = 1 - e^(-density * distance).
	environment.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	environment.fog_density = maxf(float(fog["density"]), 0.0)


func _material() -> Material:
	var sky_resource := _sky_resource()
	if sky_resource == null:
		return null
	return sky_resource.sky_material


func _sky_resource() -> Sky:
	var environment := _environment()
	if environment == null:
		return null
	return environment.sky


func _environment() -> Environment:
	if not is_instance_valid(_sky):
		return null
	var world_environment: WorldEnvironment = _sky.world_environment
	if world_environment == null:
		return null
	return world_environment.environment


func _set_param(param: String, value: Variant) -> void:
	var material := _material()
	if not material is ShaderMaterial:
		return
	if not _saved_params.has(param):
		_saved_params[param] = material.get_shader_parameter(param)
	_params_material = material
	material.set_shader_parameter(param, value)


func _is_equirectangular(texture: Texture2D) -> bool:
	var size := texture.get_size()
	if size.y <= 0:
		return false
	return absf(size.x / size.y - 2.0) <= EQUIRECTANGULAR_TOLERANCE


static func _make_lut(keys: Array, srgb_encode: bool) -> GradientTexture1D:
	var offsets := PackedFloat32Array()
	var colors := PackedColorArray()
	for key in keys:
		offsets.append(float(key[0]))
		var color := _key_color(key)
		colors.append(color.linear_to_srgb() if srgb_encode else color)

	var gradient := Gradient.new()
	gradient.offsets = offsets
	gradient.colors = colors

	var lut := GradientTexture1D.new()
	lut.gradient = gradient
	lut.width = LUT_WIDTH
	lut.use_hdr = true
	return lut


## Piecewise-linear sample of a resolved gradient at a normalized time of day, matching
## the proto: positions before the first key or after the last take that key's color.
static func _sample(keys: Array, time_of_day: float) -> Color:
	if keys.is_empty():
		return Color.WHITE
	if time_of_day <= float(keys[0][0]):
		return _key_color(keys[0])
	if time_of_day >= float(keys[-1][0]):
		return _key_color(keys[-1])

	for index in range(1, keys.size()):
		var upper: Array = keys[index]
		var upper_time := float(upper[0])
		if time_of_day <= upper_time:
			var lower: Array = keys[index - 1]
			var span := upper_time - float(lower[0])
			if span <= 0.0:
				return _key_color(upper)
			return _key_color(lower).lerp(_key_color(upper), (time_of_day - float(lower[0])) / span)

	return _key_color(keys[-1])


static func _key_color(key: Array) -> Color:
	return Color(key[1], key[2], key[3], key[4])
