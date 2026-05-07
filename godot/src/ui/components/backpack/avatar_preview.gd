class_name AvatarPreview
extends SubViewportContainer

const MIN_CAMERA_SIZE_OVERALL = 1.0
const MIN_CAMERA_SIZE_PART = 0.2
const MAX_CAMERA_SIZE = 5.0
const CAMERA_PAN_SMOOTH = 4.0
const CAMERA_ZOOM_SMOOTH = 8.0
const AVATAR_ROTATION_SMOOTH = 15.0

@export var hide_name: bool = false
@export var show_platform: bool = false
## Enables rotation and pan/zoom interactions.
@export var can_move: bool = true
## When can_move is true, controls whether vertical pan is allowed. Rotation is always enabled.
@export var can_drag: bool = true
@export var custom_environment: Environment = null
@export var with_light: bool = true
@export var preview_margin_top: int = 0
@export var preview_margin_bottom: int = 0
@export var preview_margin_left: int = 0
@export var preview_margin_right: int = 0
## Optional Control whose bottom edge defines the top margin boundary.
@export var top_node_margin: Control = null
## Optional Control whose top edge defines the bottom margin boundary.
@export var bottom_node_margin: Control = null
@export var show_aabb_debug: bool = false

var start_angle
var start_dragging_position
var start_camera_center_y: float = 0.0
var dirty_is_dragging
var _camera_focus: String = "overall"

var _cached_aabbs: Dictionary = {}
var _user_has_panned: bool = false
var _pending_camera_fit: bool = false
var _fitted_camera_size: float = MAX_CAMERA_SIZE
var _fitted_aabb_center_y: float = 0.0
var _target_camera_center_y: float = 0.0
var _target_camera_size: float = MAX_CAMERA_SIZE
var _target_avatar_rotation_y: float = 0.0
var _lerp_paused: bool = false
var _touch_points: Dictionary = {}
var _pinch_start_distance: float = 0.0
var _pinch_start_camera_size: float = 0.0

var _aabb_debug_nodes: Array[MeshInstance3D] = []

@onready var avatar = %Avatar
@onready var camera_center: Node3D = %CameraCenter
@onready var camera_3d: Camera3D = %Camera3D
@onready var platform = %Platform
@onready var subviewport: SubViewport = %SubViewport
@onready var world_environment = $SubViewport/WorldEnvironment
@onready var directional_light_3d = $SubViewport/DirectionalLight3D
@onready var outline_system = %OutlineSystem


func _ready():
	if custom_environment != null:
		world_environment.environment = custom_environment

	directional_light_3d.visible = with_light

	avatar.hide_name = hide_name
	platform.set_visible(show_platform)

	if outline_system:
		outline_system.setup(camera_3d)

	#if can_move:
	#	gui_input.connect(self._on_gui_input)
	set_process_input(true)

	_target_camera_center_y = camera_center.position.y
	_target_camera_size = camera_3d.size
	_target_avatar_rotation_y = avatar.rotation.y

	avatar.avatar_loaded.connect(_on_avatar_loaded)
	resized.connect(_on_resized)
	if top_node_margin:
		top_node_margin.resized.connect(_on_resized)
	if bottom_node_margin:
		bottom_node_margin.resized.connect(_on_resized)

	if Global.standalone:
		Global.player_identity.set_default_profile()
		var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
		avatar.async_update_avatar_from_profile(profile)


func _process(delta: float) -> void:
	if _lerp_paused:
		return
	if can_drag and can_move and _pinch_start_distance > 0.0 and _touch_points.size() >= 2:
		var current_dist: float = _get_touch_distance()
		if current_dist > 0.0:
			_target_camera_size = clampf(
				_pinch_start_camera_size * _pinch_start_distance / current_dist,
				_min_camera_size(),
				_fitted_camera_size
			)
			_clamp_camera_center()
	camera_center.position.y = lerpf(
		camera_center.position.y, _target_camera_center_y, 1.0 - exp(-CAMERA_PAN_SMOOTH * delta)
	)
	camera_3d.size = lerpf(
		camera_3d.size, _target_camera_size, 1.0 - exp(-CAMERA_ZOOM_SMOOTH * delta)
	)
	avatar.rotation.y = lerpf(
		avatar.rotation.y, _target_avatar_rotation_y, 1.0 - exp(-AVATAR_ROTATION_SMOOTH * delta)
	)


