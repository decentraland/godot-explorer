class_name NameplateLayer
extends RefCounted

## Content tiers per #2938.
enum Tier { FULL, BADGE, NAME }

## Shared screen-space layer + runtime for all avatar nameplates (#2215). Avatars
## add their NicknameUI Control here instead of rendering it into a per-avatar
## SubViewport texture (which showed uninitialized-VRAM garbage on mobile). The
## Control is projected onto the head anchor each frame, distance-faded by the
## camera distance (what the camera sees), depth-occluded against world geometry by a
## throttled raycast, and depth-sorted.
## `layer = -1` draws above the 3D world but below the default-layer HUD.

# On-screen size and distance tiers (#2938): full content < TIER_FULL_END, badge
# dropped past TIER_BADGE_END, name-only (smaller, dimmer) past TIER_NAME_END,
# fade to FADE_END, hidden beyond. SCALE is 15% larger than the prior 0.25.
const SCALE := 0.2875
# Fraction of the tag height kept ABOVE the head anchor (1.0 = bottom edge sits on the
# anchor). Below 1.0 nudges the whole tag down toward the head.
const ANCHOR_HEIGHT_FACTOR := 0.85
const TIER_FULL_END := 8.0
const TIER_BADGE_END := 12.0
const TIER_NAME_END := 20.0
const FADE_END := 25.0
# Name-only tier: reduced size and max opacity.
const TIER_NAME_SCALE := 0.8
const TIER_NAME_ALPHA := 0.7
# Crowd cap: at most this many plates show FULL content; the rest degrade to the
# badge tier. Focused (crosshair) and nearest win.
const CROWD_MAX_FULL := 7
# Alpha units/sec for smooth occlusion fade in/out.
const FADE_SPEED := 6.0
# Occlusion needs to see (a) solid world geometry + avatar bodies, and (b) other
# avatars — the latter analytically (_blocked_by_avatar), because the ClickArea
# capsule (default r=0.5) is twice the visual body width. We raycast bodies only
# because a single query with areas would also hit the nodeless, pooled DCL
# scene-sensor Area3Ds on CL_PLAYER (layer 3, mask value 4) that sat in front of
# the third-person camera and hid every tag (#2321). See #2637.
const CL_PHYSICS := AvatarCollisionLayers.CL_PHYSICS
# CL_PLAYER = layer 3 (mask value 4), per project.godot — avatar TriggerDetectors +
# the local player's CharacterBody3D.
const CL_PLAYER := AvatarCollisionLayers.CL_PLAYER
const BODY_MASK := CL_PHYSICS | CL_PLAYER
# Physics steps between occlusion raycasts per avatar (staggered) — not every step.
const OCCLUSION_PERIOD := 6
# A plate with alpha 0 whose head is farther than this skips the anchor/projection work.
const FAR_SKIP_DISTANCE := FADE_END + 5.0
# Small gap above the computed bounds top (clearance already covers head/hat
# volume — see Avatar.get_bounds_top_y).
const NAMETAG_MARGIN := 0.1
# Debug: set true at runtime (e.g. the scene-inspector `eval` command, non-production) to bypass
# the occlusion raycast entirely so tags fade by distance only — confirms whether a
# vanishing tag is an occlusion artifact. `NameplateLayer.debug_disable_occlusion = true`.
static var debug_disable_occlusion := false

