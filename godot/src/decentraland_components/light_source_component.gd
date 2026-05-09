class_name DclLightSourceComponent
extends Node3D

const DEFAULT_PROJECTOR_TEXTURE_PATH: String = "res://assets/white_pixel.png"

# Runtime-editable light settings.
# These are intentionally static vars, not consts, so the settings menu can modify them.
static var dcl_lights_system_enabled: bool = true
static var dcl_lights_rendering_enabled: bool = false
static var force_dcl_light_shadows_off: bool = true

static var debug_dcl_lights_gizmo: bool = false

static var auto_activation_range: bool = true
static var use_global_light_budget: bool = true
static var max_active_dcl_lights: int = 4

static var projector_texture_cache: Dictionary = {}
var debug_info_update_timer: float = 0.0
const DEBUG_DCL_LIGHTS_LOG: bool = false

const DCL_LIGHT_AUTO_RANGE_DIVISOR: float = 160.0

# If true, the actual Godot Light3D only renders while the avatar is inside activation range.
# The debug gizmo stays visible either way.
const ENABLE_LIGHT_ONLY_WHEN_AVATAR_IN_RANGE: bool = true

# Fixed-size debug gizmo. This does NOT scale with real light range.
const DEBUG_SPOT_CONE_MAX_LENGTH: float = 2.0
const DEBUG_SPOT_CONE_MAX_RADIUS: float = 1.35
const DEBUG_SPOT_CONE_MIN_LENGTH: float = 0.18
const DEBUG_POINT_SPHERE_RADIUS: float = 1.0
const DEBUG_SPOT_CONE_SEGMENTS: int = 32
const DEBUG_POINT_SPHERE_SEGMENTS: int = 32

const DEBUG_TEXTURE_PATH_MAX_LEN: int = 34
# Godot needs shadow_enabled=true for light_projector to work.
# So "shadows off" should not necessarily disable Light3D.shadow_enabled.
# Instead it removes dynamic shadow casters from the shadow_caster_mask.
const DCL_LIGHT_SHADOW_CASTER_MASK_ALL: int = 0xFFFFFFFF
const DCL_LIGHT_SHADOW_CASTER_MASK_NONE: int = 0

const DEBUG_INFO_LABEL_PIXEL_SIZE: float = 0.000625
const DEBUG_INFO_LABEL_OUTLINE_SIZE: int = 2

const DEBUG_INFO_LABEL_INTENSITY_POS: Vector3 = Vector3(0.0, 0.62, 0.0)
const DEBUG_INFO_LABEL_RANGE_POS: Vector3 = Vector3(-0.95, 0.15, 0.0)
const DEBUG_INFO_LABEL_TEXTURE_POS: Vector3 = Vector3(1.05, 0.15, 0.0)
const DEBUG_INFO_LABEL_BUDGET_POS: Vector3 = Vector3(0.0, -0.42, 0.0)

const DEBUG_STATUS_LABEL_PIXEL_SIZE: float = 0.0011
const DEBUG_STATUS_LABEL_OUTLINE_SIZE: int = 0
const DEBUG_STATUS_LABEL_POS: Vector3 = Vector3.ZERO

static var registered_lights: Array = []
static var debug_has_global_avatar_position: bool = false
static var debug_global_avatar_position: Vector3 = Vector3.ZERO

var current_light: Light3D = null
var projector_texture: Texture2D = null
var projector_texture_path: String = ""
var projector_texture_display_path: String = ""
var pending_http_request: HTTPRequest = null

var debug_last_kind: String = ""
var debug_last_intensity: float = 0.0
var debug_last_light_range: float = 0.0
var debug_last_inner_angle: float = 0.0
var debug_last_outer_angle: float = 0.0

# External runtime limits. Rust can call set_light_enabled without fighting
# the avatar-range visibility.
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
		if light == null or not is_instance_valid(light):
			continue

		if debug_enabled:
			light._ensure_debug_gizmo()

			if light.debug_last_kind != "":
				light._update_debug_gizmo(
					light.debug_last_kind,
					light.debug_last_intensity,
					light.debug_last_light_range,
					light.debug_last_inner_angle,
					light.debug_last_outer_angle
				)

		if light.debug_gizmo_root != null:
			light.debug_gizmo_root.visible = debug_enabled

		light._update_light_rendering_by_avatar_range()
		if light.current_light is SpotLight3D:
			light._apply_projector_texture()
	_update_global_light_budget()