func focus_camera_on(type):
	match type:
		Wearables.Categories.HAIR, Wearables.Categories.FACIAL_HAIR, Wearables.Categories.EYEWEAR, Wearables.Categories.TIARA, Wearables.Categories.FACIAL, Wearables.Categories.EYEBROWS, Wearables.Categories.MOUTH, Wearables.Categories.HAT, Wearables.Categories.EARRING, Wearables.Categories.MASK, Wearables.Categories.HELMET, Wearables.Categories.TOP_HEAD, Wearables.Categories.EYES:
			_camera_focus = "head"
		Wearables.Categories.UPPER_BODY:
			_camera_focus = "torso"
		Wearables.Categories.HANDS_WEAR, Wearables.Categories.HANDS:
			_camera_focus = "hands"
		Wearables.Categories.LOWER_BODY:
			_camera_focus = "legs"
		Wearables.Categories.FEET:
			_camera_focus = "feet"
		_:
			_camera_focus = "overall"
	_user_has_panned = false
	var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
	if aabb_key in _cached_aabbs:
		_fit_camera_to_aabb(_cached_aabbs[aabb_key], _focus_extra_margin())


func _min_camera_size() -> float:
	var floor_size := (
		MIN_CAMERA_SIZE_OVERALL if _camera_focus == "overall" else MIN_CAMERA_SIZE_PART
	)
	return (_fitted_camera_size + floor_size) / 2.0


func _focus_padding() -> float:
	match _camera_focus:
		"hands":
			return 0.3
		"feet", "torso":
			return 0.2
		"head":
			return 0.1
		_:
			return 0.0


func _focus_aabb_center_y(aabb: AABB) -> float:
	if _camera_focus == "head":
		return aabb.position.y + aabb.size.y * 0.3
	return aabb.get_center().y


func _focus_extra_margin() -> int:
	match _camera_focus:
		"hands":
			return 80
		"feet", "head", "torso":
			return 40
		_:
			return 0


func _input(event: InputEvent):
	if not can_move:
		return

	var irect: Rect2 = get_global_rect()

	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				if not irect.has_point(event.position):
					return
				dirty_is_dragging = true
				start_dragging_position = get_global_mouse_position()
				start_angle = _target_avatar_rotation_y
				start_camera_center_y = _target_camera_center_y
			else:
				dirty_is_dragging = false

		if not event.pressed and can_drag and irect.has_point(event.position):
			var dir: float = 0.0
			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				dir = -0.2
			elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				dir = 0.2

			if dir != 0.0:
				_target_camera_size = clampf(
					_target_camera_size + dir, _min_camera_size(), _fitted_camera_size
				)
				_clamp_camera_center()

	# InputEventMagnifyGesture is intentionally not used: it only fires on macOS trackpads
	# and is not emitted on iOS or Android. Pinch zoom is handled via InputEventScreenTouch
	# and InputEventScreenDrag below, which works on all platforms.

	if event is InputEventScreenTouch:
		if event.pressed:
			_touch_points[event.index] = event.position
			if _touch_points.size() == 2 and can_drag:
				dirty_is_dragging = false
				_pinch_start_distance = _get_touch_distance()
				_pinch_start_camera_size = _target_camera_size
		else:
			_touch_points.erase(event.index)
			_pinch_start_distance = 0.0

	if event is InputEventScreenDrag:
		_touch_points[event.index] = event.position

	if event is InputEventMouseMotion:
		if dirty_is_dragging:
			_apply_drag(get_global_mouse_position())


func _pan_limits() -> Vector2:
	var focus_aabb: AABB = _cached_aabbs.get(
		_camera_focus, _cached_aabbs.get("overall", AABB(Vector3.ZERO, Vector3.ONE * 2.0))
	)
	var av_xform: Transform3D = avatar.global_transform
	var aabb_bottom: float = (av_xform * Vector3(0.0, focus_aabb.position.y, 0.0)).y
	var aabb_top: float = (
		(av_xform * Vector3(0.0, focus_aabb.position.y + focus_aabb.size.y, 0.0)).y
	)
	var cam_size: float = camera_3d.size
	var vp_h: float = size.y
	var pan_min: float = aabb_bottom + cam_size * (0.5 - _effective_margin_bottom() / vp_h)
	var pan_max: float = aabb_top - cam_size * (0.5 - _effective_margin_top() / vp_h)
	return Vector2(minf(pan_min, pan_max), maxf(pan_min, pan_max))


func _clamp_camera_center() -> void:
	if not _user_has_panned:
		_target_camera_center_y = (
			_fitted_aabb_center_y
			+ (
				(_effective_margin_top() - _effective_margin_bottom())
				* _target_camera_size
				/ (2.0 * size.y)
			)
		)
	var limits: Vector2 = _pan_limits()
	_target_camera_center_y = clampf(_target_camera_center_y, limits.x, limits.y)


