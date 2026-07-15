class_name AvatarPreview
extends SubViewportContainer

const MIN_CAMERA_SIZE_OVERALL = 2.0
const MIN_CAMERA_SIZE_PART = 0.8
const MAX_CAMERA_SIZE = 5.0
const CAMERA_PAN_SMOOTH = 2.0
const CAMERA_ZOOM_SMOOTH = 2.0
const AVATAR_ROTATION_SMOOTH = 15.0

@export var hide_name: bool = false
@export var show_platform: bool = false
## Enables rotation and pan/zoom interactions.
@export var can_move: bool = true
## When can_move is true, gates wheel zoom, pinch zoom, pinch vertical-pan, and
## per-category auto-focus (focus_camera_on stays on "overall" when false).
## Rotation (mouse drag, single-touch drag) is always enabled.
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
@export var fit_avatar: bool = true
## When true, grow this SubViewportContainer upward so its top edge sits at
## the screen's top (global_position.y = 0) regardless of where the parent
## layout placed it. The SubViewport (stretch=true) follows, so the avatar
## renders into the taller area and its head reaches the top of the screen.
## Use it when sibling UI sits above the preview (e.g., the lobby's "Create
## your avatar" label) — the avatar shows behind it instead of being
## stacked below.
@export var snap_top_to_viewport: bool = false:
	set(value):
		if value == snap_top_to_viewport:
			return
		snap_top_to_viewport = value
		if is_inside_tree():
			_refresh_snap_top_state()

var start_angle
var start_dragging_position
var start_camera_center_y: float = 0.0
var dirty_is_dragging
var _camera_focus: String = "overall"

var _cached_aabbs: Dictionary = {}
var _user_has_panned: bool = false
var _pending_camera_fit: bool = false
# Tracks a deferred _fit_to_overall (the initial fully-visible fit used when
# fit_avatar is true). Set when size is briefly 0 or margins eat the viewport
# during the first call after avatar load; cleared once the fit commits.
# _on_resized re-runs _fit_to_overall while this is true so the initial fit
# isn't lost — without it, the camera stays at the scene-file default and the
# avatar reads as tiny (the profile viewer hides h_box_container_data before
# awaiting avatar load, so size=0 when _fit_to_overall fires).
var _pending_fit_overall: bool = false
var _fitted_camera_size: float = MAX_CAMERA_SIZE
var _fitted_aabb_center_y: float = 0.0
var _last_fit_stable_aabb: AABB = AABB()
var _last_fit_extra_margin: int = -1
var _target_camera_center_y: float = 0.0
var _target_camera_size: float = MAX_CAMERA_SIZE
var _target_avatar_rotation_y: float = 0.0
var _lerp_paused: bool = false
var _touch_points: Dictionary = {}
var _pinch_start_distance: float = 0.0
var _pinch_start_camera_size: float = 0.0
var _pinch_start_midpoint: Vector2 = Vector2.ZERO
var _pinch_start_center_y: float = 0.0

# State for snap_top_to_viewport. `_snap_top_connected` tracks whether our
# item_rect_changed handler is wired up so we toggle it on/off cleanly when
# the flag flips. `_last_snapped_parent_y` is the parent's global_position.y
# we last computed offset_top against — we use it to break the
# offset_top → item_rect_changed → recompute cycle.
var _snap_top_connected: bool = false
var _last_snapped_parent_y: float = NAN
var _baseline_offset_top: float = 0.0

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

	avatar.avatar_loaded.connect(async_on_avatar_loaded)
	resized.connect(_on_resized)
	if top_node_margin:
		top_node_margin.resized.connect(_on_resized)
	if bottom_node_margin:
		bottom_node_margin.resized.connect(_on_resized)

	# Remember the export-set offset_top so we can restore it if the flag
	# later flips back to false at runtime.
	_baseline_offset_top = offset_top
	if snap_top_to_viewport:
		_refresh_snap_top_state()

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
		var mid_delta_y: float = _get_touch_midpoint().y - _pinch_start_midpoint.y
		var pan: float = mid_delta_y * _target_camera_size / size.y
		var limits: Vector2 = _pan_limits()
		_target_camera_center_y = clampf(_pinch_start_center_y + pan, limits.x, limits.y)
		_user_has_panned = true
	camera_center.position.y = lerpf(
		camera_center.position.y, _target_camera_center_y, 1.0 - exp(-CAMERA_PAN_SMOOTH * delta)
	)
	camera_3d.size = lerpf(
		camera_3d.size, _target_camera_size, 1.0 - exp(-CAMERA_ZOOM_SMOOTH * delta)
	)
	avatar.rotation.y = lerpf(
		avatar.rotation.y, _target_avatar_rotation_y, 1.0 - exp(-AVATAR_ROTATION_SMOOTH * delta)
	)