static func get_light_settings() -> Dictionary:
	return {
		"lights_enabled": dcl_lights_system_enabled and dcl_lights_rendering_enabled,
		"shadows_enabled": not force_dcl_light_shadows_off,
		"max_lights": max_active_dcl_lights,
		"debug_enabled": debug_dcl_lights_gizmo,
		"auto_activation_range": auto_activation_range,
		"use_global_light_budget": use_global_light_budget,
	}


static func set_global_light_reference_position(global_pos: Vector3) -> void:
	debug_global_avatar_position = global_pos
	debug_has_global_avatar_position = true
	_update_global_light_budget()


static func clear_global_light_reference_position() -> void:
	debug_global_avatar_position = Vector3.ZERO
	debug_has_global_avatar_position = false

	for light in registered_lights:
		if light == null or not is_instance_valid(light):
			continue

		light.budget_light_enabled = false
		light.debug_budget_rank = -1
		light.debug_budget_candidate = false
		light.debug_last_distance_to_avatar = -1.0
		light._update_light_rendering_by_avatar_range()

func _set_light_render_state(should_render: bool) -> void:
	if current_light == null:
		return

	current_light.visible = should_render

	if not should_render:
		current_light.shadow_enabled = false
		current_light.shadow_caster_mask = DCL_LIGHT_SHADOW_CASTER_MASK_NONE
		return

	var has_projector: bool = (
		current_light is SpotLight3D
		and projector_texture_path != ""
		and projector_texture_path != DEFAULT_PROJECTOR_TEXTURE_PATH
	)

	# Important:
	# Godot requires shadow_enabled=true for light_projector to work.
	# Therefore projector lights must keep shadow_enabled=true even when
	# the user disabled dynamic shadows.
	current_light.shadow_enabled = runtime_shadows_enabled or has_projector

	if force_dcl_light_shadows_off:
		current_light.shadow_caster_mask = DCL_LIGHT_SHADOW_CASTER_MASK_NONE
	else:
		current_light.shadow_caster_mask = DCL_LIGHT_SHADOW_CASTER_MASK_ALL

static func _update_global_light_budget() -> void:
	if not use_global_light_budget:
		for light in registered_lights:
			if light == null or not is_instance_valid(light):
				continue

			light.budget_light_enabled = true
			light.debug_budget_rank = -1
			light.debug_budget_candidate = false
			light.debug_last_distance_to_avatar = -1.0
			light._update_light_rendering_by_avatar_range()

		return

	var candidates: Array = []

	for light in registered_lights:
		if light == null or not is_instance_valid(light):
			continue

		light.budget_light_enabled = false
		light.debug_budget_rank = -1
		light.debug_budget_candidate = false
		light.debug_last_distance_to_avatar = -1.0

		if light.current_light == null:
			light._update_light_rendering_by_avatar_range()
			continue

		if light.debug_last_kind == "":
			light._update_light_rendering_by_avatar_range()
			continue

		if not light.runtime_light_enabled:
			light._update_light_rendering_by_avatar_range()
			continue

		var range_state: int = light._get_avatar_range_state(light.debug_last_light_range)

		if range_state != 1:
			light._update_light_rendering_by_avatar_range()
			continue

		var distance_to_avatar: float = light.global_position.distance_to(
			debug_global_avatar_position
		)
		light.debug_last_distance_to_avatar = distance_to_avatar
		light.debug_budget_candidate = true

		candidates.append({
			"light": light,
			"distance": distance_to_avatar
		})

	candidates.sort_custom(
		func(a: Dictionary, b: Dictionary) -> bool:
			return float(a["distance"]) < float(b["distance"])
	)

	for i: int in range(candidates.size()):
		var light: DclLightSourceComponent = candidates[i]["light"] as DclLightSourceComponent

		if light == null or not is_instance_valid(light):
			continue

		light.debug_budget_rank = i + 1
		light.budget_light_enabled = i < max_active_dcl_lights
		light._update_light_rendering_by_avatar_range()