func _apply_drag(current_pos: Vector2) -> void:
	var drag_pixels: Vector2 = current_pos - start_dragging_position
	_target_avatar_rotation_y = start_angle + drag_pixels.x * 0.005
	if not can_drag:
		return
	var limits: Vector2 = _pan_limits()
	var pan: float = drag_pixels.y * _target_camera_size / size.y
	_target_camera_center_y = clampf(start_camera_center_y + pan, limits.x, limits.y)
	_user_has_panned = true


func _get_touch_distance() -> float:
	var keys: Array = _touch_points.keys()
	return (_touch_points[keys[0]] as Vector2).distance_to(_touch_points[keys[1]])


func reset_avatar_rotation() -> void:
	_target_avatar_rotation_y = 0.0


func _on_resized() -> void:
	if _pending_camera_fit and size.x > 0.0 and size.y > 0.0:
		var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
		if aabb_key in _cached_aabbs:
			_fit_camera_to_aabb(_cached_aabbs[aabb_key], _focus_extra_margin())


func enable_outline():
	if outline_system and avatar:
		outline_system.set_outlined_avatar(avatar)


func disable_outline():
	if outline_system:
		outline_system.set_outlined_avatar(null)


func _on_avatar_loaded():
	_cached_aabbs = _compute_avatar_aabbs()
	_update_aabb_debug_box(_cached_aabbs)
	if not _user_has_panned and _camera_focus in _cached_aabbs:
		_fit_camera_to_aabb(_cached_aabbs[_camera_focus], _focus_extra_margin())


func _get_mesh_category(mesh_name: String) -> String:
	if (
		"__hair" in mesh_name
		or "__facial_hair" in mesh_name
		or "__eyewear" in mesh_name
		or "__earring" in mesh_name
		or "__tiara" in mesh_name
		or "__helmet" in mesh_name
		or "__mask" in mesh_name
		or "__top_head" in mesh_name
		or "_head_basemesh" in mesh_name
		or "_mask_" in mesh_name
	):
		return "head"
	if "__upper_body" in mesh_name or "_ubody_basemesh" in mesh_name:
		return "torso"
	if "__lower_body" in mesh_name or "_lbody_basemesh" in mesh_name:
		return "legs"
	if "__feet" in mesh_name or "_feet_basemesh" in mesh_name:
		return "feet"
	return ""


func _compute_skinned_mesh_aabb(
	mi: MeshInstance3D, skeleton: Skeleton3D, avatar_xform_inv: Transform3D
) -> AABB:
	if mi.mesh == null:
		return AABB()
	var skin: Skin = mi.skin
	if skin == null:
		var mesh_aabb: AABB = mi.mesh.get_aabb()
		if mesh_aabb.size == Vector3.ZERO:
			return AABB()
		var result := AABB()
		var first := true
		for i in 8:
			var local_pt: Vector3 = (
				avatar_xform_inv * (mi.global_transform * mesh_aabb.get_endpoint(i))
			)
			if first:
				result = AABB(local_pt, Vector3.ZERO)
				first = false
			else:
				result = result.expand(local_pt)
		return result

	var skin_count: int = skin.get_bind_count()
	if skin_count == 0:
		return AABB()
	if not skeleton.is_inside_tree():
		return AABB()
	var skeleton_global: Transform3D = skeleton.global_transform

	var skin_matrices: Array[Transform3D] = []
	skin_matrices.resize(skin_count)
	for i in skin_count:
		var bone_idx: int = skin.get_bind_bone(i)
		if bone_idx < 0:
			bone_idx = skeleton.find_bone(skin.get_bind_name(i))
		if bone_idx < 0 or bone_idx >= skeleton.get_bone_count():
			skin_matrices[i] = Transform3D.IDENTITY
			continue
		skin_matrices[i] = (
			skeleton_global * skeleton.get_bone_global_pose(bone_idx) * skin.get_bind_pose(i)
		)

	var result := AABB()
	var first := true

	for surf_idx in mi.mesh.get_surface_count():
		var arrays: Array = mi.mesh.surface_get_arrays(surf_idx)
		if arrays.is_empty():
			continue
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var bones_raw = arrays[Mesh.ARRAY_BONES]
		var weights_raw = arrays[Mesh.ARRAY_WEIGHTS]
		if bones_raw == null or weights_raw == null or vertices.is_empty():
			continue
		if not weights_raw is PackedFloat32Array:
			continue

		var bones: PackedInt32Array
		if bones_raw is PackedFloat32Array:
			bones = PackedInt32Array(bones_raw)
		elif bones_raw is PackedInt32Array:
			bones = bones_raw
		else:
			continue

		if bones.is_empty() or bones.size() % vertices.size() != 0:
			continue

		var influences: int = bones.size() / vertices.size()

		for v_idx in vertices.size():
			var skinned_pos := Vector3.ZERO
			for b in influences:
				var bind_idx: int = bones[v_idx * influences + b]
				var weight: float = weights_raw[v_idx * influences + b]
				if weight > 0.0 and bind_idx < skin_count:
					skinned_pos += weight * (skin_matrices[bind_idx] * vertices[v_idx])
			if skinned_pos == Vector3.ZERO:
				continue
			var local_pt: Vector3 = avatar_xform_inv * skinned_pos
			if first:
				result = AABB(local_pt, Vector3.ZERO)
				first = false
			else:
				result = result.expand(local_pt)

	return result


