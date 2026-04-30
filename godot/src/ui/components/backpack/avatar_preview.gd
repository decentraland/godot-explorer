class_name AvatarPreview
extends SubViewportContainer


@export var hide_name: bool = false
@export var show_platform: bool = false
@export var can_move: bool = true
@export var custom_environment: Environment = null
@export var with_light: bool = true
@export var preview_margin_top: int = 0
@export var preview_margin_bottom: int = 0
@export var preview_margin_left: int = 0
@export var preview_margin_right: int = 0

var start_angle
var start_dragging_position
var start_camera_center_y: float = 0.0
var dirty_is_dragging
var _camera_focus: String = "overall"

var _cached_aabbs: Dictionary = {}
var _camera_tween: Tween = null

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

	avatar.avatar_loaded.connect(_on_avatar_loaded)

	if Global.standalone:
		Global.player_identity.set_default_profile()
		var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
		avatar.async_update_avatar_from_profile(profile)


func focus_camera_on(type):
	match type:
		Wearables.Categories.HAIR, Wearables.Categories.FACIAL_HAIR, Wearables.Categories.EYEWEAR, Wearables.Categories.TIARA, Wearables.Categories.FACIAL, Wearables.Categories.EYEBROWS, Wearables.Categories.MOUTH, Wearables.Categories.HAT, Wearables.Categories.EARRING, Wearables.Categories.MASK, Wearables.Categories.HELMET, Wearables.Categories.TOP_HEAD, Wearables.Categories.EYES:
			_camera_focus = "head"
		Wearables.Categories.UPPER_BODY, Wearables.Categories.HANDS_WEAR, Wearables.Categories.HANDS:
			_camera_focus = "torso"
		Wearables.Categories.LOWER_BODY:
			_camera_focus = "legs"
		Wearables.Categories.FEET:
			_camera_focus = "feet"
		_:
			_camera_focus = "overall"
	if _camera_focus in _cached_aabbs:
		_fit_camera_to_aabb(_cached_aabbs[_camera_focus])


func _input(event: InputEvent):
	if not can_move:
		return
	if get_parent_control() and event is InputEventMouseButton:
		if not get_parent_control().get_global_rect().has_point(event.position):
			return

	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dirty_is_dragging = true
				start_dragging_position = get_global_mouse_position()
				start_angle = avatar.rotation.y
				start_camera_center_y = camera_center.position.y
			else:
				dirty_is_dragging = false

		if not event.pressed:
			var dir: float = 0.0
			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				dir = -0.2
			elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				dir = 0.2

			if dir != 0.0:
				camera_3d.size = maxf(0.1, camera_3d.size + dir)

	if event is InputEventMouseMotion:
		if dirty_is_dragging:
			var diff = 0.005 * (get_global_mouse_position() - start_dragging_position)
			avatar.rotation.y = start_angle + diff.x
			camera_center.position.y = start_camera_center_y - diff.y * 10.0


func reset_avatar_rotation() -> void:
	avatar.rotation.y = 0.0


func enable_outline():
	if outline_system and avatar:
		outline_system.set_outlined_avatar(avatar)


func disable_outline():
	if outline_system:
		outline_system.set_outlined_avatar(null)


func _on_avatar_loaded():
	_cached_aabbs = _compute_avatar_aabbs()
	_update_aabb_debug_box(_cached_aabbs)
	if _camera_focus in _cached_aabbs:
		_fit_camera_to_aabb(_cached_aabbs[_camera_focus])


func _get_mesh_category(mesh_name: String) -> String:
	if "__hair" in mesh_name or "__facial_hair" in mesh_name \
			or "__eyewear" in mesh_name or "__earring" in mesh_name \
			or "__tiara" in mesh_name or "__helmet" in mesh_name \
			or "__mask" in mesh_name or "__top_head" in mesh_name \
			or "_head_basemesh" in mesh_name or "_mask_" in mesh_name:
		return "head"
	if "__upper_body" in mesh_name or "_ubody_basemesh" in mesh_name:
		return "torso"
	if "__lower_body" in mesh_name or "_lbody_basemesh" in mesh_name:
		return "legs"
	if "__feet" in mesh_name or "_feet_basemesh" in mesh_name:
		return "feet"
	return ""


