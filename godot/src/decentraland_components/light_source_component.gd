class_name DclLightSourceComponent
extends Node3D

const DEFAULT_PROJECTOR_TEXTURE_PATH: String = "res://assets/white_pixel.png"

# ponytail: insertion-ordered dict evicted past the cap; no LRU bookkeeping.
const PROJECTOR_CACHE_MAX_ENTRIES: int = 16

const DEBUG_DCL_LIGHTS_LOG: bool = false

const DCL_LIGHT_AUTO_RANGE_DIVISOR: float = 160.0

# SDK intensity is candela (proto default 16000). The reference client passes
# it through to Unity's physical-unit intensity, so candela -> Godot energy
# divides by the proto default: 16000 -> 1.0. Validated against mainnet
# scenes (rituals, doriangray, RAGE). Note: the sdk7 dynamic-lights test
# scene authors ~100, which renders nearly invisible — its values are 100x
# below real-world authoring, not a conversion bug.
const DCL_LIGHT_INTENSITY_DIVISOR: float = 16000.0

# Godot needs shadow_enabled=true for light_projector to work, so "shadows off"
# removes dynamic casters via shadow_caster_mask instead of disabling the shadow.
const DCL_LIGHT_SHADOW_CASTER_MASK_ALL: int = 0xFFFFFFFF
const DCL_LIGHT_SHADOW_CASTER_MASK_NONE: int = 0

# Readability cap for gizmo cone radius: wide authored angles (e.g. 179°)
# would otherwise produce kilometer-long lines. Length is never capped.
const DEBUG_SPOT_CONE_MAX_RADIUS: float = 30.0
const DEBUG_SPOT_CONE_MIN_LENGTH: float = 0.18
const DEBUG_POINT_SPHERE_RADIUS: float = 1.0
const DEBUG_SPOT_CONE_SEGMENTS: int = 32
const DEBUG_POINT_SPHERE_SEGMENTS: int = 32

const DEBUG_TEXTURE_PATH_MAX_LEN: int = 34
const DEBUG_INFO_LABEL_PIXEL_SIZE: float = 0.000625
const DEBUG_INFO_LABEL_OUTLINE_SIZE: int = 2
const DEBUG_INFO_LABEL_INTENSITY_POS: Vector3 = Vector3(0.0, 0.62, 0.0)
const DEBUG_INFO_LABEL_RANGE_POS: Vector3 = Vector3(-0.95, 0.15, 0.0)
const DEBUG_INFO_LABEL_TEXTURE_POS: Vector3 = Vector3(1.05, 0.15, 0.0)
const DEBUG_INFO_LABEL_BUDGET_POS: Vector3 = Vector3(0.0, -0.42, 0.0)
const DEBUG_STATUS_LABEL_PIXEL_SIZE: float = 0.0011
const DEBUG_STATUS_LABEL_POS: Vector3 = Vector3.ZERO

# Light energy fades in/out over this many seconds when render state changes
# (matches the reference client's FadeDuration).
const FADE_DURATION: float = 0.25

# Camera-direction culling: lights behind the camera don't render, unless
# their influence sphere still reaches the camera (you're standing inside
# their glow) or they're very close (wrap into view when the camera turns).
const CAMERA_CULL_CLOSE_RADIUS: float = 6.0
# dot(camera_forward, to_light) threshold; 0.2 ~= 78 deg half-angle (FOV + margin).
const CAMERA_CULL_MIN_DOT: float = 0.2

var current_light: Light3D = null
var projector_texture: Texture2D = null
var projector_texture_path: String = ""
var projector_texture_display_path: String = ""
var pending_http_request: HTTPRequest = null

var last_kind: String = ""
var last_intensity: float = 0.0
var last_light_range: float = 0.0
var last_inner_angle: float = 0.0
var last_outer_angle: float = 0.0

# External runtime limits. Rust can disable a light without fighting range logic.
var runtime_light_enabled: bool = true
var runtime_shadows_enabled: bool = true

var debug_gizmo_root: Node3D = null
var debug_status_label: Label3D = null
var debug_spot_outer_cone_mesh: MeshInstance3D = null
var debug_spot_inner_cone_mesh: MeshInstance3D = null
var debug_point_sphere_mesh: MeshInstance3D = null
var debug_info_lines_mesh: MeshInstance3D = null
var debug_info_labels: Array[Label3D] = []

var budget_light_enabled: bool = false
var debug_budget_rank: int = -1
var debug_budget_candidate: bool = false
var debug_last_distance_to_avatar: float = -1.0

