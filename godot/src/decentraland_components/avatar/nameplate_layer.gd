class_name NameplateLayer
extends RefCounted

## Shared screen-space layer + runtime for all avatar nameplates (#2215). Avatars
## add their NicknameUI Control here instead of rendering it into a per-avatar
## SubViewport texture (which showed uninitialized-VRAM garbage on mobile). The
## Control is projected onto the head anchor each frame, distance-faded by the
## camera distance (what the camera sees), depth-occluded against world geometry by a
## throttled raycast, and depth-sorted.
## `layer = -1` draws above the 3D world but below the default-layer HUD.

# On-screen size and distance fade (full < FADE_START, fade to FADE_END), per
# nickname_quad.gd. SCALE is 15% larger than the prior 0.25.
const SCALE := 0.2875
# Fraction of the tag height kept ABOVE the head anchor (1.0 = bottom edge sits on the
# anchor). Below 1.0 nudges the whole tag down toward the head.
const ANCHOR_HEIGHT_FACTOR := 0.85
const FADE_START := 10.0
const FADE_END := 15.0
# Alpha units/sec for smooth occlusion fade in/out.
const FADE_SPEED := 6.0
# Occlusion needs to see (a) solid world geometry + avatar bodies, and (b) remote
# avatar ClickArea areas. We split into two raycasts because a single query cannot
# target both without also hitting the nodeless, pooled DCL scene-sensor Area3Ds on
# CL_PLAYER (layer 3, mask value 4) that sat in front of the third-person camera and
# hid every tag (#2321). Query A (bodies only) uses CL_PHYSICS|CL_PLAYER, so scene-sensor AREAs
# cannot collide. Query B (areas only) uses CL_AVATAR, so those phantoms are not in
# the mask. See #2637.
const CL_PHYSICS := 2
# CL_PLAYER = layer 3 (mask value 4), per project.godot — avatar TriggerDetectors +
# the local player's CharacterBody3D.
const CL_PLAYER := 4
# CL_CLICKABLE_AVATAR = layer 30 (mask value 2^29) — avatar ClickAreas.
const CL_AVATAR := 536870912
const BODY_MASK := CL_PHYSICS | CL_PLAYER
const AREA_MASK := CL_AVATAR
# Frames between occlusion raycasts per avatar (staggered) — not every frame.
const OCCLUSION_PERIOD := 6
# Small gap above the computed bounds top (clearance already covers head/hat
# volume — see Avatar.get_bounds_top_y).
const NAMETAG_MARGIN := 0.1
# Debug: set true at runtime (e.g. the scene-inspector `eval` command, non-production) to bypass
# the occlusion raycast entirely so tags fade by distance only — confirms whether a
# vanishing tag is an occlusion artifact. `NameplateLayer.debug_disable_occlusion = true`.
static var debug_disable_occlusion := false

static var _root: Control = null


## The Control to parent nameplates under (screen-space). Created on first use.
static func get_root() -> Control:
	if is_instance_valid(_root):
		return _root
	var layer := CanvasLayer.new()
	layer.name = "NameplateLayer"
	layer.layer = -1
	_root = Control.new()
	_root.name = "Root"
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(_root)
	var explorer := Global.get_explorer()
	if explorer != null:
		explorer.add_child(layer)
	else:
		Global.add_child(layer)
	return _root


## Move the avatar's NicknameUI out of its per-avatar SubViewport into the shared
## layer and drop the render target. nickname_quad stays as an invisible head
## anchor (still drives the SDK NAME_TAG attach point + the screen projection).
static func attach(avatar) -> void:
	var ui = avatar.nickname_ui
	if avatar.nickname_viewport != null:
		avatar.nickname_viewport.remove_child(ui)
		avatar.nickname_viewport.queue_free()
		avatar.nickname_viewport = null
	avatar.nickname_quad.texture = null
	avatar.nickname_quad.visible = false
	ui.set_anchors_preset(Control.PRESET_TOP_LEFT)
	ui.mouse_filter = Control.MOUSE_FILTER_IGNORE
	# Start fully transparent: update() drives alpha with move_toward(), so a tag can
	# only ever fade IN from invisible once the gate opens with real data. Without this
	# the scene-default content (e.g. the "NPC#" placeholder) would fade out from the
	# inherited modulate.a≈1.0 during the first frames after spawn.
	ui.modulate.a = 0.0
	ui.hide()
	get_root().add_child(ui)


## Free the reparented NicknameUI (it lives in the shared layer, not under the avatar).
static func detach(avatar) -> void:
	if is_instance_valid(avatar.nickname_ui):
		avatar.nickname_ui.queue_free()