static var _root: Control = null
# ui instance_id -> world point the tag is drawn at (screen pos unprojected at
# anchor depth). Occlusion rays go HERE, not to the head anchor: the tag is
# visible iff the camera can see the spot where it is actually drawn.
static var _plate_ray_targets: Dictionary = {}
# ui instance_id -> {dist, focused} for visible plates (crowd ranking) and -> last
# applied content tier [tier, show_chat] (apply only on change).
static var _plate_dists: Dictionary = {}
static var _plate_tiers: Dictionary = {}
# Focused (crosshair-hovered) avatar, resolved once per frame —
# Global.get_selected_avatar() is an FFI call and update() runs per avatar.
static var _focused_frame := -1
static var _focused_avatar: Variant = null
# Occlusion is driven by one node (OcclusionDriver): attached avatars sit in OCCLUSION_PERIOD
# buckets and only plates update() found on screen are raycast. A plate entering the screen
# is tested on the next step and held at alpha 0 until then.
static var _occlusion_buckets: Array = []
static var _on_screen: Dictionary = {}
static var _occlusion_untested: Dictionary = {}
# Analytic avatar occluders, captured once per physics step (instance id, position, radius,
# top y) instead of rebuilding the avatar list for every raycast.
static var _occluders_frame: int = -1
static var _occluder_ids: PackedInt64Array = PackedInt64Array()
static var _occluder_positions: PackedVector3Array = PackedVector3Array()
static var _occluder_radii: PackedFloat32Array = PackedFloat32Array()
static var _occluder_tops: PackedFloat32Array = PackedFloat32Array()


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
	var driver := OcclusionDriver.new()
	driver.name = "OcclusionDriver"
	layer.add_child(driver)
	# Under Global, never under the explorer: each avatar owns its plate (attach()
	# reparents it here, detach() frees it on PREDELETE), so the layer has to outlive
	# every avatar. Parented under the explorer it was freed first on sign-out and left
	# live avatars holding freed plates.
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
	# The quad only anchors the plate now; its fade is dead work.
	avatar.nickname_quad.set_process(false)
	_add_to_occlusion_bucket(avatar)


## Free the reparented NicknameUI (it lives in the shared layer, not under the avatar).
static func detach(avatar) -> void:
	var avatar_id: int = avatar.get_instance_id()
	for bucket in _occlusion_buckets:
		bucket.erase(avatar)
	_on_screen.erase(avatar_id)
	_occlusion_untested.erase(avatar_id)
	if is_instance_valid(avatar.nickname_ui):
		_untrack_plate(avatar.nickname_ui)
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
	var on_screen := false
	var cam = avatar.get_viewport().get_camera_3d()
	if (
		ui.modulate.a == 0.0
		and cam != null
		and (
			cam.global_position.distance_to(avatar.nickname_quad.global_position)
			> FAR_SKIP_DISTANCE
		)
	):
		_untrack_plate(ui)
		_set_on_screen(avatar, false)
		ui.visible = false
		return
	if cam != null and avatar._nametag_gate_visible:
		var anchor: Vector3 = _anchor(avatar)
		# Fade by the camera distance — what the camera actually sees.
		var dist: float = cam.global_position.distance_to(anchor)
		if dist <= FADE_END and not cam.is_position_behind(anchor):
			var focused := _is_focused(avatar)
			var tier := _content_tier(ui, dist, focused)
			ui.size = ui.get_combined_minimum_size()
			var scale_f: float = SCALE * (TIER_NAME_SCALE if tier == Tier.NAME else 1.0)
			ui.scale = Vector2(scale_f, scale_f)
			var screen_size: Vector2 = ui.size * scale_f
			var pos: Vector2 = (
				cam.unproject_position(anchor)
				- Vector2(screen_size.x * 0.5, screen_size.y * ANCHOR_HEIGHT_FACTOR)
			)
			var view_rect := Rect2(Vector2.ZERO, avatar.get_viewport().get_visible_rect().size)
			on_screen = view_rect.intersects(Rect2(pos, screen_size))
			_set_on_screen(avatar, on_screen)
			var untested: bool = _occlusion_untested.has(avatar.get_instance_id())
			if on_screen and not avatar._nameplate_occluded and not untested:
				# Plates stay pinned to their anchor (no de-overlap shifting) —
				# design decision on #2938: in crowds the cap limits full tags
				# instead of moving them around.
				# Chat bubbles gate on distance only (#2938) — the crowd cap must not
				# hide the bubble of someone standing next to you.
				var show_chat := focused or dist <= TIER_FULL_END
				_apply_tier(ui, tier, show_chat)
				# Mutate the entry in place: a fresh Dictionary literal per plate per
				# frame showed up in allocation profiles.
				var id: int = ui.get_instance_id()
				var entry: Dictionary = _plate_dists.get(id, {})
				entry.dist = dist
				entry.focused = focused
				_plate_dists[id] = entry
				target_a = _tier_alpha(tier, dist, focused)
			else:
				_untrack_plate(ui)
			# An anchor above the top edge would clip the tag, so keep the drawn plate
			# inside the viewport.
			pos.y = maxf(pos.y, 0.0)
			pos.x = clampf(pos.x, 0.0, maxf(view_rect.size.x - screen_size.x, 0.0))
			ui.position = pos
			# Closer avatars draw on top.
			ui.z_index = clampi(-int(dist * 100.0), -4000, 4000)
			# Where the tag is actually drawn, as a
			# world point at anchor depth — the occlusion ray targets this.
			var draw_center := pos + screen_size * 0.5
			_plate_ray_targets[ui.get_instance_id()] = (
				cam.project_ray_origin(draw_center) + cam.project_ray_normal(draw_center) * dist
			)
		else:
			_untrack_plate(ui)
	else:
		_untrack_plate(ui)
	if not on_screen:
		_set_on_screen(avatar, false)
	# The plate follows the GPU-ready hold so it never floats over an avatar that isn't drawn yet.
	var fade = avatar.get_node_or_null(^"AvatarProximityFade")
	if fade != null:
		target_a *= 1.0 - fade.get_hold()
	ui.modulate.a = move_toward(
		ui.modulate.a, target_a, avatar.get_process_delta_time() * FADE_SPEED
	)
	ui.visible = ui.modulate.a > 0.01