# Fade state: render state changes ramp light_energy 0 <-> _base_energy over
# FADE_DURATION instead of hard-toggling visible. Advanced by the manager.
var _base_energy: float = 0.0
var _energy_scale: float = 0.0
var _render_target: bool = false
var _fade_active: bool = false

# Runtime-editable light settings (static so the settings menu can modify them).
# Defaults are ON so desktop/Custom-profile users don't have to enable them
# every run; apply_graphic_profile overwrites them per profile (mobile
# profiles clamp them down). See PROFILE_DEFINITIONS.
static var dcl_lights_system_enabled: bool = true
static var dcl_lights_rendering_enabled: bool = true
static var force_dcl_light_shadows_off: bool = false

static var debug_dcl_lights_gizmo: bool = false

static var auto_activation_range: bool = true
static var auto_activation_range_cap: float = 40.0
static var use_global_light_budget: bool = true
static var max_active_dcl_lights: int = 4

static var projector_texture_cache: Dictionary = {}

static var registered_lights: Array[DclLightSourceComponent] = []
static var reference_position: Vector3 = Vector3.ZERO
static var has_reference_position: bool = false
static var camera_position: Vector3 = Vector3.ZERO
static var camera_forward: Vector3 = Vector3.FORWARD
static var has_camera: bool = false

# Set whenever range/budget state must be recomputed (new light, removal,
# settings change, transform change). Consumed by LightSourceManager's tick.
static var _recompute_pending: bool = true

# --- Static API (called by LightSourceManager / settings UI) ---


static func apply_light_settings(
	lights_enabled: bool,
	shadows_enabled: bool,
	max_lights: int,
	debug_enabled: bool,
	auto_range_enabled: bool,
	budget_enabled: bool
) -> void:
	dcl_lights_system_enabled = lights_enabled
	dcl_lights_rendering_enabled = lights_enabled
	force_dcl_light_shadows_off = not shadows_enabled

	max_active_dcl_lights = max(0, max_lights)
	debug_dcl_lights_gizmo = debug_enabled
	auto_activation_range = auto_range_enabled
	use_global_light_budget = budget_enabled

	for light in registered_lights:
		if not is_instance_valid(light):
			continue

		if debug_enabled:
			light._ensure_debug_gizmo()
			if light.last_kind != "":
				light._update_debug_gizmo(
					light.last_kind,
					light.last_intensity,
					light.last_light_range,
					light.last_inner_angle,
					light.last_outer_angle
				)

		if light.debug_gizmo_root != null:
			light.debug_gizmo_root.visible = debug_enabled

	request_recompute()


static func apply_graphic_profile_settings(
	lights_enabled: bool, shadows_enabled: bool, max_lights: int, range_cap: float
) -> void:
	dcl_lights_system_enabled = lights_enabled
	dcl_lights_rendering_enabled = lights_enabled
	force_dcl_light_shadows_off = not shadows_enabled
	max_active_dcl_lights = max(0, max_lights)
	auto_activation_range_cap = range_cap
	request_recompute()


static func get_light_settings() -> Dictionary:
	return {
		"lights_enabled": dcl_lights_system_enabled and dcl_lights_rendering_enabled,
		"shadows_enabled": not force_dcl_light_shadows_off,
		"max_lights": max_active_dcl_lights,
		"debug_enabled": debug_dcl_lights_gizmo,
		"auto_activation_range": auto_activation_range,
		"use_global_light_budget": use_global_light_budget,
	}


## Called by LightSourceManager when the reference position changed or a
## recompute was requested (new light, removal, settings change). This is the
## ONLY place where range/budget/visibility state is recomputed — lights
## themselves never run per-frame logic.
static func tick(
	new_reference_position: Vector3,
	has_reference: bool,
	cam_pos: Vector3,
	cam_forward: Vector3,
	cam_available: bool
) -> void:
	reference_position = new_reference_position
	has_reference_position = has_reference
	camera_position = cam_pos
	camera_forward = cam_forward
	has_camera = cam_available
	_update_global_light_budget()


## Called EVERY frame by the manager: advances fades only for lights in
## transition (cheap early-out per light, no per-light _process nodes).
static func tick_fades(delta: float) -> void:
	for light in registered_lights:
		if is_instance_valid(light) and light._fade_active:
			light._tick_fade(delta)


static func clear_reference_position() -> void:
	has_reference_position = false
	for light in registered_lights:
		if not is_instance_valid(light):
			continue
		light.budget_light_enabled = false
		light.debug_budget_rank = -1
		light.debug_budget_candidate = false
		light.debug_last_distance_to_avatar = -1.0
		light._update_light_rendering_by_range()


static func request_recompute() -> void:
	_recompute_pending = true