func _ready() -> void:
	if not registered_lights.has(self):
		registered_lights.append(self)

	if projector_texture_path == "":
		set_projector_texture(DEFAULT_PROJECTOR_TEXTURE_PATH)

	if debug_dcl_lights_gizmo:
		_ensure_debug_gizmo()


func _exit_tree() -> void:
	registered_lights.erase(self)
	_update_global_light_budget()


func _process(_delta: float) -> void:
	if debug_last_kind == "":
		return

	_update_light_rendering_by_avatar_range()

	if not debug_dcl_lights_gizmo:
		if debug_gizmo_root != null:
			debug_gizmo_root.visible = false
		return

	if debug_gizmo_root == null:
		_update_debug_gizmo(
			debug_last_kind,
			debug_last_intensity,
			debug_last_light_range,
			debug_last_inner_angle,
			debug_last_outer_angle
		)
		return

	debug_gizmo_root.visible = true

	_update_debug_origin_marker(debug_last_kind, debug_last_light_range)

	debug_info_update_timer -= _delta
	if debug_info_update_timer <= 0.0:
		debug_info_update_timer = 0.15
		_update_debug_info_labels(
			debug_last_kind,
			debug_last_intensity,
			debug_last_light_range,
			debug_last_inner_angle,
			debug_last_outer_angle
		)


func set_debug_entity_id(entity_id: String) -> void:
	# Kept for Rust compatibility. Entity display is intentionally ignored for now.
	pass


func get_projector_texture_path() -> String:
	return projector_texture_path


func set_projector_texture(path: String, display_path: String = "") -> void:
	# Default texture should never appear as debug texture path.
	if path == DEFAULT_PROJECTOR_TEXTURE_PATH:
		projector_texture_display_path = ""
	elif display_path != "":
		# This should be the authored scene path, e.g. "/images/example.png".
		projector_texture_display_path = display_path
	elif projector_texture_display_path == "" and _looks_like_authored_texture_path(path):
		# Fallback only for real authored/local paths, not hashes or resolved content URLs.
		projector_texture_display_path = path

	if projector_texture_path == path:
		if projector_texture != null:
			return

		if pending_http_request != null:
			return

	projector_texture_path = path
	projector_texture = null

	if path == "":
		if DEBUG_DCL_LIGHTS_LOG:
			push_warning("[DCL LIGHT] Empty projector texture path")
		return

	if path.begins_with("https://"):
		_load_projector_texture_http(path)
		return

	if not ResourceLoader.exists(path):
		if DEBUG_DCL_LIGHTS_LOG:
			push_warning("[DCL LIGHT] Resource does not exist: " + path)
		return

	var res: Resource = load(path)

	if res == null:
		if DEBUG_DCL_LIGHTS_LOG:
			push_warning("[DCL LIGHT] load() returned null: " + path)
		return

	projector_texture = res as Texture2D

	if projector_texture == null:
		if DEBUG_DCL_LIGHTS_LOG:
			push_warning("[DCL LIGHT] Resource is not Texture2D: " + path)
		return

	_apply_projector_texture()

func set_projector_texture_display_path(display_path: String) -> void:
	projector_texture_display_path = display_path


func _looks_like_authored_texture_path(path: String) -> bool:
	if path == "":
		return false

	if path == DEFAULT_PROJECTOR_TEXTURE_PATH:
		return false

	if path.begins_with("https://"):
		return false

	if path.begins_with("ipfs://"):
		return false

	# DCL-authored paths normally look like "/images/example.png".
	if path.begins_with("/"):
		return true

	if path.begins_with("res://"):
		return true

	if path.begins_with("user://"):
		return true

	return false