## Crosshair-hovered avatar always gets the full tag (#2938).
static func _is_focused(avatar) -> bool:
	var frame := Engine.get_process_frames()
	if frame != _focused_frame:
		_focused_frame = frame
		_focused_avatar = Global.get_selected_avatar()
	return _focused_avatar == avatar


## Content tier for a plate at `dist`: distance bands, with the crowd cap
## degrading excess close plates one tier and focused always FULL.
static func _content_tier(ui: Control, dist: float, focused: bool) -> int:
	if focused:
		return Tier.FULL
	if dist > TIER_BADGE_END:
		return Tier.NAME
	if dist > TIER_FULL_END:
		return Tier.BADGE
	if _crowd_allows_full(ui.get_instance_id(), dist):
		return Tier.FULL
	return Tier.BADGE


## Alpha target per tier: full until TIER_NAME_END, fade to FADE_END; name-only
## tier is additionally dimmed. Focused ignores the fade.
static func _tier_alpha(tier: int, dist: float, focused: bool) -> float:
	if focused:
		return 1.0
	var fade := clampf((FADE_END - dist) / (FADE_END - TIER_NAME_END), 0.0, 1.0)
	if tier == Tier.NAME:
		return minf(TIER_NAME_ALPHA, fade)
	return fade


## True if this plate is within the CROWD_MAX_FULL closest (focused plates take
## priority; ties break by instance id so equal-distance plates don't all lose).
## Plates already FULL get one extra slot of stickiness so borderline crowds
## don't flicker tiers as people walk. Ranks against last-frame distances — a
## 1-frame lag in a crowd swap is invisible.
static func _crowd_allows_full(ui_id: int, dist: float) -> bool:
	var better := 0
	for other_id in _plate_dists:
		if other_id == ui_id:
			continue
		var other: Dictionary = _plate_dists[other_id]
		# Only close or focused plates compete for full-tier slots.
		if not other.focused and other.dist > TIER_FULL_END:
			continue
		if other.focused or other.dist < dist or (other.dist == dist and other_id < ui_id):
			better += 1
	var cached = _plate_tiers.get(ui_id)
	var was_full: bool = cached != null and cached[0] == Tier.FULL
	return better < CROWD_MAX_FULL + (1 if was_full else 0)


## Show/hide the NicknameUI rows for the tier: hash `#xxxx` only in FULL, badge
## (claimed checkmark + mic) dropped in NAME; the chat bubble follows distance
## (`show_chat`), not the capped tier. Cached per plate — the setters in
## nickname_ui.gd (mic/name_claimed) re-apply on their own writes.
static func _apply_tier(ui: Control, tier: int, show_chat: bool) -> void:
	var id := ui.get_instance_id()
	var cached = _plate_tiers.get(id)
	if cached != null and cached[0] == tier and cached[1] == show_chat:
		return
	_plate_tiers[id] = [tier, show_chat]
	_set_tier_nodes(ui, tier, show_chat)


## Re-apply the cached tier after a nickname_ui setter (mic, name_claimed) wrote
## row visibility directly. No-op for plates without an applied tier.
static func reassert_tier(ui: Control) -> void:
	var cached = _plate_tiers.get(ui.get_instance_id())
	if cached != null:
		_set_tier_nodes(ui, cached[0], cached[1])