static func is_recompute_pending() -> bool:
	return _recompute_pending


static func consume_recompute_pending() -> bool:
	var pending: bool = _recompute_pending
	_recompute_pending = false
	return pending


static func _sort_by_distance(a: Dictionary, b: Dictionary) -> bool:
	return float(a["distance"]) < float(b["distance"])


static func _update_global_light_budget() -> void:
	if not use_global_light_budget:
		for light in registered_lights:
			if not is_instance_valid(light):
				continue
			light.budget_light_enabled = true
			light.debug_budget_rank = -1
			light.debug_budget_candidate = false
			light.debug_last_distance_to_avatar = -1.0
			light._update_light_rendering_by_range()
		return

	var candidates: Array = []

	for light in registered_lights:
		if not is_instance_valid(light):
			continue

		light.budget_light_enabled = false
		light.debug_budget_rank = -1
		light.debug_budget_candidate = false
		light.debug_last_distance_to_avatar = -1.0

		if (
			light.current_light == null
			or light.last_kind == ""
			or not light.runtime_light_enabled
			or light._get_avatar_range_state(light.last_light_range) != 1
			or not light._is_in_camera_view()
		):
			light._update_light_rendering_by_range()
			continue

		var distance_to_ref: float = light.global_position.distance_to(reference_position)
		light.debug_last_distance_to_avatar = distance_to_ref
		light.debug_budget_candidate = true
		candidates.append({"light": light, "distance": distance_to_ref})

	candidates.sort_custom(_sort_by_distance)

	for i: int in range(candidates.size()):
		var light: DclLightSourceComponent = candidates[i]["light"]
		if not is_instance_valid(light):
			continue
		light.debug_budget_rank = i + 1
		light.budget_light_enabled = i < max_active_dcl_lights
		light._update_light_rendering_by_range()

	if debug_dcl_lights_gizmo:
		for light in registered_lights:
			if is_instance_valid(light) and light.debug_gizmo_root != null:
				light._refresh_debug_gizmo_state()


# --- Instance lifecycle ---


func _ready() -> void:
	if not registered_lights.has(self):
		registered_lights.append(self)
	if projector_texture_path == "":
		set_projector_texture(DEFAULT_PROJECTOR_TEXTURE_PATH)
	if debug_dcl_lights_gizmo:
		_ensure_debug_gizmo()
	request_recompute()


func _exit_tree() -> void:
	var idx: int = registered_lights.find(self)
	if idx != -1:
		registered_lights.remove_at(idx)
	request_recompute()


# --- Light configuration (called from Rust) ---


func set_spot(
	color: Color, intensity: float, light_range: float, inner_angle: float, outer_angle: float
) -> void:
	if current_light == null or not (current_light is SpotLight3D):
		_clear_light()
		current_light = SpotLight3D.new()
		current_light.name = "DCL_SpotLight"
		current_light.rotation_degrees = Vector3.ZERO
		add_child(current_light)

	var spot: SpotLight3D = current_light as SpotLight3D
	spot.light_color = color
	_base_energy = _convert_intensity(intensity)
	_apply_fade()
	spot.spot_range = light_range
	# DCL/Unity angles describe the FULL cone; Godot's spot_angle is the
	# angular RADIUS (center to edge, hard max 89.99). Halve to convert.
	var godot_outer: float = minf(outer_angle * 0.5, 89.99)
	spot.spot_angle = godot_outer
	# Godot has no inner-angle plateau; approximate the inner->outer falloff
	# with the attenuation exponent (higher = brightness holds longer toward
	# the edge, like a wide inner cone).
	var inner_radius: float = minf(inner_angle * 0.5, godot_outer)
	spot.spot_angle_attenuation = godot_outer / maxf(godot_outer - inner_radius, 0.01)

	last_kind = "spot"
	last_intensity = intensity
	last_light_range = light_range
	last_inner_angle = inner_angle
	last_outer_angle = outer_angle

	if projector_texture == null and projector_texture_path != "":
		set_projector_texture(projector_texture_path)

	_apply_projector_texture()

	if debug_dcl_lights_gizmo:
		_update_debug_gizmo("spot", intensity, light_range, inner_angle, outer_angle)

	request_recompute()


func set_point(color: Color, intensity: float, light_range: float) -> void:
	if current_light == null or not (current_light is OmniLight3D):
		_clear_light()
		current_light = OmniLight3D.new()
		current_light.name = "DCL_OmniLight"
		current_light.rotation_degrees = Vector3.ZERO
		add_child(current_light)

	var omni: OmniLight3D = current_light as OmniLight3D
	omni.light_color = color
	_base_energy = _convert_intensity(intensity)
	_apply_fade()
	omni.omni_range = light_range

	last_kind = "point"
	last_intensity = intensity
	last_light_range = light_range
	last_inner_angle = 0.0
	last_outer_angle = 0.0

	if debug_dcl_lights_gizmo:
		_update_debug_gizmo("point", intensity, light_range, 0.0, 0.0)

	request_recompute()


