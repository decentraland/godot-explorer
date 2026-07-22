class_name OutlineSystem
extends Node3D

const OUTLINE_LAYER = 20  # Using layer 20 for outlined objects (bit 19, value 524288)

const ProximityFade := preload("res://src/decentraland_components/proximity_fade.gd")

# The outlined meshes render in the SubViewport with their real materials, so
# the camera proximity fade (issue #1814) dithers them there too — leaving a
# checkerboard depth texture that the Sobel reads as edges EVERYWHERE (the
# rainbow flood artifact). Suppress the outline while the target's surface is
# inside the fade band: an outline around a dissolving object is pointless
# anyway. Extra margin so the Sobel never sees a dithered frame.
const OUTLINE_SUPPRESS_MARGIN := 0.1

var main_camera: Camera3D = null
var current_outlined_avatar: Node3D = null
var current_outlined_entity: Node3D = null

# Local-space merged AABB of the outlined entity, cached on target change so
# the per-frame suppression check is a cheap transform + box distance.
var _entity_local_aabb := AABB()
var _entity_has_aabb := false

# Cached so we only push to the shader on size changes.
var _last_pushed_viewport_size: Vector2i = Vector2i.ZERO

@onready var sub_viewport: SubViewport = $SubViewport
@onready var depth_camera: Camera3D = $SubViewport/DepthCamera
@onready var outline_quad: MeshInstance3D = $OutlineQuad


func setup(camera: Camera3D):
	main_camera = camera
	# Show the quads when running in game (they're hidden in editor by default)
	if outline_quad:
		outline_quad.visible = true
	if depth_camera and depth_camera.has_node("DepthQuad"):
		var depth_quad = depth_camera.get_node("DepthQuad")
		depth_quad.visible = true
	if outline_quad:
		outline_quad.visible = false
	if sub_viewport:
		sub_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED


func _process(_delta):
	if not main_camera:
		return

	# Match viewport size to window size
	var window_size = main_camera.get_viewport().get_visible_rect().size
	var window_size_i = Vector2i(window_size)
	if sub_viewport.size != window_size_i:
		sub_viewport.size = window_size_i

	# Push viewport size to the outline shader so it doesn't depend on
	# textureSize(extracted_texture, 0), which returns inconsistent values
	# in Godot 4.6 for ViewportTextures inside SubViewports with non-default
	# scaling_3d_scale (e.g. avatar_preview uses scaling_3d_scale=2.0).
	if outline_quad and window_size_i != _last_pushed_viewport_size:
		var mat := outline_quad.material_override as ShaderMaterial
		if mat:
			mat.set_shader_parameter("viewport_size", Vector2(window_size_i))
			_last_pushed_viewport_size = window_size_i

	# Sync depth camera with main camera (incl. ortho size for avatar preview)
	depth_camera.global_transform = main_camera.global_transform
	depth_camera.projection = main_camera.projection
	depth_camera.fov = main_camera.fov
	depth_camera.size = main_camera.size
	depth_camera.near = main_camera.near
	depth_camera.far = main_camera.far

	# Proximity-fade suppression: an outlined entity whose surface is inside the
	# fade band would flood the Sobel with dither-hole "edges". (Avatars don't
	# need this: their materials have no scene fade — only scene entities do.)
	var active := (
		(current_outlined_avatar != null or current_outlined_entity != null)
		and not _is_entity_outline_suppressed()
	)
	if outline_quad and outline_quad.visible != active:
		outline_quad.visible = active
	if sub_viewport:
		var mode := SubViewport.UPDATE_ALWAYS if active else SubViewport.UPDATE_DISABLED
		if sub_viewport.render_target_update_mode != mode:
			sub_viewport.render_target_update_mode = mode


# Merged AABB of every MeshInstance3D under root, in root's local space.
static func compute_local_aabb(root_node: Node3D) -> AABB:
	var result := AABB()
	var first := true
	var stack: Array[Node] = [root_node]
	var inverse_root := root_node.global_transform.affine_inverse()
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		if node is MeshInstance3D and node.mesh != null:
			var local: AABB = (inverse_root * node.global_transform) * node.get_aabb()
			if first:
				result = local
				first = false
			else:
				result = result.merge(local)
		stack.append_array(node.get_children())
	return result


# Distance from a point to the closest point on the AABB (0 when inside).
static func aabb_surface_distance(point: Vector3, aabb: AABB) -> float:
	var closest := point.clamp(aabb.position, aabb.end)
	return closest.distance_to(point)


func _is_entity_outline_suppressed() -> bool:
	if current_outlined_entity == null or not _entity_has_aabb:
		return false
	if not is_instance_valid(current_outlined_entity):
		return false
	var world_aabb: AABB = current_outlined_entity.global_transform * _entity_local_aabb
	var dist := aabb_surface_distance(main_camera.global_position, world_aabb)
	return dist < ProximityFade.FADE_START_DISTANCE + OUTLINE_SUPPRESS_MARGIN


func set_outlined_avatar(avatar: Node3D):
	# Clear previous outline
	if current_outlined_avatar:
		_set_avatar_layers(current_outlined_avatar, false)

	current_outlined_avatar = avatar

	# Set new outline
	if avatar:
		_set_avatar_layers(avatar, true)

	_update_effect_visibility()


func set_outlined_entity(entity: Node3D):
	if current_outlined_entity == entity:
		return

	# Clear previous outline (guard against a freed entity node)
	if is_instance_valid(current_outlined_entity):
		_set_layers_recursive(current_outlined_entity, false)

	current_outlined_entity = entity
	_entity_has_aabb = false

	# Set new outline
	if entity:
		_set_layers_recursive(entity, true)
		_entity_local_aabb = compute_local_aabb(entity)
		_entity_has_aabb = true

	_update_effect_visibility()


# The outline post-process is shared by the avatar and entity paths; keep it
# rendering while either has a target (the crosshair hits only one at a time).
func _update_effect_visibility():
	var active := current_outlined_avatar != null or current_outlined_entity != null
	if outline_quad:
		outline_quad.visible = active
	if sub_viewport:
		sub_viewport.render_target_update_mode = (
			SubViewport.UPDATE_ALWAYS if active else SubViewport.UPDATE_DISABLED
		)


func _set_avatar_layers(avatar: Node3D, add_outline: bool):
	# Find the Skeleton3D node in the avatar
	var skeleton = avatar.find_child("Skeleton3D", true, false)
	if not skeleton:
		return

	# Set layers for all MeshInstance3D children recursively
	_set_layers_recursive(skeleton, add_outline)


func _set_layers_recursive(node: Node, add_outline: bool):
	for child in node.get_children():
		if child is MeshInstance3D:
			if add_outline:
				# Add layer 20 to the mesh
				child.layers |= (1 << (OUTLINE_LAYER - 1))
			else:
				# Remove layer 20 from the mesh
				child.layers &= ~(1 << (OUTLINE_LAYER - 1))
		else:
			# Recursively check children
			_set_layers_recursive(child, add_outline)


func _exit_tree():
	if is_instance_valid(current_outlined_avatar):
		_set_avatar_layers(current_outlined_avatar, false)
	if is_instance_valid(current_outlined_entity):
		_set_layers_recursive(current_outlined_entity, false)
