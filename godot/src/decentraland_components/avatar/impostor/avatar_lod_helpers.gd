class_name AvatarLODHelpers
extends RefCounted

# Per-state node toggles used by Avatar._on_lod_state_changed. Extracted from
# avatar.gd to keep that file under the gdlint max-file-lines cap.


static func set_meshes_visible(avatar, visible: bool) -> void:
	if not avatar._mesh_lod_visibility_captured:
		capture_mesh_visibility(avatar)
	if avatar.body_shape_skeleton_3d == null:
		return
	for child in avatar.body_shape_skeleton_3d.get_children():
		if child is MeshInstance3D:
			var orig: bool = child.get_meta("lod_visible", true)
			child.visible = visible and orig


static func capture_mesh_visibility(avatar) -> void:
	if avatar._mesh_lod_visibility_captured or avatar.body_shape_skeleton_3d == null:
		return
	for child in avatar.body_shape_skeleton_3d.get_children():
		if child is MeshInstance3D:
			child.set_meta("lod_visible", child.visible)
	avatar._mesh_lod_visibility_captured = true


static func set_animation_active(avatar, active: bool) -> void:
	if avatar.animation_tree != null:
		avatar.animation_tree.active = active


static func set_animation_speed(avatar, speed: float) -> void:
	if avatar.animation_player != null:
		avatar.animation_player.speed_scale = speed


static func set_animation_throttle(avatar, throttled: bool) -> void:
	if avatar.animation_tree == null:
		return
	avatar._anim_throttle_active = throttled
	avatar._anim_throttle_acc = 0.0
	avatar._anim_throttle_counter = 0
	if throttled:
		avatar.animation_tree.callback_mode_process = (
			AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
		)
	else:
		avatar.animation_tree.callback_mode_process = (
			AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_IDLE
		)


static func set_particles_visible(avatar, visible: bool) -> void:
	for node_name in ["GPUParticles3D_Move", "GPUParticles3D_Jump", "GPUParticles3D_Land"]:
		var node = avatar.get_node_or_null(node_name)
		if node == null:
			continue
		node.visible = visible
		if node is GPUParticles3D:
			node.emitting = visible
		node.process_mode = (Node.PROCESS_MODE_INHERIT if visible else Node.PROCESS_MODE_DISABLED)


static func set_click_active(avatar, active: bool) -> void:
	if avatar.click_area == null:
		return
	var collision_shape = avatar.click_area.get_node_or_null("CollisionShape3D")
	if collision_shape != null:
		collision_shape.disabled = not active
	if avatar.click_area is Area3D:
		avatar.click_area.monitoring = active
		avatar.click_area.monitorable = active


static func set_dither_alpha(avatar, value: float) -> void:
	if avatar.body_shape_skeleton_3d == null:
		return
	var transparency: float = clamp(1.0 - value, 0.0, 1.0)
	for child in avatar.body_shape_skeleton_3d.get_children():
		if child is MeshInstance3D:
			child.transparency = transparency