func _compute_skinned_mesh_aabb(mi: MeshInstance3D, skeleton: Skeleton3D, avatar_xform_inv: Transform3D) -> AABB:
	var skin: Skin = mi.skin
	if skin == null:
		# Non-skinned: transform mesh AABB corners through node transforms
		var mesh_aabb: AABB = mi.mesh.get_aabb()
		var result := AABB()
		var first := true
		for i in 8:
			var local_pt: Vector3 = avatar_xform_inv * (mi.global_transform * mesh_aabb.get_endpoint(i))
			if first:
				result = AABB(local_pt, Vector3.ZERO)
				first = false
			else:
				result = result.expand(local_pt)
		return result

	var skin_count: int = skin.get_bind_count()
	var skeleton_global: Transform3D = skeleton.global_transform

	# Build world-space skin matrices: bone_world_pose * inverse_bind
	var skin_matrices: Array[Transform3D] = []
	skin_matrices.resize(skin_count)
	for i in skin_count:
		var bone_idx: int = skin.get_bind_bone(i)
		if bone_idx < 0:
			bone_idx = skeleton.find_bone(skin.get_bind_name(i))
		skin_matrices[i] = skeleton_global * skeleton.get_bone_global_pose(bone_idx) * skin.get_bind_pose(i)

	var result := AABB()
	var first := true

	for surf_idx in mi.mesh.get_surface_count():
		var arrays: Array = mi.mesh.surface_get_arrays(surf_idx)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var bones_raw = arrays[Mesh.ARRAY_BONES]
		var weights_raw: PackedFloat32Array = arrays[Mesh.ARRAY_WEIGHTS]
		if bones_raw == null or weights_raw == null or vertices.is_empty():
			continue

		var bones: PackedInt32Array
		if bones_raw is PackedFloat32Array:
			bones = PackedInt32Array(bones_raw)
		else:
			bones = bones_raw as PackedInt32Array

		var influences: int = bones.size() / vertices.size()

		for v_idx in vertices.size():
			var skinned_pos := Vector3.ZERO
			for b in influences:
				var bind_idx: int = bones[v_idx * influences + b]
				var weight: float = weights_raw[v_idx * influences + b]
				if weight > 0.0:
					skinned_pos += weight * (skin_matrices[bind_idx] * vertices[v_idx])
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
	var firsts: Dictionary = {"overall": true, "head": true, "torso": true, "legs": true, "feet": true}
	for child in skeleton.get_children():
		var mi := child as MeshInstance3D
		if mi == null or mi.mesh == null:
			continue
		var is_basemesh: bool = "_basemesh" in mi.name
		if not mi.visible and not is_basemesh:
			continue
		var mesh_aabb: AABB = _compute_skinned_mesh_aabb(mi, skeleton, avatar_xform_inv)
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
	return results


func _fit_camera_to_aabb(aabb: AABB) -> void:
	if not is_inside_tree():
		return
	if _camera_tween:
		_camera_tween.kill()
	var vp_w: float = size.x
	var vp_h: float = size.y
	var inner_h: float = maxf(1.0, vp_h - preview_margin_top - preview_margin_bottom)
	var inner_w: float = maxf(1.0, vp_w - preview_margin_left - preview_margin_right)
	var cam_size: float = maxf(aabb.size.y * vp_h / inner_h, aabb.size.x * vp_h / inner_w)
	var center_y: float = aabb.get_center().y + float(preview_margin_top - preview_margin_bottom) * cam_size / (2.0 * vp_h)
	_camera_tween = create_tween().set_parallel()
	_camera_tween.tween_property(camera_center, "position:y", center_y, 0.5)
	_camera_tween.tween_property(camera_3d, "size", cam_size, 0.5)


func _make_aabb_box(aabb: AABB, color: Color) -> MeshInstance3D:
	var box_mi := MeshInstance3D.new()
	var im := ImmediateMesh.new()
	var center := aabb.get_center()
	var c: Array[Vector3] = []
	for i in 8:
		c.append(aabb.get_endpoint(i) - center)
	im.surface_begin(Mesh.PRIMITIVE_LINES)
	var edges: Array[int] = [0,1, 2,3, 4,5, 6,7, 0,2, 1,3, 4,6, 5,7, 0,4, 1,5, 2,6, 3,7]
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

	if aabbs.is_empty():
		return

	var box_colors: Dictionary = {
		"overall": Color(0, 1, 0, 0.15),
		"head":    Color(0, 1, 1, 0.20),
		"torso":   Color(1, 1, 0, 0.20),
		"legs":    Color(1, 0.5, 0, 0.20),
		"feet":    Color(0, 0.5, 1, 0.20),
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
	camera_3d.position = PROFILE_HEAD_CAMERA_POSITION if face else PROFILE_BODY_CAMERA_POSITION
	camera_3d.size = ortho_size

	# Store original values to restore after capture
	var original_stretch = stretch
	var original_size = size

	# Disable stretch to allow manual SubViewport sizing
	stretch = false
	set_size(dest_size)
	subviewport.set_size(dest_size)

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := subviewport.get_texture().get_image()

	# Restore original stretch and size
	stretch = original_stretch
	set_size(original_size)

	return img