func remove_light() -> void:
	_clear_light()
	if debug_gizmo_root != null:
		debug_gizmo_root.visible = false


func set_shadow_enabled(enabled: bool) -> void:
	runtime_shadows_enabled = enabled
	_update_light_rendering_by_range()


func set_light_enabled(enabled: bool) -> void:
	runtime_light_enabled = enabled
	visible = enabled and dcl_lights_system_enabled
	request_recompute()
	if debug_gizmo_root != null:
		debug_gizmo_root.visible = debug_dcl_lights_gizmo


# --- Render state ---


func _update_light_rendering_by_range() -> void:
	if current_light == null:
		return

	if (
		not dcl_lights_system_enabled
		or not dcl_lights_rendering_enabled
		or not runtime_light_enabled
	):
		_set_light_render_state(false)
		return

	var range_state: int = _get_avatar_range_state(last_light_range)
	if range_state == -1:
		_set_light_render_state(false)
		return

	var should_render: bool = range_state == 1 and _is_in_camera_view()
	if use_global_light_budget:
		should_render = should_render and budget_light_enabled

	_set_light_render_state(should_render)


func _set_light_render_state(should_render: bool) -> void:
	if current_light == null:
		return

	if should_render == _render_target and not _fade_active:
		return

	_render_target = should_render

	# Shadows switch instantly (a shadow popping after the light faded would
	# look worse than no fade on the shadow itself).
	if not should_render:
		current_light.shadow_enabled = false
		current_light.shadow_caster_mask = DCL_LIGHT_SHADOW_CASTER_MASK_NONE
	else:
		var has_projector: bool = (
			current_light is SpotLight3D
			and projector_texture_path != ""
			and projector_texture_path != DEFAULT_PROJECTOR_TEXTURE_PATH
		)

		# Godot requires shadow_enabled=true for light_projector to work, so
		# projector lights keep it on even when dynamic shadows are disabled.
		current_light.shadow_enabled = runtime_shadows_enabled or has_projector

		if force_dcl_light_shadows_off:
			current_light.shadow_caster_mask = DCL_LIGHT_SHADOW_CASTER_MASK_NONE
		else:
			current_light.shadow_caster_mask = DCL_LIGHT_SHADOW_CASTER_MASK_ALL

	_fade_active = true


func _tick_fade(delta: float) -> void:
	if current_light == null:
		_fade_active = false
		return

	var step: float = delta / FADE_DURATION
	if _render_target:
		_energy_scale = minf(_energy_scale + step, 1.0)
		if _energy_scale >= 1.0:
			_fade_active = false
	else:
		_energy_scale = maxf(_energy_scale - step, 0.0)
		if _energy_scale <= 0.0:
			_fade_active = false

	_apply_fade()


func _apply_fade() -> void:
	if current_light == null:
		return
	current_light.visible = _energy_scale > 0.0
	current_light.light_energy = _base_energy * _energy_scale


func _is_in_camera_view() -> bool:
	if not has_camera:
		return true

	var to_light: Vector3 = global_position - camera_position
	var dist: float = to_light.length()

	# Keep lights whose range still reaches the camera: turning those off
	# would visibly kill lit surfaces on screen.
	var reach: float = maxf(CAMERA_CULL_CLOSE_RADIUS, _get_activation_range(last_light_range))
	if dist <= reach or dist <= 0.0:
		return true

	return camera_forward.dot(to_light / dist) >= CAMERA_CULL_MIN_DOT


func _clear_light() -> void:
	if current_light != null:
		current_light.queue_free()
		current_light = null


func _convert_intensity(intensity: float) -> float:
	return intensity / DCL_LIGHT_INTENSITY_DIVISOR


# ponytail: keep the /100 observation here in case a scene authored against
# the broken test-scene convention needs a compat knob later.


func _get_avatar_range_state(light_range: float) -> int:
	if not has_reference_position:
		return -1

	var activation_range: float = _get_activation_range(light_range)
	if activation_range <= 0.0:
		return -1

	var distance_to_ref: float = global_position.distance_to(reference_position)
	if distance_to_ref <= activation_range:
		return 1

	return 0


func _get_activation_range(light_range: float) -> float:
	if not auto_activation_range:
		return light_range

	var intensity_range: float = sqrt(maxf(last_intensity, 0.0) / DCL_LIGHT_AUTO_RANGE_DIVISOR)
	return minf(maxf(light_range, intensity_range), auto_activation_range_cap)