func _compute_avatar_aabbs() -> Dictionary:
	var skeleton: Skeleton3D = avatar.body_shape_skeleton_3d
	if skeleton == null:
		return {}
	var avatar_xform_inv: Transform3D = (avatar.global_transform as Transform3D).affine_inverse()
	var results: Dictionary = {}
	var firsts: Dictionary = {
		"overall": true, "head": true, "torso": true, "legs": true, "feet": true, "hands": true
	}
	for child in skeleton.get_children():
		var mi := child as MeshInstance3D
		if mi == null or mi.mesh == null:
			continue
		var is_basemesh: bool = "_basemesh" in mi.name
		if not mi.visible and not is_basemesh:
			continue
		var mesh_aabb: AABB = _compute_skinned_mesh_aabb(mi, skeleton, avatar_xform_inv)
		if mesh_aabb.size == Vector3.ZERO:
			continue
		if mi.visible:
			if firsts["overall"]:
				results["overall"] = mesh_aabb
				firsts["overall"] = false
			else:
				results["overall"] = (results["overall"] as AABB).merge(mesh_aabb)
		var cat: String = _get_mesh_category(mi.name)
		if cat != "":
			if firsts[cat]:
				results[cat] = mesh_aabb
				firsts[cat] = false
			else:
				results[cat] = (results[cat] as AABB).merge(mesh_aabb)
		if "_hands_basemesh" in mi.name:
			if firsts["hands"]:
				results["hands"] = mesh_aabb
				firsts["hands"] = false
			else:
				results["hands"] = (results["hands"] as AABB).merge(mesh_aabb)
		if "_lbody_basemesh" in mi.name:
			var knee_aabb := AABB(
				mesh_aabb.position,
				Vector3(mesh_aabb.size.x, mesh_aabb.size.y * 0.5, mesh_aabb.size.z)
			)
			if firsts["feet"]:
				results["feet"] = knee_aabb
				firsts["feet"] = false
			else:
				results["feet"] = (results["feet"] as AABB).merge(knee_aabb)
		if "_feet_basemesh" in mi.name:
			if firsts["legs"]:
				results["legs"] = mesh_aabb
				firsts["legs"] = false
			else:
				results["legs"] = (results["legs"] as AABB).merge(mesh_aabb)
	return results


func _effective_margin_top() -> float:
	var base: float = float(preview_margin_top)
	if top_node_margin and is_inside_tree():
		base += maxf(
			0.0, top_node_margin.global_position.y + top_node_margin.size.y - global_position.y
		)
	return base


func _effective_margin_bottom() -> float:
	var base: float = float(preview_margin_bottom)
	if bottom_node_margin and is_inside_tree():
		base += maxf(0.0, global_position.y + size.y - bottom_node_margin.global_position.y)
	return base