func focus_camera_on(type, instant: bool = false):
	if not can_drag:
		_camera_focus = "overall"
	else:
		match type:
			Wearables.Categories.FACIAL_HAIR:
				_camera_focus = "head_base_facial"
			Wearables.Categories.EYES, Wearables.Categories.EYEBROWS, Wearables.Categories.MOUTH:
				_camera_focus = "head_base"
			Wearables.Categories.HAIR, Wearables.Categories.EYEWEAR, Wearables.Categories.TIARA, Wearables.Categories.FACIAL, Wearables.Categories.HAT, Wearables.Categories.EARRING, Wearables.Categories.MASK, Wearables.Categories.HELMET, Wearables.Categories.TOP_HEAD, Wearables.Categories.ALL_EXTRAS:
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
	var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
	if aabb_key not in _cached_aabbs:
		return
	var new_margin: int = _focus_extra_margin()
	if (
		not instant
		and _stable_aabb() == _last_fit_stable_aabb
		and new_margin == _last_fit_extra_margin
	):
		_update_fit_limits(_cached_aabbs[aabb_key], new_margin)
		return
	_user_has_panned = false
	_fit_camera_to_aabb(_cached_aabbs[aabb_key], new_margin, instant)


func _min_camera_size() -> float:
	return MIN_CAMERA_SIZE_PART


func _focus_padding() -> float:
	match _camera_focus:
		"hands":
			return 0.3
		"torso":
			return 0.2
		"head", "head_base", "head_base_facial":
			return 0.1
		_:
			return 0.0


func _focus_aabb_center_y(aabb: AABB) -> float:
	if _camera_focus in ["head", "head_base", "head_base_facial"]:
		return aabb.position.y + aabb.size.y * 0.3
	return aabb.get_center().y


func _focus_extra_margin() -> int:
	match _camera_focus:
		"hands":
			return 80
		"feet", "head", "head_base", "head_base_facial", "torso":
			return 40
		_:
			return 0


func _input(event: InputEvent):
	if not can_move:
		return

	var iinner: Rect2 = _inner_rect()

	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				if not iinner.has_point(event.position):
					return
				dirty_is_dragging = true
				start_dragging_position = get_global_mouse_position()
				start_angle = _target_avatar_rotation_y
				start_camera_center_y = _target_camera_center_y
			else:
				dirty_is_dragging = false

		if not event.pressed and can_drag and iinner.has_point(event.position):
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
			if not iinner.has_point(event.position):
				return
			_touch_points[event.index] = event.position
			if _touch_points.size() == 2 and can_drag:
				dirty_is_dragging = false
				_pinch_start_distance = _get_touch_distance()
				_pinch_start_camera_size = _target_camera_size
				_pinch_start_midpoint = _get_touch_midpoint()
				_pinch_start_center_y = _target_camera_center_y
		else:
			_touch_points.erase(event.index)
			_pinch_start_distance = 0.0
			if _touch_points.size() == 1:
				dirty_is_dragging = true
				start_dragging_position = _touch_points.values()[0]
				start_angle = _target_avatar_rotation_y
				start_camera_center_y = _target_camera_center_y

	if event is InputEventScreenDrag:
		_touch_points[event.index] = event.position

	if event is InputEventMouseMotion:
		if dirty_is_dragging and _touch_points.size() < 2:
			_apply_drag(get_global_mouse_position())