# --- Projector texture ---


func get_projector_texture_path() -> String:
	return projector_texture_path


func set_projector_texture(path: String, display_path: String = "") -> void:
	if path == DEFAULT_PROJECTOR_TEXTURE_PATH:
		projector_texture_display_path = ""
	elif display_path != "":
		projector_texture_display_path = display_path
	elif projector_texture_display_path == "" and _looks_like_authored_texture_path(path):
		projector_texture_display_path = path

	if projector_texture_path == path:
		if projector_texture != null:
			return
		if pending_http_request != null:
			return

	projector_texture_path = path
	projector_texture = null

	if path == "":
		if current_light is SpotLight3D:
			(current_light as SpotLight3D).light_projector = null
		return

	if path.begins_with("https://"):
		_load_projector_texture_http(path)
		return

	if not ResourceLoader.exists(path):
		return

	var res: Resource = load(path)
	projector_texture = res as Texture2D
	if projector_texture == null:
		return

	_apply_projector_texture()


func set_projector_texture_display_path(display_path: String) -> void:
	projector_texture_display_path = display_path


func _looks_like_authored_texture_path(path: String) -> bool:
	if path == "" or path == DEFAULT_PROJECTOR_TEXTURE_PATH:
		return false
	if path.begins_with("https://") or path.begins_with("ipfs://"):
		return false
	return path.begins_with("/") or path.begins_with("res://") or path.begins_with("user://")


static func _cache_projector_texture(url: String, texture: Texture2D) -> void:
	projector_texture_cache[url] = texture
	while projector_texture_cache.size() > PROJECTOR_CACHE_MAX_ENTRIES:
		# Godot dictionaries iterate in insertion order; first key is oldest.
		projector_texture_cache.erase(projector_texture_cache.keys()[0])


func _load_projector_texture_http(url: String) -> void:
	if projector_texture_cache.has(url):
		projector_texture = projector_texture_cache[url] as Texture2D
		_apply_projector_texture()
		return

	if pending_http_request != null:
		pending_http_request.queue_free()
		pending_http_request = null

	var request: HTTPRequest = HTTPRequest.new()
	pending_http_request = request
	add_child(request)

	request.request_completed.connect(
		func(
			_result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray
		) -> void:
			if pending_http_request == request:
				pending_http_request = null
			request.queue_free()

			if projector_texture_path != url:
				return

			if projector_texture_cache.has(url):
				projector_texture = projector_texture_cache[url] as Texture2D
				_apply_projector_texture()
				return

			if _result != HTTPRequest.RESULT_SUCCESS or response_code < 200 or response_code >= 300:
				return

			var image: Image = Image.new()
			var err: Error = image.load_png_from_buffer(body)
			if err != OK:
				err = image.load_jpg_from_buffer(body)
			if err != OK:
				err = image.load_webp_from_buffer(body)
			if err != OK:
				return

			# Only shrink oversized masks; never upscale. Keeps mobile shadow
			# map cost bounded without touching small authored textures.
			const PROJECTOR_MAX_SIZE: int = 512
			if image.get_width() > PROJECTOR_MAX_SIZE or image.get_height() > PROJECTOR_MAX_SIZE:
				image.resize(
					mini(image.get_width(), PROJECTOR_MAX_SIZE),
					mini(image.get_height(), PROJECTOR_MAX_SIZE),
					Image.INTERPOLATE_LANCZOS
				)

			var loaded_texture: Texture2D = ImageTexture.create_from_image(image)
			_cache_projector_texture(url, loaded_texture)
			projector_texture = loaded_texture
			_apply_projector_texture()
	)

	var err: Error = request.request(url)
	if err != OK:
		request.queue_free()
		if pending_http_request == request:
			pending_http_request = null


func _apply_projector_texture() -> void:
	if current_light is SpotLight3D and projector_texture != null:
		var spot: SpotLight3D = current_light as SpotLight3D
		spot.light_projector = projector_texture


# --- Debug gizmos ---


func _refresh_debug_gizmo_state() -> void:
	if debug_gizmo_root == null:
		return
	debug_gizmo_root.visible = debug_dcl_lights_gizmo
	if not debug_dcl_lights_gizmo:
		return
	_update_debug_origin_marker()
	_update_debug_info_labels(
		last_kind, last_intensity, last_light_range, last_inner_angle, last_outer_angle
	)