func _fit_camera_to_aabb(aabb: AABB, extra_margin: int = 0) -> void:
	if not is_inside_tree():
		return
	if size.x <= 0.0 or size.y <= 0.0:
		_pending_camera_fit = true
		return
	_pending_camera_fit = false
	var padding: float = _focus_padding()
	if padding > 0.0:
		aabb = aabb.grow(maxf(aabb.size.x, aabb.size.y) * padding * 0.5)
	var vp_w: float = size.x
	var vp_h: float = size.y
	var eff_top: float = _effective_margin_top()
	var eff_bottom: float = _effective_margin_bottom()
	var inner_h: float = maxf(1.0, vp_h - eff_top - eff_bottom - extra_margin * 2)
	var inner_w: float = maxf(
		1.0, vp_w - preview_margin_left - preview_margin_right - extra_margin * 2
	)
	var cam_size: float = maxf(aabb.size.y * vp_h / inner_h, aabb.size.x * vp_h / inner_w)
	cam_size = maxf(cam_size, MIN_CAMERA_SIZE_PART)
	_fitted_camera_size = cam_size
	_fitted_aabb_center_y = _focus_aabb_center_y(aabb)
	var center_y: float = _fitted_aabb_center_y + (eff_top - eff_bottom) * cam_size / (2.0 * vp_h)
	var aabb_bottom_w: float = (avatar.global_transform * Vector3(0.0, aabb.position.y, 0.0)).y
	var aabb_top_w: float = (
		(avatar.global_transform * Vector3(0.0, aabb.position.y + aabb.size.y, 0.0)).y
	)
	var pan_min: float = aabb_bottom_w + cam_size * (0.5 - eff_bottom / vp_h)
	var pan_max: float = aabb_top_w - cam_size * (0.5 - eff_top / vp_h)
	center_y = clampf(center_y, minf(pan_min, pan_max), maxf(pan_min, pan_max))
	_target_camera_center_y = center_y
	_target_camera_size = cam_size


func _make_aabb_box(aabb: AABB, color: Color) -> MeshInstance3D:
	var box_mi := MeshInstance3D.new()
	var im := ImmediateMesh.new()
	var center := aabb.get_center()
	var c: Array[Vector3] = []
	for i in 8:
		c.append(aabb.get_endpoint(i) - center)
	im.surface_begin(Mesh.PRIMITIVE_LINES)
	var edges: Array[int] = [0, 1, 2, 3, 4, 5, 6, 7, 0, 2, 1, 3, 4, 6, 5, 7, 0, 4, 1, 5, 2, 6, 3, 7]
	for i in range(0, edges.size(), 2):
		im.surface_add_vertex(c[edges[i]])
		im.surface_add_vertex(c[edges[i + 1]])
	im.surface_end()
	box_mi.mesh = im
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = Color(color.r, color.g, color.b, 1.0)
	mat.vertex_color_use_as_albedo = false
	box_mi.material_override = mat
	box_mi.position = center
	return box_mi


func _update_aabb_debug_box(aabbs: Dictionary) -> void:
	for n in _aabb_debug_nodes:
		if is_instance_valid(n):
			n.queue_free()
	_aabb_debug_nodes.clear()

	if not show_aabb_debug or aabbs.is_empty():
		return

	var box_colors: Dictionary = {
		"overall": Color(0, 1, 0, 0.15),
		"head": Color(0, 1, 1, 0.20),
		"torso": Color(1, 1, 0, 0.20),
		"legs": Color(1, 0.5, 0, 0.20),
		"feet": Color(0, 0.5, 1, 0.20),
		"hands": Color(1, 0, 1, 0.20),
	}
	for key in box_colors:
		if key not in aabbs:
			continue
		var aabb: AABB = aabbs[key]
		if aabb.size == Vector3.ZERO:
			continue
		var box := _make_aabb_box(aabb, box_colors[key])
		avatar.add_child(box)
		_aabb_debug_nodes.append(box)


func async_get_viewport_image(face: bool, dest_size: Vector2i, ortho_size: float = 2.5) -> Image:
	avatar.emote_controller.freeze_on_idle()
	avatar.rotation.y = 0.0
	const PROFILE_BODY_CAMERA_POSITION = Vector3(0, 1.25, -3.5)
	const PROFILE_HEAD_CAMERA_POSITION = Vector3(0, 1.70, -1.25)

	# Store original values to restore after capture
	var original_stretch = stretch
	var original_size = size
	var original_camera_center_y: float = camera_center.position.y
	var original_camera_position: Vector3 = camera_3d.position
	var original_camera_size: float = camera_3d.size
	var original_target_center_y: float = _target_camera_center_y
	var original_target_size: float = _target_camera_size

	_lerp_paused = true
	camera_center.position.y = 0.0
	camera_3d.position = PROFILE_HEAD_CAMERA_POSITION if face else PROFILE_BODY_CAMERA_POSITION
	camera_3d.size = ortho_size

	# Disable stretch to allow manual SubViewport sizing
	stretch = false
	set_size(dest_size)
	subviewport.set_size(dest_size)

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := subviewport.get_texture().get_image()

	# Restore original camera and viewport state
	camera_center.position.y = original_camera_center_y
	camera_3d.position = original_camera_position
	camera_3d.size = original_camera_size
	_target_camera_center_y = original_target_center_y
	_target_camera_size = original_target_size
	stretch = original_stretch
	set_size(original_size)
	_lerp_paused = false

	return img
