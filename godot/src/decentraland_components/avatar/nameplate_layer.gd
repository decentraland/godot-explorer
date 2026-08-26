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
# Occlusion needs to see (a) solid world geometry + avatar bodies, and (b) other
# avatars — the latter analytically (_blocked_by_avatar), because the ClickArea
# capsule (default r=0.5) is twice the visual body width. We raycast bodies only
# because a single query with areas would also hit the nodeless, pooled DCL
# scene-sensor Area3Ds on CL_PLAYER (layer 3, mask value 4) that sat in front of
# the third-person camera and hid every tag (#2321). See #2637.
const CL_PHYSICS := 2
# CL_PLAYER = layer 3 (mask value 4), per project.godot — avatar TriggerDetectors +
# the local player's CharacterBody3D.
const CL_PLAYER := 4
const BODY_MASK := CL_PHYSICS | CL_PLAYER
# Analytic avatar-occluder fallback dimensions (per-avatar mesh bounds win).
const AVATAR_OCCLUSION_RADIUS := 0.4
# Frames between occlusion raycasts per avatar (staggered) — not every frame.
const OCCLUSION_PERIOD := 6
# Small gap above the computed bounds top (clearance already covers head/hat
# volume — see Avatar.get_bounds_top_y).
const NAMETAG_MARGIN := 0.1
# Vertical gap kept between stacked nameplates.
const STACK_PAD := 2.0
# Fraction of the plate height excluded (top AND bottom) from the de-overlap
# collision rect: the visible pill is much smaller than the Control (theme
# margins), so colliding with the full rect leaves a big transparent gap.
const STACK_COLLISION_INSET := 0.22
# Fraction of the overlap corrected per frame (frame-to-frame relaxation) and of
# the offset decayed back to the anchor when unconstrained (spring home). Partial
# corrections are what make stacking smooth instead of snappy/jittery.
const STACK_RELAX := 0.3
# Extra separation (px) other plates keep from a fully-faded plate — fading tags
# drift away instead of crowding while they disappear.
const STACK_FADE_PAD := 30.0
# Weakest spring factor for a fading plate (alpha 0 = this fraction of RELAX).
const STACK_FADE_MIN_RELAX := 0.15
# Debug: set true at runtime (e.g. the scene-inspector `eval` command, non-production) to bypass
# the occlusion raycast entirely so tags fade by distance only — confirms whether a
# vanishing tag is an occlusion artifact. `NameplateLayer.debug_disable_occlusion = true`.
static var debug_disable_occlusion := false

static var _root: Control = null
# De-overlap state per visible plate (ui instance_id -> last placed Rect2 /
# accumulated displacement from its projected anchor). Plates drift smoothly:
# the offset springs back to zero when nothing overlaps, and overlaps are
# resolved by partial per-frame corrections against other plates' rects.
static var _plate_rects: Dictionary = {}
static var _plate_offsets: Dictionary = {}
# ui instance_id -> last known alpha (for fade-aware spacing).
static var _plate_alphas: Dictionary = {}
# ui instance_id -> world point the tag is drawn at (stacked screen pos unprojected
# at anchor depth). Occlusion rays go HERE, not to the head anchor: the tag is
# visible iff the camera can see the spot where it is actually drawn — otherwise a
# cluster behind one occluder would lose every tag, even the stacked ones.
static var _plate_ray_targets: Dictionary = {}
# Screen rects placed this frame (ui instance_id -> Rect2) for de-overlap stacking.
static var _placed: Dictionary = {}
static var _placed_frame: int = -1


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
			var view_rect := Rect2(Vector2.ZERO, avatar.get_viewport().get_visible_rect().size)
			var on_screen := view_rect.intersects(Rect2(pos, screen_size))
			if on_screen and not avatar._nameplate_occluded:
				# De-overlap: soft-constraint stack against the other visible plates,
				# so close-together avatars' tags (and their chat bubbles, which live
				# inside the same Control) stay readable.
				pos = _stack_position(ui, pos, screen_size, view_rect.size)
				target_a = clampf((FADE_END - dist) / (FADE_END - FADE_START), 0.0, 1.0)
			else:
				_untrack_plate(ui)
			ui.position = pos
			# Closer avatars draw on top.
			ui.z_index = clampi(-int(dist * 100.0), -4000, 4000)
			# Where the tag is actually drawn (center of its collision rect), as a
			# world point at anchor depth — the occlusion ray targets this.
			var draw_center := pos + screen_size * 0.5
			_plate_ray_targets[ui.get_instance_id()] = (
				cam.project_ray_origin(draw_center) + cam.project_ray_normal(draw_center) * dist
			)
		else:
			_untrack_plate(ui)
	else:
		_untrack_plate(ui)
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
	# Occlusion is evaluated against the tag's DRAWN (stacked) position, falling
	# back to the head anchor before the first placement.
	var target: Vector3 = anchor
	if avatar.nickname_ui != null:
		target = _plate_ray_targets.get(avatar.nickname_ui.get_instance_id(), anchor)
	avatar._nameplate_occluded = _occluded(avatar, cam.global_position, target)