static func _set_tier_nodes(ui: Control, tier: int, show_chat: bool) -> void:
	ui.hash_container.visible = tier == Tier.FULL and not ui.name_claimed
	ui.message_clip.visible = show_chat
	ui.checkmark_container.visible = tier != Tier.NAME and ui.name_claimed
	ui.mic_enabled_icon.visible = tier != Tier.NAME and ui.mic_enabled


## Occlusion raycast for one plate. MUST run from _physics_process — direct_space_state
## crashes when queried from _process (idle frame). Throttling is the driver's job.
static func update_occlusion(avatar) -> void:
	if debug_disable_occlusion:
		avatar._nameplate_occluded = false
		return
	if not avatar._nametag_gate_visible:
		return
	var cam = avatar.get_viewport().get_camera_3d()
	if cam == null:
		return
	var anchor: Vector3 = _anchor(avatar)
	if cam.global_position.distance_to(anchor) > FADE_END:
		return
	# Occlusion is evaluated against the tag's DRAWN position, falling
	# back to the head anchor before the first placement.
	var target: Vector3 = anchor
	if avatar.nickname_ui != null:
		target = _plate_ray_targets.get(avatar.nickname_ui.get_instance_id(), anchor)
	avatar._nameplate_occluded = _occluded(avatar, cam.global_position, target)


## One physics step of the central driver: the current bucket's on-screen plates plus the
## plates that just came on screen.
static func _occlusion_step() -> void:
	if _occlusion_buckets.is_empty():
		return
	if debug_disable_occlusion:
		_occlusion_untested.clear()
		for bucket in _occlusion_buckets:
			for avatar in bucket:
				avatar._nameplate_occluded = false
		return
	var due: Array = _occlusion_untested.values()
	_occlusion_untested.clear()
	for avatar in _occlusion_buckets[Engine.get_physics_frames() % OCCLUSION_PERIOD]:
		if _on_screen.has(avatar.get_instance_id()) and not due.has(avatar):
			due.append(avatar)
	for avatar in due:
		if is_instance_valid(avatar) and avatar.is_inside_tree():
			update_occlusion(avatar)


static func _add_to_occlusion_bucket(avatar) -> void:
	if _occlusion_buckets.is_empty():
		for i in OCCLUSION_PERIOD:
			_occlusion_buckets.append([])
	var smallest: Array = _occlusion_buckets[0]
	for bucket in _occlusion_buckets:
		if bucket.size() < smallest.size():
			smallest = bucket
	smallest.append(avatar)


static func _set_on_screen(avatar, on_screen: bool) -> void:
	var avatar_id: int = avatar.get_instance_id()
	if on_screen == _on_screen.has(avatar_id):
		return
	if on_screen:
		_on_screen[avatar_id] = avatar
		_occlusion_untested[avatar_id] = avatar
	else:
		_on_screen.erase(avatar_id)
		_occlusion_untested.erase(avatar_id)


## Drop a plate from tracking (hidden/occluded/off-screen tags).
static func _untrack_plate(ui: Control) -> void:
	var id := ui.get_instance_id()
	_plate_ray_targets.erase(id)
	_plate_dists.erase(id)
	_plate_tiers.erase(id)


## 3D point the nameplate floats at: head anchor horizontally (follows the
## avatar), top of the avatar+wearables bounds + margin vertically — instead of
## the fixed bone offset, so tall wearables don't overlap the tag.
static func _anchor(avatar) -> Vector3:
	var anchor: Vector3 = avatar.nickname_quad.global_transform.origin
	anchor.y = avatar.get_bounds_top_y() + NAMETAG_MARGIN
	return anchor