## Per-frame: project the head anchor to screen, place/scale/sort the Control, and
## drive its alpha toward a target (smooth fade in/out). The target is 0 unless the
## tag is gated-visible, in front of the camera, within FADE_END, inside the viewport
## (frustum) and not depth-occluded — so anything off-screen/behind/occluded fades
## out and fades back in when it re-enters, instead of popping.
static func update(avatar) -> void:
	var ui = avatar.nickname_ui
	if ui == null:
		return
	var target_a := 0.0
	var cam = avatar.get_viewport().get_camera_3d()
	if cam != null and avatar._nametag_gate_visible:
		var anchor: Vector3 = _anchor(avatar)
		# Fade by the camera distance — what the camera actually sees.
		var dist: float = cam.global_position.distance_to(anchor)
		if dist <= FADE_END and not cam.is_position_behind(anchor):
			ui.size = ui.get_combined_minimum_size()
			ui.scale = Vector2(SCALE, SCALE)
			var screen_size: Vector2 = ui.size * SCALE
			var pos: Vector2 = (
				cam.unproject_position(anchor)
				- Vector2(screen_size.x * 0.5, screen_size.y * ANCHOR_HEIGHT_FACTOR)
			)
			ui.position = pos
			# Closer avatars draw on top.
			ui.z_index = clampi(-int(dist * 100.0), -4000, 4000)
			var view_rect := Rect2(Vector2.ZERO, avatar.get_viewport().get_visible_rect().size)
			var on_screen := view_rect.intersects(Rect2(pos, screen_size))
			if on_screen and not avatar._nameplate_occluded:
				target_a = clampf((FADE_END - dist) / (FADE_END - FADE_START), 0.0, 1.0)
	ui.modulate.a = move_toward(
		ui.modulate.a, target_a, avatar.get_process_delta_time() * FADE_SPEED
	)
	ui.visible = ui.modulate.a > 0.01


## Throttled occlusion raycast. MUST run from _physics_process — direct_space_state
## crashes when queried from _process (idle frame).
static func update_occlusion(avatar) -> void:
	if debug_disable_occlusion:
		avatar._nameplate_occluded = false
		return
	if not avatar._nametag_gate_visible:
		return
	if (Engine.get_physics_frames() + int(avatar.unique_id)) % OCCLUSION_PERIOD != 0:
		return
	var cam = avatar.get_viewport().get_camera_3d()
	if cam == null:
		return
	var anchor: Vector3 = _anchor(avatar)
	if cam.global_position.distance_to(anchor) > FADE_END:
		return
	avatar._nameplate_occluded = _occluded(avatar, cam.global_position, anchor)


## 3D point the nameplate floats at: head anchor horizontally (follows the
## avatar), top of the avatar+wearables bounds + margin vertically — instead of
## the fixed bone offset, so tall wearables don't overlap the tag.
static func _anchor(avatar) -> Vector3:
	var anchor: Vector3 = avatar.nickname_quad.global_transform.origin
	anchor.y = avatar.get_bounds_top_y() + NAMETAG_MARGIN
	return anchor


## True if something sits between camera and anchor. Uses two raycasts so remote avatar
## ClickArea areas occlude the tag while the nodeless, pooled DCL scene-sensor Area3Ds
## on CL_PLAYER (layer 3, mask value 4) cannot. See the BODY_MASK/AREA_MASK comment above.
static func _occluded(avatar, from: Vector3, to: Vector3) -> bool:
	var space = avatar.get_world_3d().direct_space_state
	if space == null:
		return false

	# Exclude this avatar's own colliders (so it never occludes its own tag) plus the
	# local player's colliders (in third person they sit right next to the camera, in
	# the path of every remote tag's ray — the #2321 failure mode).
	var exclude: Array[RID] = []
	if is_instance_valid(avatar.click_area):
		exclude.append(avatar.click_area.get_rid())
	if is_instance_valid(avatar.trigger_detector):
		exclude.append(avatar.trigger_detector.get_rid())
	exclude.append_array(_local_player_rids())

	# Query A: solid bodies — world geometry + TriggerDetector/player bodies. A hit on
	# a hidden avatar's collider (blocked / modifier-area hidden) doesn't count: the
	# invisible avatar must not hide the tags behind it.
	var body_query := PhysicsRayQueryParameters3D.create(from, to)
	body_query.collision_mask = BODY_MASK
	body_query.collide_with_areas = false
	body_query.exclude = exclude
	var body_hit: Dictionary = space.intersect_ray(body_query)
	if not body_hit.is_empty():
		var collider = body_hit.get("collider")
		if not (collider is Node3D) or collider.is_visible_in_tree():
			return true

	var area_query := PhysicsRayQueryParameters3D.create(from, to)
	area_query.collision_mask = AREA_MASK
	area_query.collide_with_areas = true
	area_query.collide_with_bodies = false
	area_query.exclude = exclude
	return not space.intersect_ray(area_query).is_empty()


## RIDs of the local player's CharacterBody3D and its avatar's TriggerDetector.
## Recomputed per (throttled) ray: cheap, and survives player/avatar node swaps.
static func _local_player_rids() -> Array[RID]:
	var explorer := Global.get_explorer()
	if explorer == null or not is_instance_valid(explorer.player):
		return []
	var rids: Array[RID] = []
	var player = explorer.player
	if player is CollisionObject3D:
		rids.append(player.get_rid())
	var player_avatar = player.get("avatar")
	if player_avatar != null and is_instance_valid(player_avatar.trigger_detector):
		rids.append(player_avatar.trigger_detector.get_rid())
	return rids