func _load_projector_texture_http(url: String) -> void:
	# Texture already loaded by another light.
	if projector_texture_cache.has(url):
		projector_texture = projector_texture_cache[url] as Texture2D
		_apply_projector_texture()

		if DEBUG_DCL_LIGHTS_LOG:
			print("[DCL LIGHTS] HTTP projector cache HIT url=", url)

		return

	if pending_http_request != null:
		pending_http_request.queue_free()
		pending_http_request = null

	var request: HTTPRequest = HTTPRequest.new()
	pending_http_request = request
	add_child(request)

	request.request_completed.connect(
		func(
			result: int,
			response_code: int,
			headers: PackedStringArray,
			body: PackedByteArray
		) -> void:
			if pending_http_request == request:
				pending_http_request = null

			request.queue_free()

			if projector_texture_path != url:
				return

			# Maybe another light finished loading the same URL while this request was running.
			if projector_texture_cache.has(url):
				projector_texture = projector_texture_cache[url] as Texture2D
				_apply_projector_texture()
				return

			if result != HTTPRequest.RESULT_SUCCESS or response_code < 200 or response_code >= 300:
				if DEBUG_DCL_LIGHTS_LOG:
					push_warning(
						"[DCL LIGHT] HTTP texture failed: " + url + " code=" + str(response_code)
					)
				return

			var image: Image = Image.new()
			var err: Error = image.load_png_from_buffer(body)

			if err != OK:
				err = image.load_jpg_from_buffer(body)

			if err != OK:
				err = image.load_webp_from_buffer(body)

			if err != OK:
				if DEBUG_DCL_LIGHTS_LOG:
					push_warning("[DCL LIGHT] Could not decode image from: " + url)
				return

			image.resize(512, 512, Image.INTERPOLATE_LANCZOS)

			var loaded_texture: Texture2D = ImageTexture.create_from_image(image)
			projector_texture_cache[url] = loaded_texture
			projector_texture = loaded_texture

			if DEBUG_DCL_LIGHTS_LOG:
				print("[DCL LIGHTS] HTTP projector loaded OK url=", url, " size=", image.get_size())

			_apply_projector_texture()
	)

	var err: Error = request.request(url)
	if err != OK:
		if DEBUG_DCL_LIGHTS_LOG:
			push_warning("[DCL LIGHT] HTTP request failed to start: " + url)

		request.queue_free()

		if pending_http_request == request:
			pending_http_request = null


func _apply_projector_texture() -> void:
	if current_light is SpotLight3D and projector_texture != null:
		var spot: SpotLight3D = current_light as SpotLight3D
		spot.light_projector = projector_texture


func set_spot(
	color: Color,
	intensity: float,
	light_range: float,
	inner_angle: float,
	outer_angle: float
) -> void:
	if current_light == null or not (current_light is SpotLight3D):
		_clear_light()

		current_light = SpotLight3D.new()
		current_light.name = "DCL_SpotLight"

		# Keep the light rotation exactly as authored by the DCL entity transform.
		current_light.rotation_degrees = Vector3(0.0, 0.0, 0.0)

		add_child(current_light)

	var spot: SpotLight3D = current_light as SpotLight3D

	spot.light_color = color
	spot.light_energy = _convert_intensity(intensity)
	spot.spot_range = light_range
	spot.spot_angle = outer_angle
	spot.spot_angle_attenuation = 1.0

	debug_last_kind = "spot"
	debug_last_intensity = intensity
	debug_last_light_range = light_range
	debug_last_inner_angle = inner_angle
	debug_last_outer_angle = outer_angle

	if projector_texture == null and projector_texture_path != "":
		set_projector_texture(projector_texture_path)

	_apply_projector_texture()

	if debug_dcl_lights_gizmo:
		_update_debug_gizmo("spot", intensity, light_range, inner_angle, outer_angle)

	_update_light_rendering_by_avatar_range()
	_update_global_light_budget()

	if DEBUG_DCL_LIGHTS_LOG:
		_debug_log_light_state("spot", intensity, light_range, inner_angle, outer_angle)


func set_point(
	color: Color,
	intensity: float,
	light_range: float
) -> void:
	if current_light == null or not (current_light is OmniLight3D):
		_clear_light()

		current_light = OmniLight3D.new()
		current_light.name = "DCL_OmniLight"

		# Keep the light rotation exactly as authored by the DCL entity transform.
		current_light.rotation_degrees = Vector3(0.0, 0.0, 0.0)

		add_child(current_light)

	var omni: OmniLight3D = current_light as OmniLight3D

	omni.light_color = color
	omni.light_energy = _convert_intensity(intensity)
	omni.omni_range = light_range

	debug_last_kind = "point"
	debug_last_intensity = intensity
	debug_last_light_range = light_range
	debug_last_inner_angle = 0.0
	debug_last_outer_angle = 0.0

	if debug_dcl_lights_gizmo:
		_update_debug_gizmo("point", intensity, light_range, 0.0, 0.0)

	_update_light_rendering_by_avatar_range()
	_update_global_light_budget()

	if DEBUG_DCL_LIGHTS_LOG:
		_debug_log_light_state("point", intensity, light_range, 0.0, 0.0)