func _ensure_debug_gizmo() -> void:
	if debug_gizmo_root != null:
		return

	debug_gizmo_root = Node3D.new()
	debug_gizmo_root.name = "DCL_Light_DebugGizmo"
	add_child(debug_gizmo_root)

	debug_status_label = _make_center_status_label("StatusLabel")
	debug_gizmo_root.add_child(debug_status_label)

	debug_spot_outer_cone_mesh = MeshInstance3D.new()
	debug_spot_outer_cone_mesh.name = "SpotOuterCone"
	debug_gizmo_root.add_child(debug_spot_outer_cone_mesh)

	debug_spot_inner_cone_mesh = MeshInstance3D.new()
	debug_spot_inner_cone_mesh.name = "SpotInnerCone"
	debug_gizmo_root.add_child(debug_spot_inner_cone_mesh)

	debug_point_sphere_mesh = MeshInstance3D.new()
	debug_point_sphere_mesh.name = "PointSphere"
	debug_gizmo_root.add_child(debug_point_sphere_mesh)

	debug_info_lines_mesh = MeshInstance3D.new()
	debug_info_lines_mesh.name = "InfoLines"
	debug_gizmo_root.add_child(debug_info_lines_mesh)

	debug_info_labels.clear()
	for i in range(6):
		var label := _make_debug_info_label("DebugInfoLabel" + str(i))
		debug_info_labels.append(label)
		debug_gizmo_root.add_child(label)

	debug_gizmo_root.visible = debug_dcl_lights_gizmo


func _make_debug_info_label(label_name: String) -> Label3D:
	var label := Label3D.new()
	label.name = label_name
	label.modulate = Color(1.0, 1.0, 1.0, 1.0)
	label.fixed_size = true
	label.pixel_size = DEBUG_INFO_LABEL_PIXEL_SIZE
	label.outline_size = DEBUG_INFO_LABEL_OUTLINE_SIZE
	label.no_depth_test = true
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.text = ""
	return label


func _make_center_status_label(label_name: String) -> Label3D:
	var label := Label3D.new()
	label.name = label_name
	label.position = DEBUG_STATUS_LABEL_POS
	label.fixed_size = true
	label.pixel_size = DEBUG_STATUS_LABEL_PIXEL_SIZE
	label.no_depth_test = true
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.text = "🟢"
	return label


func _make_debug_material(kind: String, alpha: float = 1.0) -> StandardMaterial3D:
	var mat: StandardMaterial3D = StandardMaterial3D.new()

	var color: Color
	if kind == "spot_outer":
		color = Color(1.0, 0.55, 0.10, alpha)
	elif kind == "spot_inner":
		color = Color(1.0, 1.0, 0.15, alpha)
	elif kind == "inside_range_no_budget":
		color = Color(1.0, 0.9, 0.05, alpha)
	elif kind == "info_line":
		color = Color(1.0, 1.0, 1.0, alpha)
	else:
		color = Color(0.15, 0.85, 1.0, alpha)

	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = color
	mat.emission_enabled = true
	mat.emission = color
	mat.no_depth_test = true

	if alpha < 1.0:
		mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	else:
		mat.transparency = BaseMaterial3D.TRANSPARENCY_DISABLED

	return mat


func _update_debug_gizmo(
	kind: String, intensity: float, light_range: float, inner_angle: float, outer_angle: float
) -> void:
	_ensure_debug_gizmo()
	if debug_gizmo_root == null:
		return

	debug_gizmo_root.visible = true
	debug_gizmo_root.position = Vector3.ZERO
	debug_gizmo_root.rotation_degrees = Vector3.ZERO
	debug_gizmo_root.scale = Vector3.ONE

	_update_debug_origin_marker()

	if kind == "spot":
		if debug_spot_outer_cone_mesh != null:
			debug_spot_outer_cone_mesh.visible = true
		if debug_spot_inner_cone_mesh != null:
			debug_spot_inner_cone_mesh.visible = true
		if debug_point_sphere_mesh != null:
			debug_point_sphere_mesh.visible = false
		_update_debug_spot_cones(light_range, inner_angle, outer_angle)
	else:
		if debug_spot_outer_cone_mesh != null:
			debug_spot_outer_cone_mesh.visible = false
		if debug_spot_inner_cone_mesh != null:
			debug_spot_inner_cone_mesh.visible = false
		if debug_point_sphere_mesh != null:
			debug_point_sphere_mesh.visible = true
		_update_debug_point_sphere(light_range)

	_update_debug_info_labels(kind, intensity, light_range, inner_angle, outer_angle)