## Soft de-overlap: each visible plate carries an offset from its projected anchor.
## The offset springs home when unconstrained, and overlaps push it apart — along
## the axis of least penetration (sideways when cheaper, else up) — easing by a
## fraction per frame. ponytail: O(n²) over visible plates within FADE_END (15m);
## n is small. Upgrade to a spatial hash if ever laggy.
static func _stack_position(
	ui: Control, desired: Vector2, screen_size: Vector2, view_size: Vector2
) -> Vector2:
	var id := ui.get_instance_id()
	var alpha: float = clampf(ui.modulate.a, 0.0, 1.0)
	var offset: Vector2 = _plate_offsets.get(id, Vector2.ZERO)
	# Collide with the visible pill, not the whole Control (transparent margins).
	var inset := screen_size.y * STACK_COLLISION_INSET
	var col_size := Vector2(screen_size.x, screen_size.y - inset * 2.0)
	# Hard target: fully clear any overlapping plate along the axis of LEAST
	# penetration (sideways when that's cheaper, upward otherwise — no name
	# towers), computed from their last placed collision rects. Fixed point per
	# frame, so no limit cycle. Only the higher-instance-id plate yields: if both
	# moved, a pair would escalate without bound.
	var target := Vector2.ZERO
	for other_id in _plate_rects:
		if other_id == id or id < other_id:
			continue
		var other: Rect2 = _plate_rects[other_id]
		var candidate := Rect2(desired + target + Vector2(0, inset), col_size)
		if not candidate.intersects(other):
			continue
		# Keep extra distance from plates that are fading out.
		var other_alpha: float = _plate_alphas.get(other_id, 1.0)
		var pad := STACK_PAD + (1.0 - other_alpha) * STACK_FADE_PAD
		var overlap_x: float = (
			minf(candidate.end.x, other.end.x) - maxf(candidate.position.x, other.position.x)
		)
		var overlap_y: float = (
			minf(candidate.end.y, other.end.y) - maxf(candidate.position.y, other.position.y)
		)
		if overlap_x < overlap_y:
			# Sideways: push to the side this plate is already leaning toward.
			if desired.x + col_size.x * 0.5 >= other.get_center().x:
				target.x = other.end.x + pad - desired.x
			else:
				target.x = other.position.x - pad - col_size.x - desired.x
		else:
			target.y = other.position.y - pad - (desired.y + inset) - col_size.y
	# Spring toward the target: exponential approach, smooth both ways. Fading
	# plates get a weaker spring so they drift instead of snapping.
	var relax: float = STACK_RELAX * lerpf(STACK_FADE_MIN_RELAX, 1.0, alpha)
	offset = offset.lerp(target, relax)
	var pos: Vector2 = desired + offset
	pos.y = maxf(pos.y, 0.0)
	pos.x = clampf(pos.x, 0.0, maxf(view_size.x - screen_size.x, 0.0))
	_plate_offsets[id] = pos - desired
	_plate_rects[id] = Rect2(pos + Vector2(0, inset), col_size)
	_plate_alphas[id] = alpha
	return pos


## Drop a plate from the de-overlap solver (hidden/occluded/off-screen tags reserve
## no space).
static func _untrack_plate(ui: Control) -> void:
	var id := ui.get_instance_id()
	_plate_rects.erase(id)
	_plate_offsets.erase(id)
	_plate_alphas.erase(id)


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
	var body_hit: Dictionary = space.intersect_ray(body_query)
	if not body_hit.is_empty():
		var collider = body_hit.get("collider")
		if not (collider is Node3D) or collider.is_visible_in_tree():
			return true

	return _blocked_by_avatar(avatar, from, to)


## True if another avatar's body (analytic vertical cylinder, not the physics
## ClickArea — its default r=0.5 capsule is twice the visual body width and near
## the camera it hid half the tags on screen) crosses the camera→anchor segment.
## Covers comms avatars, scene NPCs (whose ClickArea shape is LOD-disabled) and
## the local player.
static func _blocked_by_avatar(avatar, from: Vector3, to: Vector3) -> bool:
	var others: Array = []
	if Global.avatars != null:
		others.append_array(Global.avatars.get_avatars())
	var player_avatar = Global.scene_runner.player_avatar_node if Global.scene_runner else null
	if player_avatar != null:
		others.append(player_avatar)
	# Scene NPCs (AvatarShape) are not in Global.avatars — they register in a group.
	others.append_array(avatar.get_tree().get_nodes_in_group("avatar_shapes"))
	var delta := to - from
	var d_xz := Vector2(delta.x, delta.z)
	var d_xz_len_sq := d_xz.length_squared()
	for other in others:
		if other == avatar or not is_instance_valid(other) or not other.visible:
			continue
		var o: Vector3 = other.global_position
		# Cylinder sized to the other's real body+wearables: radius from its mesh
		# bounds, top at its computed bounds top (both capped).
		var radius: float = other.occlusion_radius
		var top_y: float = minf(other.get_bounds_top_y(), o.y + 3.0)
		# Closest point (in XZ) of the segment to the other's vertical axis.
		var t := 0.0
		if d_xz_len_sq > 0.0001:
			t = clampf(Vector2(o.x - from.x, o.z - from.z).dot(d_xz) / d_xz_len_sq, 0.0, 1.0)
		var closest := from + delta * t
		if closest.y < o.y - 0.1 or closest.y > top_y:
			continue
		var dx := closest.x - o.x
		var dz := closest.z - o.z
		if dx * dx + dz * dz < radius * radius:
			return true
	return false