func remove_light() -> void:
	_clear_light()

	if debug_gizmo_root != null:
		debug_gizmo_root.visible = false


func set_light_enabled(enabled: bool) -> void:
	runtime_light_enabled = enabled
	visible = enabled and dcl_lights_system_enabled

	_update_global_light_budget()
	_update_light_rendering_by_avatar_range()

	if debug_gizmo_root != null:
		debug_gizmo_root.visible = debug_dcl_lights_gizmo


func _update_light_rendering_by_avatar_range() -> void:
	if current_light == null:
		return

	if not dcl_lights_system_enabled:
		_set_light_render_state(false)
		return

	if not dcl_lights_rendering_enabled:
		_set_light_render_state(false)
		return

	if not runtime_light_enabled:
		_set_light_render_state(false)
		return

	if not ENABLE_LIGHT_ONLY_WHEN_AVATAR_IN_RANGE:
		var should_render_without_range: bool = true

		if use_global_light_budget:
			should_render_without_range = budget_light_enabled

		_set_light_render_state(should_render_without_range)
		return

	var range_state: int = _get_avatar_range_state(debug_last_light_range)

	if range_state == -1:
		_set_light_render_state(false)
		return

	var inside_range: bool = range_state == 1
	var allowed_by_budget: bool = true

	if use_global_light_budget:
		allowed_by_budget = budget_light_enabled

	var should_render: bool = inside_range and allowed_by_budget

	_set_light_render_state(should_render)


func _clear_light() -> void:
	if current_light != null:
		current_light.queue_free()
		current_light = null


func _convert_intensity(intensity: float) -> float:
	return intensity / 16000.0


func _debug_log_light_state(
	kind: String,
	original_intensity: float,
	light_range: float,
	inner_angle: float,
	outer_angle: float
) -> void:
	if current_light == null:
		print("[DCL LIGHTS] gd kind=", kind, " current_light=null")
		return

	var texture_info: String = "none"
	if projector_texture != null:
		texture_info = str(projector_texture.get_size())

	var global_forward_minus_z: Vector3 = -current_light.global_transform.basis.z.normalized()
	var global_forward_plus_z: Vector3 = current_light.global_transform.basis.z.normalized()

	var base: String = "[DCL LIGHTS] gd kind=%s light=%s original_intensity=%s converted_energy=%s range=%s color=%s shadow=%s visible=%s projector_path=%s projector_texture=%s component_global_pos=%s component_global_rot=%s light_local_rot=%s light_global_rot=%s global_forward_minus_z=%s global_forward_plus_z=%s" % [
		kind,
		current_light.name,
		str(original_intensity),
		str(current_light.light_energy),
		str(light_range),
		str(current_light.light_color),
		str(current_light.shadow_enabled),
		str(current_light.visible),
		projector_texture_path,
		texture_info,
		str(global_position),
		str(global_rotation_degrees),
		str(current_light.rotation_degrees),
		str(current_light.global_rotation_degrees),
		str(global_forward_minus_z),
		str(global_forward_plus_z)
	]

	if current_light is SpotLight3D:
		var spot: SpotLight3D = current_light as SpotLight3D
		print(
			base,
			" inner_angle=", inner_angle,
			" outer_angle=", outer_angle,
			" spot_range=", spot.spot_range,
			" spot_angle=", spot.spot_angle,
			" spot_angle_attenuation=", spot.spot_angle_attenuation,
			" has_projector=", spot.light_projector != null
		)
	elif current_light is OmniLight3D:
		var omni: OmniLight3D = current_light as OmniLight3D
		print(base, " omni_range=", omni.omni_range)
	else:
		print(base)


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