func _update_debug_origin_marker() -> void:
	if debug_status_label == null:
		return

	var range_state: int = _get_avatar_range_state(last_light_range)

	if range_state == 1:
		if use_global_light_budget and not budget_light_enabled:
			debug_status_label.text = "🟡"
		else:
			debug_status_label.text = "🟢"
	else:
		debug_status_label.text = "🔴"

	debug_status_label.position = DEBUG_STATUS_LABEL_POS


func _update_debug_spot_cones(light_range: float, inner_angle: float, outer_angle: float) -> void:
	_update_debug_spot_cone_mesh(
		debug_spot_outer_cone_mesh, light_range, outer_angle, "spot_outer", true
	)
	_update_debug_spot_cone_mesh(
		debug_spot_inner_cone_mesh, light_range, inner_angle, "spot_inner", false
	)


func _update_debug_spot_cone_mesh(
	target_mesh_instance: MeshInstance3D,
	light_range: float,
	angle_degrees: float,
	material_kind: String,
	draw_rays: bool
) -> void:
	if target_mesh_instance == null:
		return

	var dimensions: Dictionary = _get_debug_spot_cone_dimensions(light_range, angle_degrees)
	var cone_length: float = float(dimensions["length"])
	var cone_radius: float = float(dimensions["radius"])

	var mesh: ImmediateMesh = ImmediateMesh.new()
	var mat: StandardMaterial3D = _make_debug_material(material_kind, 0.95)

	mesh.surface_begin(Mesh.PRIMITIVE_LINES, mat)

	for i: int in range(DEBUG_SPOT_CONE_SEGMENTS):
		var a0: float = TAU * float(i) / float(DEBUG_SPOT_CONE_SEGMENTS)
		var a1: float = TAU * float(i + 1) / float(DEBUG_SPOT_CONE_SEGMENTS)

		var p0: Vector3 = Vector3(cos(a0) * cone_radius, sin(a0) * cone_radius, -cone_length)
		var p1: Vector3 = Vector3(cos(a1) * cone_radius, sin(a1) * cone_radius, -cone_length)

		mesh.surface_add_vertex(p0)
		mesh.surface_add_vertex(p1)

		if draw_rays:
			mesh.surface_add_vertex(Vector3.ZERO)
			mesh.surface_add_vertex(p0)

	mesh.surface_end()

	target_mesh_instance.mesh = mesh
	target_mesh_instance.position = Vector3.ZERO
	target_mesh_instance.rotation_degrees = Vector3.ZERO
	target_mesh_instance.scale = Vector3.ONE


func _update_debug_point_sphere(light_range: float) -> void:
	if debug_point_sphere_mesh == null:
		return

	var mesh: ImmediateMesh = ImmediateMesh.new()
	var mat: StandardMaterial3D = _make_debug_material("point", 0.95)
	var radius: float = maxf(light_range, DEBUG_POINT_SPHERE_RADIUS)

	mesh.surface_begin(Mesh.PRIMITIVE_LINES, mat)

	for i: int in range(DEBUG_POINT_SPHERE_SEGMENTS):
		var a0: float = TAU * float(i) / float(DEBUG_POINT_SPHERE_SEGMENTS)
		var a1: float = TAU * float(i + 1) / float(DEBUG_POINT_SPHERE_SEGMENTS)

		var x0: float = cos(a0) * radius
		var y0: float = sin(a0) * radius
		var x1: float = cos(a1) * radius
		var y1: float = sin(a1) * radius

		mesh.surface_add_vertex(Vector3(x0, y0, 0.0))
		mesh.surface_add_vertex(Vector3(x1, y1, 0.0))
		mesh.surface_add_vertex(Vector3(x0, 0.0, y0))
		mesh.surface_add_vertex(Vector3(x1, 0.0, y1))
		mesh.surface_add_vertex(Vector3(0.0, x0, y0))
		mesh.surface_add_vertex(Vector3(0.0, x1, y1))

	mesh.surface_end()

	debug_point_sphere_mesh.mesh = mesh
	debug_point_sphere_mesh.position = Vector3.ZERO
	debug_point_sphere_mesh.rotation_degrees = Vector3.ZERO
	debug_point_sphere_mesh.scale = Vector3.ONE


