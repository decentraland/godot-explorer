class_name OutlineSystem
extends Node3D

const OUTLINE_LAYER = 20  # Using layer 20 for outlined objects (bit 19, value 524288)

var main_camera: Camera3D = null
var current_outlined_avatar: Node3D = null

@onready var sub_viewport: SubViewport = $SubViewport
@onready var depth_camera: Camera3D = $SubViewport/DepthCamera
@onready var outline_quad: MeshInstance3D = $OutlineQuad


func setup(camera: Camera3D):
	main_camera = camera


func _process(_delta):
	if not main_camera:
		return

	# Match viewport size to window size
	var window_size = get_viewport().get_visible_rect().size
	if sub_viewport.size != Vector2i(window_size):
		sub_viewport.size = Vector2i(window_size)

	# Sync depth camera with main camera
	depth_camera.fov = main_camera.fov
	depth_camera.near = main_camera.near
	depth_camera.far = main_camera.far


func set_outlined_avatar(avatar: Node3D):
	# Clear previous outline
	if current_outlined_avatar:
		_set_avatar_layers(current_outlined_avatar, false)

	current_outlined_avatar = avatar

	# Set new outline
	if avatar:
		_set_avatar_layers(avatar, true)


func _set_avatar_layers(avatar: Node3D, add_outline: bool):
	# Find the Skeleton3D node in the avatar
	var skeleton = avatar.find_child("Skeleton3D", true, false)
	if not skeleton:
		return

	# Set layers for all MeshInstance3D children
	for child in skeleton.get_children():
		if child is MeshInstance3D:
			if add_outline:
				# Add layer 20 to the mesh
				child.layers |= (1 << (OUTLINE_LAYER - 1))
			else:
				# Remove layer 20 from the mesh
				child.layers &= ~(1 << (OUTLINE_LAYER - 1))


func _exit_tree():
	if current_outlined_avatar:
		_set_avatar_layers(current_outlined_avatar, false)