func _make_debug_material(kind: String, alpha: float = 1.0) -> StandardMaterial3D:
	var mat: StandardMaterial3D = StandardMaterial3D.new()

	var color: Color
	if kind == "spot_outer":
		color = Color(1.0, 0.55, 0.10, alpha)
	elif kind == "spot_inner":
		color = Color(1.0, 1.0, 0.15, alpha)
	elif kind == "inside_range":
		color = Color(0.20, 1.0, 0.25, alpha)
	elif kind == "inside_range_no_budget":
		color = Color(1.0, 0.9, 0.05, alpha)
	elif kind == "outside_range":
		color = Color(1.0, 0.15, 0.15, alpha)
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
	kind: String,
	intensity: float,
	light_range: float,
	inner_angle: float,
	outer_angle: float
) -> void:
	_ensure_debug_gizmo()

	if debug_gizmo_root == null:
		return

	debug_gizmo_root.visible = true
	debug_gizmo_root.position = Vector3.ZERO
	debug_gizmo_root.rotation_degrees = Vector3.ZERO
	debug_gizmo_root.scale = Vector3.ONE

	_update_debug_origin_marker(kind, light_range)

	if kind == "spot":
		if debug_spot_outer_cone_mesh != null:
			debug_spot_outer_cone_mesh.visible = true
		if debug_spot_inner_cone_mesh != null:
			debug_spot_inner_cone_mesh.visible = true
		if debug_point_sphere_mesh != null:
			debug_point_sphere_mesh.visible = false

		_update_debug_spot_cones(inner_angle, outer_angle)
	else:
		if debug_spot_outer_cone_mesh != null:
			debug_spot_outer_cone_mesh.visible = false
		if debug_spot_inner_cone_mesh != null:
			debug_spot_inner_cone_mesh.visible = false
		if debug_point_sphere_mesh != null:
			debug_point_sphere_mesh.visible = true

		_update_debug_point_sphere()

	_update_debug_info_labels(kind, intensity, light_range, inner_angle, outer_angle)


func _update_debug_origin_marker(_kind: String, light_range: float) -> void:
	if debug_status_label == null:
		return

	var range_state: int = _get_avatar_range_state(light_range)

	if range_state == 1:
		if use_global_light_budget and not budget_light_enabled:
			debug_status_label.text = "🟡"
		else:
			debug_status_label.text = "🟢"
	else:
		debug_status_label.text = "🔴"

	debug_status_label.position = DEBUG_STATUS_LABEL_POS


func _update_debug_spot_cones(inner_angle: float, outer_angle: float) -> void:
	_update_debug_spot_cone_mesh(
		debug_spot_outer_cone_mesh,
		outer_angle,
		"spot_outer",
		true
	)

	_update_debug_spot_cone_mesh(
		debug_spot_inner_cone_mesh,
		inner_angle,
		"spot_inner",
		false
	)


func _update_debug_spot_cone_mesh(
	target_mesh_instance: MeshInstance3D,
	angle_degrees: float,
	material_kind: String,
	draw_rays: bool
) -> void:
	if target_mesh_instance == null:
		return

	var dimensions: Dictionary = _get_debug_spot_cone_dimensions(angle_degrees)
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
	target_mesh_instance.material_override = null
	target_mesh_instance.position = Vector3.ZERO
	target_mesh_instance.rotation_degrees = Vector3.ZERO
	target_mesh_instance.scale = Vector3.ONE


func _update_debug_point_sphere() -> void:
	if debug_point_sphere_mesh == null:
		return

	var mesh: ImmediateMesh = ImmediateMesh.new()
	var mat: StandardMaterial3D = _make_debug_material("point", 0.95)

	var radius: float = DEBUG_POINT_SPHERE_RADIUS

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
	debug_point_sphere_mesh.material_override = null
	debug_point_sphere_mesh.position = Vector3.ZERO
	debug_point_sphere_mesh.rotation_degrees = Vector3.ZERO
	debug_point_sphere_mesh.scale = Vector3.ONE

func _make_center_status_label(name: String) -> Label3D:
	var label := Label3D.new()
	label.name = name
	label.position = DEBUG_STATUS_LABEL_POS
	label.fixed_size = true
	label.pixel_size = DEBUG_STATUS_LABEL_PIXEL_SIZE
	label.outline_size = DEBUG_STATUS_LABEL_OUTLINE_SIZE
	label.no_depth_test = true
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.text = "🟢"
	label.modulate = Color(1.0, 1.0, 1.0, 1.0)
	return label

