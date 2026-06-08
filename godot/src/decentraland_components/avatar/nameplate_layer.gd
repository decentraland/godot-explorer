class_name NameplateLayer
extends RefCounted

## Shared screen-space layer + runtime for all avatar nameplates (#2215). Avatars
## add their NicknameUI Control here instead of rendering it into a per-avatar
## SubViewport texture (which showed uninitialized-VRAM garbage on mobile). The
## Control is projected onto the head anchor each frame, distance-faded, depth-
## occluded against world geometry by a throttled raycast, and depth-sorted.
## `layer = -1` draws above the 3D world but below the default-layer HUD.

# On-screen size (matches the prior fixed_size Sprite3D) and distance fade
# (full < FADE_START, fade to FADE_END), per nickname_quad.gd.
const SCALE := 0.25
const FADE_START := 10.0
const FADE_END := 15.0
# Alpha units/sec for smooth occlusion fade in/out.
const FADE_SPEED := 6.0
# Occlude against solid world geometry (CL_PHYSICS) and avatar bodies (CL_AVATAR /
# layer 30, the remote click_areas, an Area3D — hence collide_with_areas). The own
# click_area is excluded per-ray so a tag never self-occludes. NOTE: the local player
# frees its click_area, so its own body does not occlude others yet (follow-up).
const CL_PHYSICS := 2
const CL_AVATAR := 536870912
const OCCLUSION_MASK := CL_PHYSICS | CL_AVATAR
# Frames between occlusion raycasts per avatar (staggered) — not every frame.
const OCCLUSION_PERIOD := 6

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
		var anchor: Vector3 = avatar.nickname_quad.global_transform.origin
		var dist: float = cam.global_position.distance_to(anchor)
		if dist <= FADE_END and not cam.is_position_behind(anchor):
			ui.size = ui.get_combined_minimum_size()
			ui.scale = Vector2(SCALE, SCALE)
			var screen_size: Vector2 = ui.size * SCALE
			var pos: Vector2 = (
				cam.unproject_position(anchor) - Vector2(screen_size.x * 0.5, screen_size.y)
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
	if not avatar._nametag_gate_visible:
		return
	if (Engine.get_physics_frames() + int(avatar.unique_id)) % OCCLUSION_PERIOD != 0:
		return
	var cam = avatar.get_viewport().get_camera_3d()
	if cam == null:
		return
	var anchor: Vector3 = avatar.nickname_quad.global_transform.origin
	if cam.global_position.distance_to(anchor) > FADE_END:
		return
	avatar._nameplate_occluded = _occluded(avatar, cam.global_position, anchor)


## True if solid world geometry or another avatar's body sits between camera and anchor.
static func _occluded(avatar, from: Vector3, to: Vector3) -> bool:
	var space = avatar.get_world_3d().direct_space_state
	if space == null:
		return false
	var query := PhysicsRayQueryParameters3D.create(from, to)
	query.collision_mask = OCCLUSION_MASK
	# Avatar bodies (click_area) are Area3D; exclude this avatar's own so it never
	# occludes its own tag (in first person the ray starts inside it → no hit).
	query.collide_with_areas = true
	if is_instance_valid(avatar.click_area):
		query.exclude = [avatar.click_area.get_rid()]
	return not space.intersect_ray(query).is_empty()
