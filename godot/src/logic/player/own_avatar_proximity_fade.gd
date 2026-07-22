class_name OwnAvatarProximityFade
extends Node

## Object-level (uniform) proximity fade for the LOCAL player's avatar
## (issue #1814). When the camera is forced into the avatar's head — e.g. the
## spring arm / collision clamp pulls it to its minimum distance against a
## wall — the whole avatar dissolves uniformly instead of the camera seeing
## inside-out head geometry.
##
## This is deliberately NOT the per-fragment scene fade (ProximityFade): a
## fragment-level dither would show a cross-section of the skull. Here the
## entire avatar fades as one object, driven by camera distance to the head
## pivot (Mount). Lives only under the local player's Avatar instance in
## player.tscn, so remote avatars are never affected.
##
## Mechanism: per-instance shader parameter `own_fade` + a dither discard in
## the avatar shaders (assets/avatar/dcl_toon*.gdshader, mask_material.gdshader).
## GeometryInstance3D.transparency is NOT used — it is ignored by the mobile
## renderer for opaque materials (verified empirically). Per-instance params
## keep the shared toon ShaderMaterials untouched, so other avatars wearing
## the same wearable are unaffected.

## Distance from the head pivot at which the avatar starts fading out.
const FADE_START_DISTANCE := 0.6
## Distance at which the avatar is fully transparent.
const FADE_GONE_DISTANCE := 0.3

var _meshes: Array[MeshInstance3D] = []
var _last_fade := -1.0

@onready var _avatar: Node3D = get_parent()
@onready var _mount: Node3D = get_node("../../Mount")


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
	if _mount == null:
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return

	var dist := camera.global_position.distance_to(_mount.global_position)
	var fade := clampf(
		(FADE_START_DISTANCE - dist) / (FADE_START_DISTANCE - FADE_GONE_DISTANCE), 0.0, 1.0
	)
	if is_equal_approx(fade, _last_fade):
		return
	_last_fade = fade
	for mesh in _meshes:
		mesh.set_instance_shader_parameter(&"own_fade", fade)