func _pan_limits() -> Vector2:
	var bounds_aabb: AABB = _cached_aabbs.get(
		"overall", _cached_aabbs.get("body_base", AABB(Vector3.ZERO, Vector3.ONE * 2.0))
	)
	var av_xform: Transform3D = avatar.global_transform
	var aabb_bottom: float = (av_xform * Vector3(0.0, bounds_aabb.position.y, 0.0)).y
	var aabb_top: float = (
		(av_xform * Vector3(0.0, bounds_aabb.position.y + bounds_aabb.size.y, 0.0)).y
	)
	var cam_size: float = _target_camera_size
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


func _get_touch_distance() -> float:
	var keys: Array = _touch_points.keys()
	return (_touch_points[keys[0]] as Vector2).distance_to(_touch_points[keys[1]])


func _get_touch_midpoint() -> Vector2:
	var keys: Array = _touch_points.keys()
	return ((_touch_points[keys[0]] as Vector2) + (_touch_points[keys[1]] as Vector2)) * 0.5


func reset_avatar_rotation() -> void:
	_target_avatar_rotation_y = 0.0


func _refresh_snap_top_state() -> void:
	if snap_top_to_viewport:
		if not _snap_top_connected:
			item_rect_changed.connect(_apply_snap_top_to_viewport)
			_snap_top_connected = true
		_apply_snap_top_to_viewport()
	else:
		if _snap_top_connected:
			item_rect_changed.disconnect(_apply_snap_top_to_viewport)
			_snap_top_connected = false
		_last_snapped_parent_y = NAN
		# Restore the original offset_top from the scene file.
		if not is_equal_approx(offset_top, _baseline_offset_top):
			offset_top = _baseline_offset_top


func _apply_snap_top_to_viewport() -> void:
	if not snap_top_to_viewport or not is_inside_tree():
		return
	var parent_ctrl: Control = get_parent_control()
	if parent_ctrl == null:
		return
	var parent_y: float = parent_ctrl.global_position.y
	# Dedupe against the parent_y we last snapped against. Setting offset_top
	# re-fires item_rect_changed; without this guard the handler would
	# recurse (or at least burn frames re-doing identical work).
	if not is_nan(_last_snapped_parent_y) and is_equal_approx(parent_y, _last_snapped_parent_y):
		return
	_last_snapped_parent_y = parent_y
	# Shift this Control upward by the parent's distance from the screen
	# top. anchor_top stays at 0 so this just grows the rect (anchor_bottom=1
	# pins the bottom edge to wherever the parent layout placed it).
	var desired: float = _baseline_offset_top - parent_y
	if not is_equal_approx(offset_top, desired):
		offset_top = desired


func _on_resized() -> void:
	if size.x <= 0.0 or size.y <= 0.0:
		return
	if _pending_fit_overall and "overall" in _cached_aabbs:
		_fit_to_overall()
		return
	if _pending_camera_fit:
		var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
		if aabb_key in _cached_aabbs:
			_fit_camera_to_aabb(_cached_aabbs[aabb_key], _focus_extra_margin())


func enable_outline():
	if outline_system and avatar:
		outline_system.set_outlined_avatar(avatar)


func disable_outline():
	if outline_system:
		outline_system.set_outlined_avatar(null)


func async_on_avatar_loaded():
	if not is_inside_tree():
		return
	await get_tree().process_frame
	if not is_inside_tree():
		return
	var first_load: bool = _cached_aabbs.is_empty()
	_cached_aabbs = _compute_avatar_aabbs()
	_update_aabb_debug_box(_cached_aabbs)
	var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
	if aabb_key in _cached_aabbs:
		if first_load:
			if fit_avatar and _camera_focus == "overall":
				_fit_to_overall()
			else:
				_fit_camera_to_aabb.call_deferred(
					_cached_aabbs[aabb_key], _focus_extra_margin(), true
				)
		else:
			_update_fit_limits(_cached_aabbs[aabb_key], _focus_extra_margin())


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