func _update_debug_info_labels(
	kind: String, intensity: float, light_range: float, inner_angle: float, outer_angle: float
) -> void:
	if debug_info_labels.size() < 6:
		return

	var tex_text: String = "tex:n"
	if projector_texture_path != "" and projector_texture_path != DEFAULT_PROJECTOR_TEXTURE_PATH:
		var debug_texture_path: String = projector_texture_display_path
		if debug_texture_path == "":
			debug_texture_path = projector_texture_path
		tex_text = "tex:" + _short_path(debug_texture_path)

	var budget_text: String
	if use_global_light_budget and debug_budget_candidate:
		budget_text = "rank:" + str(debug_budget_rank)
		if debug_last_distance_to_avatar >= 0.0:
			budget_text += " d:" + _fmtf(debug_last_distance_to_avatar)
	else:
		budget_text = "budget:-"

	_set_debug_info_label(0, DEBUG_INFO_LABEL_INTENSITY_POS, "i:" + _fmtf(intensity))
	_set_debug_info_label(
		1,
		DEBUG_INFO_LABEL_RANGE_POS,
		"r:" + _fmtf(light_range) + " ar:" + _fmtf(_get_activation_range(light_range))
	)
	_set_debug_info_label(2, DEBUG_INFO_LABEL_TEXTURE_POS, tex_text)
	_set_debug_info_label(3, DEBUG_INFO_LABEL_BUDGET_POS, budget_text)

	if kind == "spot":
		var inner_anchor: Vector3 = _get_debug_spot_circle_anchor(light_range, inner_angle, -1.0)
		var outer_anchor: Vector3 = _get_debug_spot_circle_anchor(light_range, outer_angle, 1.0)
		_set_debug_info_label(
			4, inner_anchor + Vector3(-0.36, -0.14, 0.0), "inner:" + _fmtf(inner_angle)
		)
		_set_debug_info_label(
			5, outer_anchor + Vector3(0.36, 0.14, 0.0), "outer:" + _fmtf(outer_angle)
		)
	else:
		_set_debug_info_label(4, Vector3.ZERO, "")
		_set_debug_info_label(5, Vector3.ZERO, "")

	_update_debug_info_lines()


func _set_debug_info_label(index: int, pos: Vector3, text: String) -> void:
	if index < 0 or index >= debug_info_labels.size():
		return

	var label := debug_info_labels[index]
	if label == null:
		return

	label.position = pos
	label.text = text
	label.visible = text != ""


func _update_debug_info_lines() -> void:
	if debug_info_lines_mesh == null:
		return

	var mesh := ImmediateMesh.new()
	var mat := _make_debug_material("info_line", 0.85)

	mesh.surface_begin(Mesh.PRIMITIVE_LINES, mat)

	for i in range(debug_info_labels.size()):
		var label := debug_info_labels[i]
		if label == null or not label.visible:
			continue

		var line_start: Vector3 = Vector3.ZERO
		var line_end: Vector3 = label.position * 0.86

		if last_kind == "spot":
			if i == 4:
				line_start = _get_debug_spot_circle_anchor(last_light_range, last_inner_angle, -1.0)
				line_end = label.position
			elif i == 5:
				line_start = _get_debug_spot_circle_anchor(last_light_range, last_outer_angle, 1.0)
				line_end = label.position

		mesh.surface_add_vertex(line_start)
		mesh.surface_add_vertex(line_end)

	mesh.surface_end()

	debug_info_lines_mesh.mesh = mesh
	debug_info_lines_mesh.position = Vector3.ZERO
	debug_info_lines_mesh.rotation_degrees = Vector3.ZERO
	debug_info_lines_mesh.scale = Vector3.ONE


# Gizmo geometry shows the REAL light coverage: cone length = authored range,
# radius from the authored angle. Wide angles can produce huge radii, so the
# radius is capped as a readability tradeoff (length stays truthful).
func _get_debug_spot_cone_dimensions(light_range: float, angle_degrees: float) -> Dictionary:
	var safe_angle: float = clampf(angle_degrees, 0.1, 175.0)
	var half_angle_rad: float = deg_to_rad(safe_angle * 0.5)
	var tan_half_angle: float = tan(half_angle_rad)

	var cone_length: float = maxf(light_range, DEBUG_SPOT_CONE_MIN_LENGTH)
	var cone_radius: float = minf(tan_half_angle * cone_length, DEBUG_SPOT_CONE_MAX_RADIUS)

	return {
		"length": cone_length,
		"radius": cone_radius,
	}


func _get_debug_spot_circle_anchor(
	light_range: float, angle_degrees: float, side: float
) -> Vector3:
	var dimensions: Dictionary = _get_debug_spot_cone_dimensions(light_range, angle_degrees)
	return Vector3(side * float(dimensions["radius"]), 0.0, -float(dimensions["length"]))


func _fmtf(v: float) -> String:
	return str(snappedf(v, 0.01))


func _short_path(path: String) -> String:
	if path.length() <= DEBUG_TEXTURE_PATH_MAX_LEN:
		return path
	return (
		"..." + path.substr(path.length() - DEBUG_TEXTURE_PATH_MAX_LEN, DEBUG_TEXTURE_PATH_MAX_LEN)
	)
