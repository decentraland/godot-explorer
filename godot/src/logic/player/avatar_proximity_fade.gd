class_name AvatarProximityFade
extends Node

## Object-level (uniform) proximity fade for avatars (issue #1814). When the
## camera gets within arm's reach of an avatar's head — the local player
## pinned against a wall by the collision clamp, or any avatar walking into
## the camera's personal space — the whole avatar dissolves uniformly instead
## of the camera seeing inside-out head geometry (or a face filling the
## screen).
##
## This is deliberately NOT a per-fragment fade: a fragment-level dither would
## show a cross-section of the skull. Here the entire avatar fades as one
## object, driven by camera distance to the head.
##
## Mechanism: per-instance shader parameter `own_fade` + a dither discard in
## the avatar shaders (assets/avatar/dcl_toon*.gdshader, mask_material.gdshader).
## GeometryInstance3D.transparency is NOT used — it is ignored by the mobile
## renderer for opaque materials (verified empirically). Per-instance params
## keep the shared toon ShaderMaterials untouched, so every avatar fades
## independently even when sharing wearables.
##
## Attached to every avatar by avatar.gd (local player, remote avatars, NPCs).
## Only the explorer's world camera drives it — avatar previews (backpack,
## passport, impostor capture) use close-up cameras of their own.

## Distance from the head at which the avatar starts fading out.
const FADE_START_DISTANCE := 0.6
## Distance at which the avatar is fully transparent.
const FADE_GONE_DISTANCE := 0.3
## Head height above the avatar origin (matches the local player's Mount
## pivot at 1.71 closely enough for every avatar).
const HEAD_HEIGHT := 1.7

var _meshes: Array[MeshInstance3D] = []
var _last_fade := -1.0

@onready var _avatar: Node3D = get_parent()


func _ready() -> void:
	_collect_meshes()
	# Wearables/body shapes are (re)loaded dynamically — recollect on changes.
	if _avatar.has_signal("avatar_loaded"):
		_avatar.connect("avatar_loaded", _recollect_deferred)
	_avatar.child_entered_tree.connect(_recollect_deferred.unbind(1))


func _recollect_deferred() -> void:
	_collect_meshes.call_deferred()


func _collect_meshes() -> void:
	_meshes.clear()
	_walk(_avatar)


func _walk(node: Node) -> void:
	for child in node.get_children():
		if child is MeshInstance3D and _supports_own_fade(child):
			_meshes.push_back(child)
		_walk(child)


# Only meshes whose shaders declare the own_fade uniform (the dcl_toon / mask
# shaders) can be faded per-instance; setting the param on anything else would
# just spam errors (e.g. the glider prop's StandardMaterial3D).
func _supports_own_fade(mesh_instance: MeshInstance3D) -> bool:
	if _mat_supports_own_fade(mesh_instance.material_override):
		return true
	for i in range(mesh_instance.get_surface_override_material_count()):
		if _mat_supports_own_fade(mesh_instance.get_surface_override_material(i)):
			return true
	if mesh_instance.mesh != null:
		for i in range(mesh_instance.mesh.get_surface_count()):
			if _mat_supports_own_fade(mesh_instance.mesh.surface_get_material(i)):
				return true
	return false


func _mat_supports_own_fade(mat: Material) -> bool:
	return mat is ShaderMaterial and mat.shader != null and mat.shader.code.contains("own_fade")


func _process(_delta: float) -> void:
	# Only avatars in the world (root viewport) fade. Avatar previews
	# (backpack, passport) and the impostor capture live in SubViewports with
	# close-up cameras of their own — exempt.
	if get_viewport() != get_tree().root:
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return

	var head: Vector3 = _avatar.global_position + Vector3(0, HEAD_HEIGHT, 0)
	var dist := camera.global_position.distance_to(head)
	var fade := clampf(
		(FADE_START_DISTANCE - dist) / (FADE_START_DISTANCE - FADE_GONE_DISTANCE), 0.0, 1.0
	)
	if is_equal_approx(fade, _last_fade):
		return
	_last_fade = fade
	for mesh in _meshes:
		mesh.set_instance_shader_parameter(&"own_fade", fade)