func _symmetrize_aabb_x(aabb: AABB) -> AABB:
	var left_half: float = -aabb.position.x
	var right_half: float = aabb.position.x + aabb.size.x
	var max_half: float = maxf(left_half, right_half)
	return AABB(
		Vector3(-max_half, aabb.position.y, aabb.position.z),
		Vector3(max_half * 2.0, aabb.size.y, aabb.size.z)
	)


func _compute_avatar_aabbs() -> Dictionary:
	var skeleton: Skeleton3D = avatar.body_shape_skeleton_3d
	if skeleton == null:
		return {}
	var avatar_xform_inv: Transform3D = (avatar.global_transform as Transform3D).affine_inverse()
	var results: Dictionary = {}
	var firsts: Dictionary = {
		"overall": true,
		"head": true,
		"torso": true,
		"legs": true,
		"feet": true,
		"hands": true,
		"head_base": true,
		"head_base_facial": true,
		"body_base": true,
		"torso_base": true,
		"legs_base": true,
		"feet_base": true,
		"hands_base": true,
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
		if is_basemesh:
			if firsts["body_base"]:
				results["body_base"] = mesh_aabb
				firsts["body_base"] = false
			else:
				results["body_base"] = (results["body_base"] as AABB).merge(mesh_aabb)
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
			if firsts["hands_base"]:
				results["hands_base"] = mesh_aabb
				firsts["hands_base"] = false
			else:
				results["hands_base"] = (results["hands_base"] as AABB).merge(mesh_aabb)
		if "_ubody_basemesh" in mi.name:
			if firsts["torso_base"]:
				results["torso_base"] = mesh_aabb
				firsts["torso_base"] = false
			else:
				results["torso_base"] = (results["torso_base"] as AABB).merge(mesh_aabb)
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
			if firsts["feet_base"]:
				results["feet_base"] = knee_aabb
				firsts["feet_base"] = false
			else:
				results["feet_base"] = (results["feet_base"] as AABB).merge(knee_aabb)
			if firsts["legs_base"]:
				results["legs_base"] = mesh_aabb
				firsts["legs_base"] = false
			else:
				results["legs_base"] = (results["legs_base"] as AABB).merge(mesh_aabb)
		if "_feet_basemesh" in mi.name:
			if firsts["legs"]:
				results["legs"] = mesh_aabb
				firsts["legs"] = false
			else:
				results["legs"] = (results["legs"] as AABB).merge(mesh_aabb)
			if firsts["legs_base"]:
				results["legs_base"] = mesh_aabb
				firsts["legs_base"] = false
			else:
				results["legs_base"] = (results["legs_base"] as AABB).merge(mesh_aabb)
		if "_head_basemesh" in mi.name:
			if firsts["head_base"]:
				results["head_base"] = mesh_aabb
				firsts["head_base"] = false
			else:
				results["head_base"] = (results["head_base"] as AABB).merge(mesh_aabb)
			if firsts["head_base_facial"]:
				results["head_base_facial"] = mesh_aabb
				firsts["head_base_facial"] = false
			else:
				results["head_base_facial"] = (results["head_base_facial"] as AABB).merge(mesh_aabb)
		if "__facial_hair" in mi.name and mi.visible:
			if firsts["head_base_facial"]:
				results["head_base_facial"] = mesh_aabb
				firsts["head_base_facial"] = false
			else:
				results["head_base_facial"] = (results["head_base_facial"] as AABB).merge(mesh_aabb)
	for key in ["feet", "feet_base"]:
		if key in results:
			var a: AABB = results[key]
			results[key] = AABB(a.position + Vector3(0.0, -0.05, 0.0), a.size)
	for key in results:
		results[key] = _symmetrize_aabb_x(results[key])
	return results


func _inner_rect() -> Rect2:
	var r: Rect2 = get_global_rect()
	var mt: float = _effective_margin_top()
	var mb: float = _effective_margin_bottom()
	var size: Vector2 = r.size - Vector2(float(preview_margin_left + preview_margin_right), mt + mb)
	size.x = maxf(size.x, 0.0)
	size.y = maxf(size.y, 0.0)
	return Rect2(r.position + Vector2(float(preview_margin_left), mt), size)


## Replace the bottom_node_margin Control after _ready has connected signals.
func set_bottom_margin_node(node: Control) -> void:
	if bottom_node_margin == node:
		return
	if bottom_node_margin and bottom_node_margin.resized.is_connected(_on_resized):
		bottom_node_margin.resized.disconnect(_on_resized)
	bottom_node_margin = node
	if bottom_node_margin:
		bottom_node_margin.resized.connect(_on_resized)
	_pending_camera_fit = true
	if size.x > 0.0 and size.y > 0.0 and not _cached_aabbs.is_empty():
		var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
		if aabb_key in _cached_aabbs:
			_last_fit_stable_aabb = AABB()
			_last_fit_extra_margin = -1
			_fit_camera_to_aabb(_cached_aabbs[aabb_key], _focus_extra_margin())


## Replace the top_node_margin Control after _ready has connected signals.
## Hosts use this when their layout puts a different visible control above
## the preview (e.g., lobby's create-avatar flow uses its "Create your
## avatar" label instead of the backpack's hidden navbar).
func set_top_margin_node(node: Control) -> void:
	if top_node_margin == node:
		return
	if top_node_margin and top_node_margin.resized.is_connected(_on_resized):
		top_node_margin.resized.disconnect(_on_resized)
	top_node_margin = node
	if top_node_margin:
		top_node_margin.resized.connect(_on_resized)
	# Force a re-fit with the new reference; reset dedupe so it actually applies.
	_pending_camera_fit = true
	if size.x > 0.0 and size.y > 0.0 and not _cached_aabbs.is_empty():
		var aabb_key: String = _camera_focus if _camera_focus in _cached_aabbs else "overall"
		if aabb_key in _cached_aabbs:
			_last_fit_stable_aabb = AABB()
			_last_fit_extra_margin = -1
			_fit_camera_to_aabb(_cached_aabbs[aabb_key], _focus_extra_margin())


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


func _update_fit_limits(aabb: AABB, extra_margin: int = 0) -> void:
	if not is_inside_tree() or size.x <= 0.0 or size.y <= 0.0:
		return
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
	var limit_aabb: AABB = _cached_aabbs.get("body_base", aabb)
	var cam_size: float = maxf(
		limit_aabb.size.y * vp_h / inner_h, limit_aabb.size.x * vp_h / inner_w
	)
	_fitted_camera_size = maxf(cam_size, MIN_CAMERA_SIZE_PART)
	_fitted_aabb_center_y = _focus_aabb_center_y(aabb)


func _stable_aabb() -> AABB:
	var fallback: AABB = _cached_aabbs.get("overall", AABB(Vector3.ZERO, Vector3.ONE * 2.0))
	match _camera_focus:
		"head", "head_base", "head_base_facial":
			return _cached_aabbs.get("head_base", fallback)
		"torso":
			return _cached_aabbs.get("torso_base", fallback)
		"legs":
			return _cached_aabbs.get("legs_base", fallback)
		"feet":
			return _cached_aabbs.get("feet_base", fallback)
		"hands":
			return _cached_aabbs.get("hands_base", fallback)
	return _cached_aabbs.get("body_base", fallback)


func _fit_to_overall() -> void:
	if not is_inside_tree():
		return
	if size.x <= 0.0 or size.y <= 0.0:
		_pending_fit_overall = true
		return
	if "overall" not in _cached_aabbs:
		return
	var aabb: AABB = _cached_aabbs["overall"]
	var vp_h: float = size.y
	var vp_w: float = size.x
	var eff_top: float = _effective_margin_top()
	var eff_bottom: float = _effective_margin_bottom()
	var raw_inner_h: float = vp_h - eff_top - eff_bottom
	var raw_inner_w: float = vp_w - preview_margin_left - preview_margin_right
	# Parallel guard to _fit_camera_to_aabb: when margins claim almost the full
	# viewport, the parent layout pass hasn't settled yet — defer rather than
	# commit a wildly zoomed-out cam_size that makes the avatar appear tiny.
	if raw_inner_h < 50.0 or raw_inner_w < 50.0:
		_pending_fit_overall = true
		return
	_pending_fit_overall = false
	var inner_h: float = maxf(1.0, raw_inner_h)
	var inner_w: float = maxf(1.0, raw_inner_w)
	var cam_size: float = maxf(aabb.size.y * vp_h / inner_h, aabb.size.x * vp_h / inner_w)
	cam_size = maxf(cam_size, MIN_CAMERA_SIZE_OVERALL)
	var center_y: float = aabb.get_center().y + (eff_top - eff_bottom) * cam_size / (2.0 * vp_h)
	var av_xform: Transform3D = avatar.global_transform
	var bottom_w: float = (av_xform * Vector3(0.0, aabb.position.y, 0.0)).y
	var top_w: float = (av_xform * Vector3(0.0, aabb.position.y + aabb.size.y, 0.0)).y
	var pan_min: float = bottom_w + cam_size * (0.5 - eff_bottom / vp_h)
	var pan_max: float = top_w - cam_size * (0.5 - eff_top / vp_h)
	center_y = clampf(center_y, minf(pan_min, pan_max), maxf(pan_min, pan_max))
	_target_camera_center_y = center_y
	_target_camera_size = cam_size


func _fit_camera_to_aabb(aabb: AABB, extra_margin: int = 0, instant: bool = false) -> void:
	if not is_inside_tree():
		return
	if size.x <= 0.0 or size.y <= 0.0:
		_pending_camera_fit = true
		return
	var vp_h: float = size.y
	var vp_w: float = size.x
	var eff_top: float = _effective_margin_top()
	var eff_bottom: float = _effective_margin_bottom()
	var available_h: float = vp_h - eff_top - eff_bottom
	var raw_inner_w: float = vp_w - preview_margin_left - preview_margin_right - extra_margin * 2
	# Defer when the core available space is too small — layout pass hasn't settled yet.
	# We check available_h (without extra_margin) so that tight-but-valid layouts
	# (e.g. hide_navbar mode where the panel starts near the top) are not deferred
	# indefinitely just because the category padding exceeds the remaining gap.
	if available_h < 50.0 or raw_inner_w < 50.0:
		_pending_camera_fit = true
		return
	# Cap extra_margin so it always leaves a positive inner area, even in contexts
	# where the panel covers most of the viewport height.
	var capped_extra: int = mini(extra_margin, int(available_h * 0.5) - 1)
	var raw_inner_h: float = available_h - capped_extra * 2
	_pending_camera_fit = false
	_update_fit_limits(aabb, capped_extra)
	var stable: AABB = _stable_aabb()
	if stable == _last_fit_stable_aabb and capped_extra == _last_fit_extra_margin:
		return
	_last_fit_stable_aabb = stable
	_last_fit_extra_margin = capped_extra
	var inner_h: float = maxf(1.0, raw_inner_h)
	var inner_w: float = maxf(1.0, raw_inner_w)
	var cam_size: float = maxf(stable.size.y * vp_h / inner_h, stable.size.x * vp_h / inner_w)
	cam_size = clampf(cam_size, _min_camera_size(), _fitted_camera_size)
	var center_y: float = stable.get_center().y + (eff_top - eff_bottom) * cam_size / (2.0 * vp_h)
	var focus_bottom_w: float = (avatar.global_transform * Vector3(0.0, aabb.position.y, 0.0)).y
	var focus_top_w: float = (
		(avatar.global_transform * Vector3(0.0, aabb.position.y + aabb.size.y, 0.0)).y
	)
	var pan_min: float = focus_bottom_w + cam_size * (0.5 - eff_bottom / vp_h)
	var pan_max: float = focus_top_w - cam_size * (0.5 - eff_top / vp_h)
	center_y = clampf(center_y, minf(pan_min, pan_max), maxf(pan_min, pan_max))
	_target_camera_center_y = center_y
	_target_camera_size = cam_size
	# `instant` writes the camera directly (bypassing the _process lerp) so a
	# first-load fit doesn't animate in from a wrong frame. Never do this while a
	# capture owns the camera: async_get_viewport_image sets _lerp_paused and
	# drives a fixed top_level camera (position/rotation/ortho size). A deferred
	# first_load fit (async_on_avatar_loaded → call_deferred(..., instant=true))
	# can otherwise land mid-capture, clobber the fixed ortho size, and render the
	# avatar oversized — this is what broke the first avatar's snapshot.
	if instant and not _lerp_paused:
		camera_3d.size = cam_size
		camera_center.position.y = center_y


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
		"head_base": Color(1, 0, 0, 0.20),
		"head_base_facial": Color(1, 0.5, 0.5, 0.20),
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


func async_get_viewport_image(
	face: bool, dest_size: Vector2i, ortho_size: float = 2.5, ssaa: int = 1
) -> Image:
	avatar.emote_controller.freeze_on_idle()
	avatar.rotation.y = 0.0
	const PROFILE_BODY_CAMERA_POSITION = Vector3(0, 1.25, -3.5)
	const PROFILE_HEAD_CAMERA_POSITION = Vector3(0, 1.70, -1.25)
	const PROFILE_BODY_CAMERA_ROTATION = Vector3(-5.0, 180.0, 0.0)
	const PROFILE_HEAD_CAMERA_ROTATION = Vector3(0.0, 180.0, 0.0)

	# Store original values to restore after capture
	var original_stretch = stretch
	var original_size = size
	var original_camera_position: Vector3 = camera_3d.position
	var original_camera_rotation: Vector3 = camera_3d.rotation_degrees
	var original_camera_size: float = camera_3d.size
	var original_target_center_y: float = _target_camera_center_y
	var original_target_size: float = _target_camera_size

	_lerp_paused = true
	camera_3d.top_level = true
	camera_3d.position = PROFILE_HEAD_CAMERA_POSITION if face else PROFILE_BODY_CAMERA_POSITION
	camera_3d.rotation_degrees = (
		PROFILE_HEAD_CAMERA_ROTATION if face else PROFILE_BODY_CAMERA_ROTATION
	)
	camera_3d.size = ortho_size

	# Disable stretch to allow manual SubViewport sizing.
	# Render at ssaa * dest_size internally, then Lanczos-downsample to
	# dest_size for high-quality SSAA on top of the viewport's MSAA / FXAA
	# / scaling_3d_scale (the latter is hard-clamped to <=2.0 in
	# viewport.cpp, so this is the only way to push past 2x SSAA).
	var render_size := dest_size * maxi(1, ssaa)
	stretch = false
	set_size(render_size)
	subviewport.set_size(render_size)

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := subviewport.get_texture().get_image()
	if ssaa > 1:
		img.resize(dest_size.x, dest_size.y, Image.INTERPOLATE_LANCZOS)

	# Restore original camera and viewport state
	camera_3d.top_level = false
	camera_3d.position = original_camera_position
	camera_3d.rotation_degrees = original_camera_rotation
	camera_3d.size = original_camera_size
	_target_camera_center_y = original_target_center_y
	_target_camera_size = original_target_size
	stretch = original_stretch
	set_size(original_size)
	_lerp_paused = false

	return img