## True if something sits between camera and anchor: a world-geometry/avatar-body
## raycast, then the analytic avatar-cylinder check. See the constants comment above.
static func _occluded(avatar, from: Vector3, to: Vector3) -> bool:
	var space = avatar.get_world_3d().direct_space_state
	if space == null:
		return false

	# Exclude this avatar's own colliders (so it never occludes its own tag). The
	# local player's body is deliberately NOT excluded from other avatars' rays:
	# your own avatar legitimately blocks tags behind it in third person.
	var exclude: Array[RID] = []
	if is_instance_valid(avatar.click_area):
		exclude.append(avatar.click_area.get_rid())
	if is_instance_valid(avatar.trigger_detector):
		exclude.append(avatar.trigger_detector.get_rid())
	if avatar.is_local_player:
		var parent = avatar.get_parent()
		if parent is CollisionObject3D:
			exclude.append(parent.get_rid())

	# Query A: solid bodies — world geometry + TriggerDetector/player bodies. A hit on
	# a hidden avatar's collider (blocked / modifier-area hidden) doesn't count: the
	# invisible avatar must not hide the tags behind it.
	var body_query := PhysicsRayQueryParameters3D.create(from, to)
	body_query.collision_mask = BODY_MASK
	body_query.collide_with_areas = false
	body_query.exclude = exclude
	# A hit on a hidden avatar's collider (blocked / modifier-area hidden) doesn't
	# count: the invisible avatar must not hide the tags behind it. Skip it and
	# re-ray so real geometry behind it is still tested (bounded retries).
	for _i in 3:
		var body_hit: Dictionary = space.intersect_ray(body_query)
		if body_hit.is_empty():
			break
		var collider = body_hit.get("collider")
		if not (collider is Node3D) or collider.is_visible_in_tree():
			return true
		body_query.exclude = body_query.exclude + [body_hit.get("rid")]

	return _blocked_by_avatar(avatar, from, to)


## True if another avatar's body (analytic vertical cylinder, not the physics
## ClickArea — its default r=0.5 capsule is twice the visual body width and near
## the camera it hid half the tags on screen) crosses the camera→anchor segment.
## Covers comms avatars, scene NPCs (whose ClickArea shape is LOD-disabled) and
## the local player.
static func _blocked_by_avatar(avatar, from: Vector3, to: Vector3) -> bool:
	_snapshot_occluders(avatar.get_tree())
	var self_id: int = avatar.get_instance_id()
	var delta := to - from
	var d_xz := Vector2(delta.x, delta.z)
	var d_xz_len_sq := d_xz.length_squared()
	for i in _occluder_ids.size():
		if _occluder_ids[i] == self_id:
			continue
		var o: Vector3 = _occluder_positions[i]
		var radius: float = _occluder_radii[i]
		var top_y: float = _occluder_tops[i]
		# Closest point (in XZ) of the segment to the other's vertical axis.
		var t := 0.0
		if d_xz_len_sq > 0.0001:
			t = clampf(Vector2(o.x - from.x, o.z - from.z).dot(d_xz) / d_xz_len_sq, 0.0, 1.0)
		var closest := from + delta * t
		# Camera-inside-the-occluder degenerate case (spring arm collapsed against a
		# wall in third person): the camera sitting inside a visible avatar must not
		# blank every tag on screen — the #2321 symptom with a smaller trigger.
		if from.distance_squared_to(closest) < 0.25:
			continue
		if closest.y < o.y - 0.1 or closest.y > top_y:
			continue
		var dx := closest.x - o.x
		var dz := closest.z - o.z
		if dx * dx + dz * dz < radius * radius:
			return true
	return false


## Visible avatar cylinders (comms avatars, the local player, scene NPCs), once per physics
## step: radius from mesh bounds, top at the computed bounds top (capped).
static func _snapshot_occluders(tree: SceneTree) -> void:
	var frame := Engine.get_physics_frames()
	if frame == _occluders_frame:
		return
	_occluders_frame = frame
	var others: Array = []
	if Global.avatars != null:
		others.append_array(Global.avatars.get_avatars())
	var player_avatar = Global.scene_runner.player_avatar_node if Global.scene_runner else null
	if player_avatar != null:
		others.append(player_avatar)
	# Scene NPCs (AvatarShape) are not in Global.avatars — they register in a group.
	others.append_array(tree.get_nodes_in_group("avatar_shapes"))
	_occluder_ids.clear()
	_occluder_positions.clear()
	_occluder_radii.clear()
	_occluder_tops.clear()
	for other in others:
		if not is_instance_valid(other) or not other.visible:
			continue
		var o: Vector3 = other.global_position
		_occluder_ids.append(other.get_instance_id())
		_occluder_positions.append(o)
		_occluder_radii.append(other.occlusion_radius)
		_occluder_tops.append(minf(other.get_bounds_top_y(), o.y + 3.0))


class OcclusionDriver:
	extends Node

	func _physics_process(_delta: float) -> void:
		NameplateLayer._occlusion_step()
