class_name AvatarLODHelpers
extends RefCounted

# Per-state node toggles used by Avatar._on_lod_state_changed, plus the
# animation freeze/throttle policy (resolve_anim_drive / apply_screen_freeze /
# ensure_anim_active). Extracted from avatar.gd to keep that file under the
# gdlint max-file-lines cap and so the freeze/throttle invariant is unit-testable
# headless without a full Avatar scene (see test/avatar/test_avatar_anim_throttle.gd).

# Mirror of Avatar.LODState. Kept local so this helper and its unit test don't
# pull in the full Avatar class (avoids a cyclic class_name reference, since
# Avatar already depends on AvatarLODHelpers). Must stay in sync with avatar.gd;
# the test asserts the values match.
const LOD_FULL := 0
const LOD_MID := 1
const LOD_CROSSFADE := 2
const LOD_FAR := 3


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


# Single source of truth for how the AnimationTree should be driven given the
# avatar's real on-screen state (from VisibleOnScreenNotifier3D) and LOD state.
# Returns {active, throttle}:
#   off-screen        -> not active        (frozen; cull hides it, CPU saved)
#   on-screen FULL    -> active, no throttle (full-rate animation)
#   on-screen MID/XF  -> active, throttled  (~20fps via manual advance)
#   on-screen FAR     -> not active        (real mesh hidden, impostor drawn)
static func resolve_anim_drive(on_screen: bool, lod_state: int) -> Dictionary:
	if not on_screen:
		return {"active": false, "throttle": false}
	match lod_state:
		LOD_FULL:
			return {"active": true, "throttle": false}
		LOD_MID, LOD_CROSSFADE:
			return {"active": true, "throttle": true}
		_:
			return {"active": false, "throttle": false}


# Freeze the AnimationTree while the avatar is not being drawn, restore it when
# it comes back. Always routes the throttle through set_animation_throttle so the
# callback mode and _anim_throttle_active stay consistent: the freeze resets the
# callback to IDLE, so even if something re-activates the tree (see
# ensure_anim_active) it advances normally instead of being stuck in MANUAL with
# no advance — the frozen-while-drawn bug. The local player is never frozen.
static func apply_screen_freeze(avatar) -> void:
	if avatar.animation_tree == null or avatar.is_local_player:
		return
	if not avatar._on_screen:
		if not avatar._anim_frozen_off_screen:
			set_animation_active(avatar, false)
			set_animation_throttle(avatar, false)
			avatar._anim_freeze_start_ms = Time.get_ticks_msec()
			avatar._anim_frozen_off_screen = true
	elif avatar._anim_frozen_off_screen:
		avatar._anim_frozen_off_screen = false
		var elapsed_s: float = (Time.get_ticks_msec() - avatar._anim_freeze_start_ms) / 1000.0
		var drive: Dictionary = resolve_anim_drive(avatar._on_screen, avatar._lod_state)
		set_animation_active(avatar, drive.active)
		set_animation_throttle(avatar, drive.throttle)
		# Advance by the wall-clock time we were paused so emote phase matches what
		# it would have been — single recompute, no frame-by-frame catch-up.
		if avatar.animation_tree.active and elapsed_s > 0.0:
			avatar.animation_tree.advance(elapsed_s)


# Keep the tree active for drawn avatars, but never undo an off-screen freeze.
# Re-activating while frozen is what left avatars stuck instead of throttling.
static func ensure_anim_active(avatar) -> void:
	if avatar.animation_tree == null:
		return
	if not avatar._anim_frozen_off_screen and not avatar.animation_tree.active:
		avatar.animation_tree.active = true


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