func _update_debug_info_labels(
	kind: String,
	intensity: float,
	light_range: float,
	inner_angle: float,
	outer_angle: float
) -> void:
	if debug_info_labels.size() < 6:
		return

	var tex_text: String = "tex:n"
	if projector_texture_path != "" and projector_texture_path != DEFAULT_PROJECTOR_TEXTURE_PATH:
		var debug_texture_path: String = projector_texture_display_path

		if debug_texture_path == "":
			debug_texture_path = projector_texture_path

		tex_text = "tex:" + _short_path(debug_texture_path)

	var budget_text: String = ""
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
		var inner_anchor: Vector3 = _get_debug_spot_circle_anchor(inner_angle, -1.0)
		var outer_anchor: Vector3 = _get_debug_spot_circle_anchor(outer_angle, 1.0)

		var inner_label_pos: Vector3 = inner_anchor + Vector3(-0.36, -0.14, 0.0)
		var outer_label_pos: Vector3 = outer_anchor + Vector3(0.36, 0.14, 0.0)

		_set_debug_info_label(
			4,
			inner_label_pos,
			"inner:" + _fmtf(inner_angle)
		)

		_set_debug_info_label(
			5,
			outer_label_pos,
			"outer:" + _fmtf(outer_angle)
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

		if label == null:
			continue

		if not label.visible:
			continue

		var line_start: Vector3 = Vector3.ZERO
		var line_end: Vector3 = label.position * 0.86

		if debug_last_kind == "spot":
			if i == 4:
				line_start = _get_debug_spot_circle_anchor(debug_last_inner_angle, -1.0)
				line_end = label.position
			elif i == 5:
				line_start = _get_debug_spot_circle_anchor(debug_last_outer_angle, 1.0)
				line_end = label.position

		mesh.surface_add_vertex(line_start)
		mesh.surface_add_vertex(line_end)

	mesh.surface_end()

	debug_info_lines_mesh.mesh = mesh
	debug_info_lines_mesh.material_override = null
	debug_info_lines_mesh.position = Vector3.ZERO
	debug_info_lines_mesh.rotation_degrees = Vector3.ZERO
	debug_info_lines_mesh.scale = Vector3.ONE


func _get_avatar_range_state(light_range: float) -> int:
	if not debug_has_global_avatar_position:
		return -1

	var activation_range: float = _get_activation_range(light_range)

	if activation_range <= 0.0:
		return -1

	var distance_to_avatar: float = global_position.distance_to(debug_global_avatar_position)
	if distance_to_avatar <= activation_range:
		return 1

	return 0


func _get_activation_range(light_range: float) -> float:
	if not auto_activation_range:
		return light_range

	var intensity_range: float = sqrt(
		maxf(debug_last_intensity, 0.0) / DCL_LIGHT_AUTO_RANGE_DIVISOR
	)

	return maxf(light_range, intensity_range)


func _get_debug_spot_cone_dimensions(angle_degrees: float) -> Dictionary:
	var safe_angle: float = clampf(angle_degrees, 0.1, 175.0)
	var half_angle_rad: float = deg_to_rad(safe_angle * 0.5)
	var tan_half_angle: float = tan(half_angle_rad)

	var cone_length: float = DEBUG_SPOT_CONE_MAX_LENGTH

	if tan_half_angle > 0.0:
		var length_for_max_radius: float = DEBUG_SPOT_CONE_MAX_RADIUS / tan_half_angle
		cone_length = minf(DEBUG_SPOT_CONE_MAX_LENGTH, length_for_max_radius)

	cone_length = maxf(cone_length, DEBUG_SPOT_CONE_MIN_LENGTH)

	var cone_radius: float = tan_half_angle * cone_length

	return {
		"length": cone_length,
		"radius": cone_radius,
	}


func _get_debug_spot_circle_anchor(angle_degrees: float, side: float) -> Vector3:
	var dimensions: Dictionary = _get_debug_spot_cone_dimensions(angle_degrees)
	var cone_length: float = float(dimensions["length"])
	var cone_radius: float = float(dimensions["radius"])

	return Vector3(side * cone_radius, 0.0, -cone_length)


func _fmtf(v: float) -> String:
	return str(snappedf(v, 0.01))


func _short_path(path: String) -> String:
	if path.length() <= DEBUG_TEXTURE_PATH_MAX_LEN:
		return path

	return "..." + path.substr(path.length() - DEBUG_TEXTURE_PATH_MAX_LEN, DEBUG_TEXTURE_PATH_MAX_LEN)
