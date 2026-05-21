class_name Avatar
extends DclAvatar

signal avatar_loaded

# LOD state: FULL (close), MID (15-25m), CROSSFADE (25-30m), FAR (>=30m)
enum LODState { FULL, MID, CROSSFADE, FAR }

# Debug to store each avatar loaded in user://avatars
const DEBUG_SAVE_AVATAR_DATA = false

# Useful to filter wearable categories (and distinguish between top_head and head)
const WEARABLE_NAME_PREFIX = "__"

const TOON_SHADER = preload("res://assets/avatar/dcl_toon.gdshader")
const TOON_SHADER_ALPHA_CLIP = preload("res://assets/avatar/dcl_toon_alpha_clip.gdshader")
const TOON_SHADER_ALPHA_BLEND = preload("res://assets/avatar/dcl_toon_alpha_blend.gdshader")
const TOON_SHADER_DOUBLE = preload("res://assets/avatar/dcl_toon_double.gdshader")
const TOON_SHADER_ALPHA_CLIP_DOUBLE = preload(
	"res://assets/avatar/dcl_toon_alpha_clip_double.gdshader"
)
const TOON_SHADER_ALPHA_BLEND_DOUBLE = preload(
	"res://assets/avatar/dcl_toon_alpha_blend_double.gdshader"
)

# Maps AvatarAnchorPointType (SDK proto, see avatar_attach.proto) to skeleton
# bone names. Ids 0 (POSITION) and 1 (NAME_TAG) are non-skeletal and resolved
# directly in get_anchor_point_global_transform.
const _ANCHOR_BONE_NAMES := {
	2: "Avatar_LeftHand",
	3: "Avatar_RightHand",
	4: "Avatar_Head",
	5: "Avatar_Neck",
	6: "Avatar_Spine",
	7: "Avatar_Spine1",
	8: "Avatar_Spine2",
	9: "Avatar_Hips",
	10: "Avatar_LeftShoulder",
	11: "Avatar_LeftArm",
	12: "Avatar_LeftForeArm",
	13: "Avatar_LeftHandIndex1",
	14: "Avatar_RightShoulder",
	15: "Avatar_RightArm",
	16: "Avatar_RightForeArm",
	17: "Avatar_RightHandIndex1",
	18: "Avatar_LeftUpLeg",
	19: "Avatar_LeftLeg",
	20: "Avatar_LeftFoot",
	21: "Avatar_LeftToeBase",
	22: "Avatar_RightUpLeg",
	23: "Avatar_RightLeg",
	24: "Avatar_RightFoot",
	25: "Avatar_RightToeBase",
}

@export var skip_process: bool = false
@export var hide_name: bool = false:
	set(value):
		hide_name = value
		_apply_nickname_visibility()
@export var non_3d_audio: bool = false

# Entity info for trigger area detection
var dcl_entity_id: int = -1
var is_local_player: bool = false

# Public
var avatar_id: String = ""
var hidden: bool = false
var passport_disabled: bool = false
var nametag_hidden: bool = false
var avatar_ready: bool = false
var has_connected_web3: bool = false  # Whether the user has connected a web3 wallet (not a guest)

# AvatarShape-specific state (NPCs from scene SDK)
var is_avatar_shape: bool = false
var last_expression_trigger_timestamp: int = -1
var last_expression_trigger_id: String = ""

var finish_loading = false
var wearables_by_category: Dictionary = {}

var emote_controller: AvatarEmoteController  # Rust binded. Don't change this variable name

var generate_attach_points: bool = false
# anchor_point_id -> bone index in body_shape_skeleton_3d. Only entries whose
# bone resolved at activation time are stored.
var anchor_bone_idx: Dictionary = {}
# anchor_point_id -> cached bone global pose, with basis pre-scaled by 100 to
# cancel the Skeleton3D's 0.01 unit-conversion scale. After composing with
# body_shape_skeleton_3d.global_transform the basis has world scale 1; the
# entity's own scale is then preserved by avatar_attach.gd.
var anchor_transform: Dictionary = {}

var voice_chat_audio_player: AudioStreamPlayer = null
var voice_chat_audio_player_gen: AudioStreamGenerator = null

var mask_material = preload("res://assets/avatar/mask_material.tres")

# Signal-based wearable loader for threaded loading
var wearable_loader: WearableLoader = null

# Session-level override (e.g. "Hide UI" setting). This should not persist into avatar state.
var _force_hide_name: bool = false

# Previous-frame jump_count for rising-edge detection of double-jump SFX.
var _last_jump_count: int = 0
# #b2: first _process tick should not treat wire-provided jump_count>=2 as a
# rising edge — otherwise a remote avatar first seen mid-double-jump plays the
# SFX from nothing. Cleared after the first frame where we seed _last_jump_count.
var _jump_count_sync_pending: bool = true
# Latched so we don't spam Close audio / Glider_End restart / hide-timer scheduling.
var _glider_close_initiated: bool = false
# Previous glide_state for _update_glider_prop's edge detection.
var _prop_last_glide_state: int = 0
# #b1/#b12: first call to _update_glider_prop should adopt whatever curr_state
# came in on the wire (OPENING/GLIDING/CLOSING) without spamming audio, instead
# of staying invisible because prev_state==0 doesn't match any branch.
var _prop_sync_pending: bool = true
var _glide_forward_blend: float = 0.0

# Registry for scene emote content URLs: scene_id -> {base_url, emotes: {glb_hash -> audio_hash}}
var _scene_emote_registry: Dictionary = {}

# Indices of bones added to body_shape_skeleton_3d by _merge_extra_wearable_bones_into_base
# and currently in use by the active wearables.
var _active_extra_bone_indices: Array[int] = []
# Slots recycled from previous merges (renamed + disabled) waiting to be reused by the next
# merge. Skeleton3D has no remove_bone() in Godot 4.6, so reusing slots is the only way to
# keep bone_count bounded across outfit / body-shape changes.
var _free_bone_pool: Array[int] = []
var _stale_bone_counter: int = 0

var _lod_state: int = LODState.FULL
var _impostor_layer: int = -1
var _lod_phase: int = 0
var _mesh_lod_visibility_captured: bool = false
# Written by AvatarLODCoordinator each tick. Caps the natural distance LOD so
# only the N closest avatars stay FULL, the next M MID/CROSSFADE, rest FAR.
var _lod_rank_cap: int = LODState.FULL
# Set by AvatarLODCoordinator: true when this avatar's rank is beyond the real
# impostor layer cap. Such avatars borrow another slot's texture and render
# fully tinted (black silhouette). Tracks the active slot's mode so a flip
# triggers a clean reallocation.
var _use_overflow_impostor: bool = false
var _impostor_layer_is_overflow: bool = false
# Set by AvatarLODCoordinator: true when the avatar's bounding sphere is fully
# outside the camera frustum. Off-frustum avatars release their impostor slot
# entirely — no multimesh instance, no real layer, no capture. Disk cache makes
# re-entry fast (texture rehydrates from PNG without recapture).
var _off_frustum: bool = false
# Latched while off-frustum: the AnimationTree was paused regardless of LOD
# state, so when we come back in-frustum we know we have to restore the
# state-driven anim setup (active/manual/throttle).
var _anim_frozen_off_frustum: bool = false
# Wall-clock ms when the freeze started. Used to advance the AnimationTree by
# the elapsed time on re-entry so the emote phase matches what it would have
# been had we not paused — single one-shot recompute, not a frame-by-frame
# catch-up, so the CPU saving from the freeze is preserved.
var _anim_freeze_start_ms: int = 0

# Skinning throttle (MID/CROSSFADE only): drive AnimationTree manually and
# advance every N frames so the skeleton bones update at ~20fps instead of
# ~60fps. Imperceptible at 15-30m distance and a sizeable CPU saving when
# many avatars share the screen.
var _anim_throttle_acc: float = 0.0
var _anim_throttle_counter: int = 0
var _anim_throttle_active: bool = false

@onready var animation_tree = $AnimationTree
@onready var animation_player = $AnimationPlayer

@onready var nickname_ui = %NicknameUI
@onready var nickname_quad = %NicknameQuad
@onready var nickname_viewport = %NicknameViewport

@onready var timer_hide_mic = %Timer_HideMic
@onready var body_shape_skeleton_3d: Skeleton3D = $Armature/Skeleton3D
@onready var bone_attachment_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name
@onready var audio_player_emote = $AudioPlayer_Emote

@onready var avatar_modifier_area_detector = $avatar_modifier_area_detector
@onready var click_area = $ClickArea
@onready var trigger_detector = %TriggerDetector

@onready var glider_prop: Node3D = %GliderProp
@onready var audio_player_double_jump: AudioStreamPlayer3D = %AudioPlayer_DoubleJump

# Cache of toon ShaderMaterials keyed by source BaseMaterial3D's instance_id.
# Lets avatars wearing the same wearable share a single ShaderMaterial across
# the whole scene. Skin/hair surfaces clone-on-write in apply_color_and_facial
# so per-avatar tints don't leak.
static var _toon_material_cache: Dictionary = {}

# Issue #1945: matches a Blender-style `_<digits>$` duplicate-import suffix on a
# bone name (e.g. `Avatar_Hips_2`). Compiled once and shared across instances.
static var _bone_suffix_regex: RegEx = RegEx.create_from_string("^(.*)_\\d+$")


func _ready():
	var billboard_mode = (
		BaseMaterial3D.BillboardMode.BILLBOARD_FIXED_Y
		if Global.is_xr()
		else BaseMaterial3D.BillboardMode.BILLBOARD_ENABLED
	)
	nickname_quad.billboard = billboard_mode

	wearable_loader = WearableLoader.new()
	emote_controller = AvatarEmoteController.new(self, animation_player, animation_tree)
	body_shape_skeleton_3d.skeleton_updated.connect(self._attach_point_skeleton_updated)

	avatar_modifier_area_detector.set_avatar_modifier_area.connect(
		self._on_set_avatar_modifier_area
	)
	avatar_modifier_area_detector.unset_avatar_modifier_area.connect(
		self._unset_avatar_modifier_area
	)

	if non_3d_audio:
		var audio_player_name = audio_player_emote.get_name()
		remove_child(audio_player_emote)
		audio_player_emote.queue_free()

		audio_player_emote = AudioStreamPlayer.new()
		audio_player_emote.bus = &"AvatarAndEmotes"
		add_child(audio_player_emote)
		audio_player_emote.name = audio_player_name

	# Hide mic when the avatar is spawned
	nickname_ui.mic_enabled = false
	Global.on_chat_message.connect(on_chat_message)
	_apply_nickname_visibility()

	_lod_phase = int(self.unique_id) % AvatarImpostorConfig.DISTANCE_CHECK_PERIOD_FRAMES
	AvatarLODCoordinator.register(self)

	# Setup metadata for raycast detection (same as DCL entities)
	click_area.set_meta("is_avatar", true)
	click_area.set_meta("avatar_id", avatar_id)


func _exit_tree() -> void:
	AvatarLODCoordinator.unregister(self)

	# For local player and remote avatars, trigger detection is setup later via setup_trigger_detection()
	# For AvatarShapes (scene NPCs), remove_trigger_detection() is called from avatar_shape.rs


## Setup trigger detection for this avatar (local player and remote avatars only).
## - For local player: entity_id=SceneEntityId.PLAYER (0x10000)
## - For remote avatars: entity_id=assigned entity from avatar_scene.rs
func setup_trigger_detection(p_entity_id: int) -> void:
	dcl_entity_id = p_entity_id

	# Set metadata on TriggerDetector so trigger_area.rs can identify this avatar
	trigger_detector.set_meta("dcl_entity_id", dcl_entity_id)

	# Enable the collision shape
	trigger_detector.get_node("CollisionShape3D").disabled = false


## Remove trigger detection for this avatar (AvatarShapes/scene NPCs only).
## Called from avatar_shape.rs after the avatar is added to the scene.
func remove_trigger_detection() -> void:
	if trigger_detector != null:
		trigger_detector.queue_free()
		trigger_detector = null


func on_chat_message(address: String, message: String, _timestamp: float):
	if avatar_id != address:
		return
	nickname_ui.async_show_message(message)
	_request_nickname_redraw()


func _input(event):
	if event.is_action_pressed("ia_pointer"):
		# Only handle input if this avatar is currently selected and not blocked/hidden
		var selected = Global.get_selected_avatar()
		if selected and selected == self and avatar_id and not hidden and not passport_disabled:
			if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
				var explorer = Global.get_explorer()
				if (
					is_instance_valid(explorer)
					and explorer.is_session_hide_main_hud()
					and explorer.is_session_hide_view_profile()
				):
					return
				Global.open_profile_by_avatar.emit(self)


func try_show():
	avatar_modifier_area_detector.check_areas()


func _on_set_avatar_modifier_area(area: DclAvatarModifierArea3D):
	_unset_avatar_modifier_area()  # Reset state

	for exclude_id in area.exclude_ids:
		if avatar_id == exclude_id:
			return  # the avatar is not going to be modified

	for modifier in area.avatar_modifiers:
		if modifier == 0:  # hide avatar
			hide()
			_hide_impostor_render()
			_set_click_area_enabled(false)
		elif modifier == 1:  # disable passport
			passport_disabled = true
		elif modifier == 2:  # hide nametag
			nametag_hidden = true

	_apply_nickname_visibility()


func set_hidden(value):
	hidden = value
	if hidden:
		hide()
		_hide_impostor_render()
		# Disable click detection so blocked/hidden avatars can't be interacted with
		_set_click_area_enabled(false)
	else:
		try_show()
		# Re-enable click detection
		_set_click_area_enabled(true)


# The impostor MultiMesh lives on AvatarScene (parent), not on the Avatar node,
# so hide()/visible=false on the avatar doesn't affect it. Force its slot's
# fade_alpha to 0 so the GPU discards the fragment until LOD recomputes.
func _hide_impostor_render() -> void:
	if _impostor_layer >= 0 and Global.avatars != null:
		Global.avatars.set_impostor_state(get_instance_id(), 0.0, 0.0, 0.0)


# Stable identity key for the disk-backed impostor texture cache. The hash
# combines eth address (for cross-session stability) with the visual identity
# (body + wearables + colors), so a user that changes outfit gets a fresh
# capture instead of pulling stale pixels from the previous look's PNG. NPCs
# fall through the same path with an empty eth segment, so visually-identical
# NPCs share a cache entry.
func _get_impostor_cache_key() -> String:
	if avatar_data == null:
		return ""
	var parts := PackedStringArray()
	parts.append(avatar_id.to_lower() if avatar_id != "" else "")
	parts.append(avatar_data.get_body_shape())
	var wearables = avatar_data.get_wearables()
	if wearables is Array:
		var sorted_wearables: Array = wearables.duplicate()
		sorted_wearables.sort()
		for w in sorted_wearables:
			parts.append(w)
	parts.append(str(avatar_data.get_skin_color()))
	parts.append(str(avatar_data.get_eyes_color()))
	parts.append(str(avatar_data.get_hair_color()))
	return "|".join(parts).sha1_text()


func _set_click_area_enabled(enabled: bool) -> void:
	if click_area:
		var collision_shape = click_area.get_node_or_null("CollisionShape3D")
		if collision_shape:
			collision_shape.disabled = not enabled


func _unset_avatar_modifier_area():
	if not hidden:
		show()
		_set_click_area_enabled(true)
	passport_disabled = false
	nametag_hidden = false
	_apply_nickname_visibility()


func async_update_avatar_from_profile(profile: DclUserProfile):
	var avatar = profile.get_avatar()
	var new_avatar_name: String = profile.get_name()
	if not profile.has_claimed_name():
		new_avatar_name += "#" + profile.get_ethereum_address().right(4)
	nickname_ui.name_claimed = profile.has_claimed_name()

	avatar_id = profile.get_ethereum_address()
	has_connected_web3 = profile.has_connected_web3()
	prints("Async update avatar from profile", avatar_id)

	# Update metadata with the new avatar_id
	if click_area:
		click_area.set_meta("avatar_id", avatar_id)

	await async_update_avatar(avatar, new_avatar_name)


func async_update_avatar(
	new_avatar: DclAvatarWireFormat, new_avatar_name: String, avatar_shape_config: Dictionary = {}
):
	if new_avatar == null:
		printerr("Trying to update an avatar with an null value")
		return

	# Handle AvatarShape-specific config (NPCs from scene SDK)
	is_avatar_shape = avatar_shape_config.get("is_avatar_shape", false)

	# Adopt the AvatarShape.id when it's an eth address so the impostor capturer
	# can route through the catalyst body-texture path
	# (`avatar_id.begins_with("0x")`) instead of an off-screen render.
	if is_avatar_shape:
		var shape_id: String = avatar_shape_config.get("id", "")
		if shape_id.begins_with("0x") and avatar_id != shape_id:
			avatar_id = shape_id
			if click_area:
				click_area.set_meta("avatar_id", avatar_id)

	# Update metadata for raycast detection
	if click_area:
		click_area.set_meta("is_avatar_shape", is_avatar_shape)

	# Handle expression_trigger for AvatarShape emotes
	if is_avatar_shape:
		var expression_trigger_id = avatar_shape_config.get("expression_trigger_id", "")
		var expression_trigger_timestamp: int = avatar_shape_config.get(
			"expression_trigger_timestamp", -1
		)

		# Determine if we should trigger the emote:
		# 1. If timestamp is valid (>= 0) and greater than last timestamp, OR
		# 2. If no timestamp (-1) but the expression_trigger_id changed
		var should_trigger = false
		if not expression_trigger_id.is_empty():
			if expression_trigger_timestamp >= 0:
				# Timestamp-based triggering (Lamport timestamp pattern)
				should_trigger = expression_trigger_timestamp > last_expression_trigger_timestamp
			else:
				# No timestamp - trigger when id changes
				should_trigger = expression_trigger_id != last_expression_trigger_id

		if should_trigger:
			last_expression_trigger_timestamp = expression_trigger_timestamp
			last_expression_trigger_id = expression_trigger_id
			# Defer emote play to after avatar is loaded if needed
			if avatar_ready:
				_async_play_expression_trigger(expression_trigger_id)
			else:
				# Store pending emote to play after avatar loads
				set_meta("pending_expression_trigger", expression_trigger_id)

	# Skip redundant updates - if avatar data hasn't changed and avatar is already loaded,
	# no need to re-duplicate all meshes and materials (saves Vulkan descriptor sets)
	if finish_loading and avatar_data != null and avatar_data.equal(new_avatar):
		# Only update the name if it changed
		if get_avatar_name() != new_avatar_name:
			set_avatar_name(new_avatar_name)
			var splitted_nickname = new_avatar_name.split("#", false)
			if splitted_nickname.size() > 1:
				nickname_ui.nickname = splitted_nickname[0]
				nickname_ui.tag = splitted_nickname[1]
			else:
				nickname_ui.nickname = new_avatar_name
				nickname_ui.tag = ""
			nickname_ui.nickname_color = DclAvatar.get_nickname_color(new_avatar_name)
			# Re-trigger UPDATE_ONCE so the SubViewport repaints with the new text
			_apply_nickname_visibility()
		return

	set_avatar_data(new_avatar)
	set_avatar_name(new_avatar_name)

	var wearable_to_request := []

	var splitted_nickname = new_avatar_name.split("#", false)
	if splitted_nickname.size() > 1:
		nickname_ui.nickname = splitted_nickname[0]
		nickname_ui.tag = splitted_nickname[1]
	else:
		nickname_ui.nickname = new_avatar_name
		nickname_ui.tag = ""

	nickname_ui.nickname_color = DclAvatar.get_nickname_color(new_avatar_name)
	nickname_ui.mic_enabled = false

	_apply_nickname_visibility()

	wearable_to_request.append_array(avatar_data.get_wearables())

	for emote_urn in avatar_data.get_emotes():
		if emote_urn.begins_with("urn"):
			wearable_to_request.push_back(emote_urn)

	wearable_to_request.push_back(avatar_data.get_body_shape())

	# Enable to store a bunch of avatar of a session
	if DEBUG_SAVE_AVATAR_DATA:
		DirAccess.make_dir_absolute("user://avatars")
		var file_path = (
			"user://avatars/"
			+ (
				(
					avatar_id
					+ "_"
					+ new_avatar_name
					+ "_"
					+ str(Time.get_unix_time_from_system())
					+ ".json"
				)
				. validate_filename()
			)
		)
		var dict: Dictionary = {
			"userId": avatar_id,
			"name": new_avatar_name,
			"time": Time.get_unix_time_from_system(),
			"wearables": avatar_data.get_wearables(),
			"bodyShape": avatar_data.get_body_shape(),
			"forceRender": avatar_data.get_force_render(),
			"emotes": avatar_data.get_emotes()
		}
		var file = FileAccess.open(file_path, FileAccess.WRITE)
		if file != null:
			file.store_string(JSON.stringify(dict))
			file.close()

	# TODO: Validate if the current profile can own this wearables
	# tracked at https://github.com/decentraland/godot-explorer/issues/244
	# wearable_to_request = filter_owned_wearables(wearable_to_request)

	finish_loading = false

	var promise = Global.content_provider.fetch_wearables(
		wearable_to_request, Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)
	await async_fetch_wearables_dependencies()


func set_force_hide_name(value: bool) -> void:
	if _force_hide_name == value:
		return
	_force_hide_name = value
	if is_inside_tree():
		_apply_nickname_visibility()


## Bump the nickname SubViewport to redraw exactly one frame. UPDATE_ONCE
## auto-resets to UPDATE_DISABLED after rendering, so callers must invoke
## this every time something nickname-related changes — `_apply_nickname_visibility`
## bumps once on show, individual setters (chat message, mic, etc.) bump as
## state changes, and `_process` bumps after the viewport resizes.
##
## Gate on `nickname_quad.visible` (the source of truth for "is this nickname
## actually being shown") rather than `render_target_update_mode == UPDATE_DISABLED`
## — the latter is also the post-render state after UPDATE_ONCE auto-resets, so
## it can't distinguish "explicitly hidden" from "just finished rendering one frame".
func _request_nickname_redraw() -> void:
	if nickname_viewport == null or nickname_quad == null:
		return
	if not nickname_quad.visible:
		return
	nickname_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE


func _apply_nickname_visibility() -> void:
	if nickname_quad == null:
		return
	# Hide nickname for AvatarShapes only when the scene didn't set a real name
	# (the proto default is "NPC", which is noise). Also hide on FAR LOD —
	# unreadable at impostor distance and each quad is an extra draw call.
	var current_name: String = get_avatar_name()
	var avatar_shape_has_no_name: bool = (
		is_avatar_shape and (current_name.is_empty() or current_name == "NPC")
	)
	var far_lod: bool = _lod_state == LODState.FAR
	var should_hide := (
		avatar_shape_has_no_name or hide_name or _force_hide_name or far_lod or nametag_hidden
	)
	if should_hide:
		nickname_quad.hide()
		if nickname_viewport != null:
			nickname_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	else:
		nickname_quad.show()
		if nickname_viewport != null:
			# UPDATE_ONCE: redraw one frame here, then the SubViewport idles until
			# something nickname-related changes (see _request_nickname_redraw).
			nickname_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE


func update_colors(eyes_color: Color, skin_color: Color, hair_color: Color) -> void:
	avatar_data.set_eyes_color(eyes_color)
	avatar_data.set_skin_color(skin_color)
	avatar_data.set_hair_color(hair_color)

	if finish_loading:
		apply_color_and_facial()
		if _impostor_layer >= 0 and not _impostor_layer_is_overflow and Global.avatars != null:
			Global.avatars.invalidate_impostor_texture(get_instance_id(), _get_impostor_cache_key())
			ImpostorCapturer.request_capture(self)


func async_fetch_wearables_dependencies():
	var wearables_dict: Dictionary = {}

	# Fill data
	var body_shape_id := avatar_data.get_body_shape()
	wearables_dict[body_shape_id] = Global.content_provider.get_wearable(body_shape_id)
	for item in avatar_data.get_wearables():
		wearables_dict[item] = Global.content_provider.get_wearable(item)

	var async_calls_info: Array = []
	var async_calls: Array = []
	for emote_urn in avatar_data.get_emotes():
		if emote_urn.begins_with("urn"):
			var emote_promises = emote_controller.async_fetch_emote(emote_urn, body_shape_id)
			for emote_promise in emote_promises:
				async_calls.push_back(emote_promise)
				async_calls_info.push_back(emote_urn)

	# Use signal-based wearable loading with threaded ResourceLoader
	# Safety check: avatar may have been freed during async operations
	if not is_instance_valid(wearable_loader) or not is_inside_tree():
		return
	await wearable_loader.async_load_wearables(wearables_dict.keys(), body_shape_id)

	var promises_result: Array = await PromiseUtils.async_all(async_calls)
	for i in range(promises_result.size()):
		if promises_result[i] is PromiseError:
			printerr("Error loading ", async_calls_info[i], ":", promises_result[i].get_error())

	await async_load_wearables()


func async_try_to_set_body_shape(body_shape_hash):
	# Safety check: avatar may have been freed during async operations
	if not is_instance_valid(wearable_loader) or not is_inside_tree():
		return
	var body_shape: Node3D = await wearable_loader.async_get_wearable_node(body_shape_hash)
	if body_shape == null:
		printerr("Avatar: Failed to load body shape ", body_shape_hash)
		return

	var new_skeleton = body_shape.find_child("Skeleton3D")
	if new_skeleton == null:
		body_shape.queue_free()
		return

	for child in body_shape_skeleton_3d.get_children():
		if child is MeshInstance3D:
			body_shape_skeleton_3d.remove_child(child)
			child.queue_free()

	# Recycle any extra bones merged in the previous assembly so the upcoming
	# _merge_extra_wearable_bones_into_base pass starts from a clean slate.
	_recycle_extra_wearable_bones()

	# Reparent children directly (no need to duplicate since wearable_loader
	# returns a fresh instantiated scene that we'll discard anyway)
	for child in new_skeleton.get_children():
		new_skeleton.remove_child(child)
		child.set_owner(null)  # Clear owner since we're reparenting
		child.name = "bodyshape_" + child.name.to_lower()
		body_shape_skeleton_3d.add_child(child)

	# Free the now-empty body shape container
	body_shape.queue_free()
	_add_attach_points()


# Renames bones previously merged via _merge_extra_wearable_bones_into_base to a
# unique stale sentinel, disables them, detaches them from the active hierarchy
# (parent=-1, rest=identity), and pushes their indices into the free pool so the
# next merge can reuse them instead of growing the skeleton. Detaching keeps the
# per-frame skeleton transform walk cheap by leaving stale slots as flat roots.
func _recycle_extra_wearable_bones() -> void:
	if _active_extra_bone_indices.is_empty():
		return
	for bone_idx in _active_extra_bone_indices:
		if bone_idx < 0 or bone_idx >= body_shape_skeleton_3d.get_bone_count():
			continue
		var stale_name = "__stale_bone_%d" % _stale_bone_counter
		_stale_bone_counter += 1
		body_shape_skeleton_3d.set_bone_name(bone_idx, stale_name)
		body_shape_skeleton_3d.set_bone_enabled(bone_idx, false)
		body_shape_skeleton_3d.set_bone_parent(bone_idx, -1)
		body_shape_skeleton_3d.set_bone_rest(bone_idx, Transform3D.IDENTITY)
		body_shape_skeleton_3d.reset_bone_pose(bone_idx)
		_free_bone_pool.push_back(bone_idx)
	_active_extra_bone_indices.clear()


# Resolves a wearable bone name to its counterpart in body_shape_skeleton_3d,
# stripping a Blender-style duplicate-import suffix (`_2`, `_001`, ...) only
# when the un-suffixed name already exists. Returns the original name otherwise
# so genuine extra bones (ADR-316 spring bones) still get merged as new bones.
# Fixes #1945: wearables exported from Blender after re-importing the DCL armature
# carry `Avatar_Hips_2` etc. — without this collapse they merge as a parallel,
# un-animated leg/spine chain that stays in rest pose during emotes/jump/glide.
func _resolve_to_base_bone_name(bone_name: String) -> String:
	if body_shape_skeleton_3d.find_bone(bone_name) != -1:
		return bone_name
	var m := _bone_suffix_regex.search(bone_name)
	if m == null:
		return bone_name
	var stripped := m.get_string(1)
	if body_shape_skeleton_3d.find_bone(stripped) != -1:
		return stripped
	return bone_name


# Copies bones that exist in the wearable's Skeleton3D but not in body_shape_skeleton_3d
# (typically ADR-316 spring bones for hair, earrings, capes, etc.). Parents are added
# before children so parent-by-name resolution always succeeds. Without this, mesh
# skins referencing indices beyond body_shape_skeleton_3d.get_bone_count() log
# `Skin bind #N contains bone index bind: N, which is greater than the skeleton bone count`.
func _merge_extra_wearable_bones_into_base(wearable_skel: Skeleton3D) -> void:
	var wearable_bone_count = wearable_skel.get_bone_count()
	if wearable_bone_count == 0:
		return

	# Collect missing bones along with their depth in the wearable hierarchy so we
	# can add parents before children.
	var missing: Array = []  # Array of [depth, wearable_idx, name]
	for i in wearable_bone_count:
		var bone_name = wearable_skel.get_bone_name(i)
		# Skip if the bone already exists in the base, including under its
		# de-suffixed name. The wearable's `Avatar_Hips_2` collapses onto the
		# animated `Avatar_Hips` instead of being merged as a parallel root.
		if _resolve_to_base_bone_name(bone_name) != bone_name:
			continue
		if body_shape_skeleton_3d.find_bone(bone_name) != -1:
			continue
		var depth = 0
		var cursor = wearable_skel.get_bone_parent(i)
		while cursor != -1:
			depth += 1
			cursor = wearable_skel.get_bone_parent(cursor)
		missing.push_back([depth, i, bone_name])

	if missing.is_empty():
		return

	missing.sort_custom(func(a, b): return a[0] < b[0])

	for entry in missing:
		var wearable_idx: int = entry[1]
		var bone_name: String = entry[2]
		var new_idx: int
		if not _free_bone_pool.is_empty():
			new_idx = _free_bone_pool.pop_back()
			body_shape_skeleton_3d.set_bone_name(new_idx, bone_name)
			body_shape_skeleton_3d.set_bone_enabled(new_idx, true)
		else:
			new_idx = body_shape_skeleton_3d.add_bone(bone_name)
		body_shape_skeleton_3d.set_bone_rest(new_idx, wearable_skel.get_bone_rest(wearable_idx))
		body_shape_skeleton_3d.reset_bone_pose(new_idx)
		_active_extra_bone_indices.push_back(new_idx)
		# Always reset parent: a recycled slot may have been linked to a stale
		# parent from its previous use. Resolve through the same de-suffix path
		# so a spring bone whose parent is `Avatar_Spine_2` reparents onto the
		# base `Avatar_Spine` instead of leaving as root.
		var parent_wearable_idx = wearable_skel.get_bone_parent(wearable_idx)
		var parent_base_idx = -1
		if parent_wearable_idx != -1:
			var parent_name = wearable_skel.get_bone_name(parent_wearable_idx)
			parent_base_idx = body_shape_skeleton_3d.find_bone(
				_resolve_to_base_bone_name(parent_name)
			)
			if parent_base_idx == -1:
				push_warning(
					(
						"[AVATAR] Extra bone '%s' parent '%s' not found in base skeleton; leaving as root"
						% [bone_name, parent_name]
					)
				)
		body_shape_skeleton_3d.set_bone_parent(new_idx, parent_base_idx)


# Rewrites a MeshInstance3D's Skin so every bind references its target bone by name.
# Godot resolves named binds against the attached skeleton at runtime, so once the
# mesh is reparented to body_shape_skeleton_3d (which may have been extended with
# extra wearable bones) every joint resolves correctly, including ADR-316 spring bones.
# Issue #1945: when the wearable was exported with duplicate-suffixed bones
# (`Avatar_Hips_2`, `Avatar_LeftLeg_2`, ...), the de-suffix lookup retargets the
# binds onto the animated base bones — without it the mesh tracks merged but
# inert `_2` clones and stays in rest pose during emotes/jump/glide.
func _rebind_skin_by_name(mesh: MeshInstance3D, wearable_skel: Skeleton3D) -> void:
	if mesh.skin == null:
		return
	var skin: Skin = mesh.skin.duplicate()
	var wearable_bone_count = wearable_skel.get_bone_count()
	for i in skin.get_bind_count():
		var bone_idx = skin.get_bind_bone(i)
		if bone_idx >= 0 and bone_idx < wearable_bone_count:
			var bone_name = wearable_skel.get_bone_name(bone_idx)
			skin.set_bind_name(i, _resolve_to_base_bone_name(bone_name))
	mesh.skin = skin


func _convert_to_toon(base_mat: BaseMaterial3D) -> ShaderMaterial:
	var is_alpha_scissor = base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	var is_alpha_blend = (
		base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA
		or base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_HASH
		or base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_DEPTH_PRE_PASS
	)
	var double_sided = base_mat.cull_mode == BaseMaterial3D.CULL_DISABLED
	var toon_mat = ShaderMaterial.new()
	if is_alpha_scissor and double_sided:
		toon_mat.shader = TOON_SHADER_ALPHA_CLIP_DOUBLE
	elif is_alpha_scissor:
		toon_mat.shader = TOON_SHADER_ALPHA_CLIP
	elif is_alpha_blend and double_sided:
		toon_mat.shader = TOON_SHADER_ALPHA_BLEND_DOUBLE
	elif is_alpha_blend:
		toon_mat.shader = TOON_SHADER_ALPHA_BLEND
	elif double_sided:
		toon_mat.shader = TOON_SHADER_DOUBLE
	else:
		toon_mat.shader = TOON_SHADER
	toon_mat.set_shader_parameter("albedo_color", base_mat.albedo_color)
	if base_mat.albedo_texture:
		toon_mat.set_shader_parameter("albedo_texture", base_mat.albedo_texture)
	if base_mat.emission_enabled:
		toon_mat.set_shader_parameter("emission_color", base_mat.emission)
		if base_mat.emission_texture:
			toon_mat.set_shader_parameter("emission_texture", base_mat.emission_texture)
	if is_alpha_scissor:
		toon_mat.set_shader_parameter("alpha_scissor_threshold", base_mat.alpha_scissor_threshold)
	return toon_mat


func apply_toon_material(node_to_apply: Node):
	if not (node_to_apply is MeshInstance3D) or node_to_apply.mesh == null:
		return
	for surface_idx in range(node_to_apply.mesh.get_surface_count()):
		var mat = node_to_apply.mesh.surface_get_material(surface_idx)
		if mat == null or not (mat is BaseMaterial3D):
			continue
		var key: int = mat.get_instance_id()
		var cached = _toon_material_cache.get(key)
		if cached == null or not is_instance_valid(cached):
			cached = _convert_to_toon(mat)
			_toon_material_cache[key] = cached
		node_to_apply.mesh.surface_set_material(surface_idx, cached)


func async_load_wearables():
	# Safety check: avatar may have been freed during async operations
	if not is_instance_valid(wearable_loader) or not is_inside_tree():
		return

	# Hide skeleton immediately if show_only_wearables to prevent flash of default body
	var show_only_wearables = avatar_data.get_show_only_wearables()
	if show_only_wearables:
		body_shape_skeleton_3d.visible = false

	var curated_wearables := Wearables.get_curated_wearable_list(
		avatar_data.get_body_shape(),
		avatar_data.get_wearables(),
		avatar_data.get_force_render(),
		avatar_data.get_show_only_wearables()
	)
	if curated_wearables.wearables_by_category.is_empty():
		printerr("couldn't get curated wearables")
		return

	wearables_by_category = curated_wearables.wearables_by_category
	var body_shape_wearable = wearables_by_category.get(Wearables.Categories.BODY_SHAPE)
	if body_shape_wearable == null:
		printerr("body shape not found")
		return

	# If some wearables are needed but they weren't included in the first request (fallback wearables)
	if not curated_wearables.need_to_fetch.is_empty():
		var need_to_fetch_promise = Global.content_provider.fetch_wearables(
			Array(curated_wearables.need_to_fetch), Global.realm.get_profile_content_url()
		)
		await PromiseUtils.async_all(need_to_fetch_promise)
		# Safety check: avatar may have been freed during async operations
		if not is_instance_valid(wearable_loader) or not is_inside_tree():
			return
		# Use signal-based wearable loading with threaded ResourceLoader
		await wearable_loader.async_load_wearables(
			curated_wearables.need_to_fetch, body_shape_wearable.get_id()
		)

		for wearable_id in curated_wearables.need_to_fetch:
			var wearable = Global.content_provider.get_wearable(wearable_id)
			if wearable != null:
				wearables_by_category[wearable.get_category()] = wearable

	await async_try_to_set_body_shape(
		Wearables.get_item_main_file_hash(body_shape_wearable, avatar_data.get_body_shape())
	)
	wearables_by_category.erase(Wearables.Categories.BODY_SHAPE)

	var has_own_skin = false
	var has_own_upper_body = false
	var has_own_lower_body = false
	var has_own_feet = false
	var has_own_hands = false
	var has_own_head = false

	for category in wearables_by_category:
		# Safety check: avatar may have been freed during async operations
		if not is_instance_valid(wearable_loader) or not is_inside_tree():
			return

		var wearable = wearables_by_category[category]

		# Skip texture-based wearables (eyes, eyebrows, mouth)
		if Wearables.is_texture(category):
			continue

		var file_hash = Wearables.get_item_main_file_hash(wearable, avatar_data.get_body_shape())
		var obj = await wearable_loader.async_get_wearable_node(file_hash)
		if obj == null:
			printerr("Avatar: Failed to load wearable ", category, " hash: ", file_hash)
			continue

		# Reparent wearable meshes directly (no need to duplicate since wearable_loader
		# returns a fresh instantiated scene that we'll discard anyway)
		var wearable_skeletons = obj.find_children("Skeleton3D")
		for skeleton_3d in wearable_skeletons:
			# Spring bones (ADR-316) and other extra bones not in the base armature
			# must be copied into body_shape_skeleton_3d before meshes are reparented,
			# otherwise mesh skins reference bone indices that don't exist here.
			_merge_extra_wearable_bones_into_base(skeleton_3d)

			for child in skeleton_3d.get_children():
				if child is MeshInstance3D:
					_rebind_skin_by_name(child, skeleton_3d)
				skeleton_3d.remove_child(child)
				child.set_owner(null)  # Clear owner since we're reparenting
				# WEARABLE_NAME_PREFIX is used to identify non-bodyshape parts
				child.name = child.name.to_lower() + WEARABLE_NAME_PREFIX + category
				body_shape_skeleton_3d.add_child(child)

		# Free the now-empty wearable container
		obj.queue_free()

		match category:
			Wearables.Categories.UPPER_BODY:
				has_own_upper_body = true
			Wearables.Categories.LOWER_BODY:
				has_own_lower_body = true
			Wearables.Categories.FEET:
				has_own_feet = true
			Wearables.Categories.HANDS:
				has_own_hands = true
			Wearables.Categories.HEAD:
				has_own_head = true
			Wearables.Categories.SKIN:
				has_own_skin = true

	# Here hidings is an alias
	var hidings = curated_wearables.hidden_categories

	# When show_only_wearables is true, hide all body parts (skin, hair, facial features)
	var base_bodyshape_hidings = {
		"ubody_basemesh":
		show_only_wearables or has_own_skin or has_own_upper_body or hidings.has("upper_body"),
		"lbody_basemesh":
		show_only_wearables or has_own_skin or has_own_lower_body or hidings.has("lower_body"),
		"feet_basemesh": show_only_wearables or has_own_skin or has_own_feet or hidings.has("feet"),
		"hands_basemesh":
		show_only_wearables or has_own_skin or has_own_hands or hidings.has("hands"),
		"head_basemesh": show_only_wearables or has_own_skin or has_own_head or hidings.has("head"),
		"mask_eyes":
		(
			show_only_wearables
			or has_own_skin
			or has_own_head
			or hidings.has("eyes")
			or hidings.has("head")
		),
		"mask_eyebrows":
		(
			show_only_wearables
			or has_own_skin
			or has_own_head
			or hidings.has("eyebrows")
			or hidings.has("head")
		),
		"mask_mouth":
		(
			show_only_wearables
			or has_own_skin
			or has_own_head
			or hidings.has("mouth")
			or hidings.has("head")
		),
	}

	# Final computation of hidings
	hidings = Dictionary()
	hidings.merge(base_bodyshape_hidings)
	for category in curated_wearables.hidden_categories:
		hidings[WEARABLE_NAME_PREFIX + category] = true

	for child in body_shape_skeleton_3d.get_children():
		var should_hide = false
		for ends_with in hidings:
			if child.name.ends_with(ends_with) and hidings[ends_with]:
				should_hide = true
				break

		if should_hide:
			child.hide()

	for child in body_shape_skeleton_3d.get_children():
		if child.visible and child is MeshInstance3D:
			# Shallow-duplicate the Mesh so per-avatar surface_set_material calls
			# don't leak across avatars; materials referenced inside stay shared.
			child.mesh = child.mesh.duplicate_deep(Resource.DEEP_DUPLICATE_NONE)

	apply_toon_material(body_shape_skeleton_3d)
	for child in body_shape_skeleton_3d.get_children():
		apply_toon_material(child)

	apply_color_and_facial()

	# For show_only_wearables, reset skeleton to T-pose so wearable doesn't animate
	if show_only_wearables:
		for i in range(body_shape_skeleton_3d.get_bone_count()):
			body_shape_skeleton_3d.reset_bone_pose(i)

	body_shape_skeleton_3d.visible = true
	finish_loading = true
	# Emotes - get from cached emote scenes
	for emote_urn in avatar_data.get_emotes():
		if not emote_urn.begins_with("urn"):
			# Default (utility emotes)
			continue

		var emote = Global.content_provider.get_wearable(emote_urn)
		if emote == null:
			continue
		var file_hash = Wearables.get_item_main_file_hash(emote, avatar_data.get_body_shape())
		if file_hash.is_empty():
			continue
		# Use emote_loader from emote_controller to get the cached emote (threaded loading)
		var obj = await emote_controller.emote_loader.async_get_emote_gltf(file_hash)
		if obj != null:
			emote_controller.load_emote_from_dcl_emote_gltf(emote_urn, obj, file_hash)

	emote_controller.clean_unused_emotes()

	# Refresh LOD-related state since meshes were re-created.
	_mesh_lod_visibility_captured = false
	if _impostor_layer >= 0 and not _impostor_layer_is_overflow and Global.avatars != null:
		Global.avatars.invalidate_impostor_texture(get_instance_id(), _get_impostor_cache_key())
		ImpostorCapturer.request_capture(self)

	avatar_ready = true
	avatar_loaded.emit()

	# Play any pending expression trigger that was set before avatar was ready
	if has_meta("pending_expression_trigger"):
		var pending_emote = get_meta("pending_expression_trigger")
		remove_meta("pending_expression_trigger")
		_async_play_expression_trigger(pending_emote)


func apply_color_and_facial():
	for child in body_shape_skeleton_3d.get_children():
		if child.visible and child is MeshInstance3D:
			for i in range(child.get_surface_override_material_count()):
				var mat_name = child.mesh.get("surface_" + str(i) + "/name").to_lower()
				var is_skin: bool = mat_name.find("skin") != -1
				var is_hair: bool = mat_name.find("hair") != -1
				var material = child.mesh.surface_get_material(i)

				if material is ShaderMaterial and (is_skin or is_hair):
					# Cached materials are shared between avatars. Clone before
					# writing the per-avatar tint so it doesn't leak.
					material = material.duplicate()
					child.mesh.surface_set_material(i, material)
					if is_skin:
						material.set_shader_parameter("albedo_color", avatar_data.get_skin_color())
					else:
						material.set_shader_parameter("albedo_color", avatar_data.get_hair_color())
				elif material is StandardMaterial3D:
					material.metallic = 0
					material.metallic_specular = 0
					if is_skin:
						material.albedo_color = avatar_data.get_skin_color()
					elif is_hair:
						material.roughness = 1
						material.albedo_color = avatar_data.get_hair_color()

	var eyes = wearables_by_category.get(Wearables.Categories.EYES)
	var eyebrows = wearables_by_category.get(Wearables.Categories.EYEBROWS)
	var mouth = wearables_by_category.get(Wearables.Categories.MOUTH)
	self.apply_facial_features_to_meshes(eyes, eyebrows, mouth)


func apply_facial_features_to_meshes(wearable_eyes, wearable_eyebrows, wearable_mouth):
	var body_shape_id := avatar_data.get_body_shape()
	var eyes = Wearables.get_wearable_facial_hashes(wearable_eyes, body_shape_id)
	var eyebrows = Wearables.get_wearable_facial_hashes(wearable_eyebrows, body_shape_id)
	var mouth = Wearables.get_wearable_facial_hashes(wearable_mouth, body_shape_id)

	for child in body_shape_skeleton_3d.get_children():
		if not child.visible or not child is MeshInstance3D:
			continue

		if child.name.ends_with("mask_eyes"):
			if not eyes.is_empty():
				apply_texture_and_mask(child, eyes, avatar_data.get_eyes_color(), Color.WHITE)
			else:
				child.hide()
		elif child.name.ends_with("mask_eyebrows"):
			if not eyebrows.is_empty():
				apply_texture_and_mask(child, eyebrows, avatar_data.get_hair_color(), Color.BLACK)
			else:
				child.hide()
		elif child.name.ends_with("mask_mouth"):
			if not mouth.is_empty():
				apply_texture_and_mask(child, mouth, avatar_data.get_skin_color(), Color.BLACK)
			else:
				child.hide()


func apply_texture_and_mask(mesh: MeshInstance3D, textures: Array, color: Color, mask_color: Color):
	var current_material = mask_material.duplicate()
	current_material.set_shader_parameter(
		"base_texture", Global.content_provider.get_texture_from_hash(textures[0])
	)
	current_material.set_shader_parameter("material_color", color)
	current_material.set_shader_parameter("mask_color", mask_color)

	if textures.size() > 1:
		current_material.set_shader_parameter(
			"mask_texture", Global.content_provider.get_texture_from_hash(textures[1])
		)
	else:
		current_material.set_shader_parameter("mask_texture", null)

	mesh.mesh.surface_set_material(0, current_material)


func _maybe_update_lod() -> void:
	if not avatar_ready or is_local_player or hidden:
		return
	if Engine.get_frames_drawn() % AvatarImpostorConfig.DISTANCE_CHECK_PERIOD_FRAMES != _lod_phase:
		return
	_update_lod()


func _update_lod() -> void:
	# Bypass: when the impostor system is globally disabled, force FULL so
	# the benchmark's OFF phase measures full-mesh rendering with zero
	# impostor capture/eviction work.
	var config = Global.get_config()
	if config != null and not config.avatar_impostors_enabled:
		if _lod_state != LODState.FULL:
			_apply_lod_state(LODState.FULL, 1.0, 0.0, 0.0, 0.0)
		return

	# Defense for any path that hides the avatar without going through the
	# helpers (set_hidden / modifier area). Don't change LOD state — the
	# avatar's mesh is already invisible via the parent hide(); we just need
	# the impostor slot off.
	if not visible:
		_hide_impostor_render()
		return

	var camera = get_viewport().get_camera_3d()
	if camera == null:
		return

	var dist: float = camera.global_position.distance_to(global_position)

	var new_state: int = _lod_state
	var dither_alpha: float = 1.0
	var fade_alpha: float = 0.0
	var tint_strength: float = 0.0

	if dist < AvatarImpostorConfig.MID_RANGE_NEAR:
		new_state = LODState.FULL
	elif dist < AvatarImpostorConfig.DISTANCE_NEAR:
		new_state = LODState.MID
	elif dist < AvatarImpostorConfig.DISTANCE_FAR:
		new_state = LODState.CROSSFADE
		var span: float = AvatarImpostorConfig.DISTANCE_FAR - AvatarImpostorConfig.DISTANCE_NEAR
		var t: float = (dist - AvatarImpostorConfig.DISTANCE_NEAR) / span
		dither_alpha = 1.0 - t
		fade_alpha = t
	else:
		new_state = LODState.FAR
		dither_alpha = 0.0
		fade_alpha = 1.0
		var tint_span: float = (
			AvatarImpostorConfig.TINT_FULL_DISTANCE - AvatarImpostorConfig.DISTANCE_FAR
		)
		tint_strength = clamp((dist - AvatarImpostorConfig.DISTANCE_FAR) / tint_span, 0.0, 1.0)

	# Concurrency cap: only the closest N stay FULL, the next M stay
	# MID/CROSSFADE, the rest are demoted to FAR. Applies to emoters too — mass
	# emote scenarios shouldn't bypass the budget.
	if _lod_rank_cap > new_state:
		new_state = _lod_rank_cap
		if new_state == LODState.FAR:
			dither_alpha = 0.0
			fade_alpha = 1.0
		else:
			dither_alpha = 1.0
			fade_alpha = 0.0

	_apply_lod_state(new_state, dither_alpha, fade_alpha, tint_strength, dist)
	_apply_off_frustum_anim_freeze()


func _apply_lod_state(
	state: int, dither_alpha: float, fade_alpha: float, tint_strength: float, distance: float
) -> void:
	var state_changed: bool = state != _lod_state
	var prev_state: int = _lod_state
	_lod_state = state

	AvatarLODHelpers.set_dither_alpha(self, dither_alpha)

	if _off_frustum:
		# Off-frustum: avatar is invisible to the camera. Drop the slot
		# entirely so its multimesh instance and (if owned) layer return to
		# the pool for an in-frustum avatar to use. Re-entry rehydrates from
		# the disk cache — fast and gap-free.
		if _impostor_layer >= 0 and Global.avatars != null:
			Global.avatars.clear_impostor(get_instance_id())
			_impostor_layer = -1
			_impostor_layer_is_overflow = false
	elif state == LODState.FAR or state == LODState.CROSSFADE:
		_ensure_impostor_layer(distance)
		if _impostor_layer >= 0 and Global.avatars != null:
			Global.avatars.set_impostor_state(
				get_instance_id(), fade_alpha, tint_strength, distance
			)
	elif _impostor_layer >= 0 and Global.avatars != null:
		# Don't release the slot on every LOD oscillation — re-allocating would
		# trigger a new capture every time the camera swings between FAR and
		# MID/FULL. Keep the slot, just hide the multimesh instance with
		# fade_alpha=0. The slot is freed when the avatar is removed entirely
		# (AvatarScene::remove_avatar).
		Global.avatars.set_impostor_state(get_instance_id(), 0.0, 0.0, distance)

	if state_changed:
		_on_lod_state_changed(state, prev_state)


func _ensure_impostor_layer(distance: float) -> void:
	if Global.avatars == null:
		return
	# Always call through. Rust handles the dispatch:
	#   * No existing slot -> fresh allocation (real with cached texture if
	#     available, or pure overflow when allow_overflow is true).
	#   * Existing slot -> toggle render mode in place, keeping the layer warm.
	# No clear-then-realloc dance, so a camera that briefly turns off-frustum
	# doesn't trigger a recapture or flicker the borrowed silhouette.
	_impostor_layer = Global.avatars.request_impostor_layer(
		get_instance_id(), self, distance, _use_overflow_impostor, _get_impostor_cache_key()
	)
	if _impostor_layer < 0:
		return
	_impostor_layer_is_overflow = _use_overflow_impostor
	# Capture only when the slot owns a real layer that hasn't been filled yet.
	if Global.avatars.impostor_needs_capture(get_instance_id()):
		ImpostorCapturer.request_capture(self)


func _release_impostor() -> void:
	if _impostor_layer < 0:
		return
	if Global.avatars != null:
		Global.avatars.clear_impostor(get_instance_id())
	_impostor_layer = -1


func _on_lod_state_changed(new_state: int, _prev_state: int) -> void:
	match new_state:
		LODState.FULL:
			AvatarLODHelpers.set_meshes_visible(self, true)
			AvatarLODHelpers.set_animation_active(self, true)
			AvatarLODHelpers.set_animation_speed(self, 1.0)
			AvatarLODHelpers.set_animation_throttle(self, false)
			AvatarLODHelpers.set_particles_visible(self, true)
			AvatarLODHelpers.set_click_active(self, true)
			_apply_nickname_visibility()
		LODState.MID:
			AvatarLODHelpers.set_meshes_visible(self, true)
			AvatarLODHelpers.set_animation_active(self, true)
			AvatarLODHelpers.set_animation_speed(self, 1.0)
			AvatarLODHelpers.set_animation_throttle(self, true)
			AvatarLODHelpers.set_particles_visible(self, false)
			AvatarLODHelpers.set_click_active(self, false)
			_apply_nickname_visibility()
		LODState.CROSSFADE:
			AvatarLODHelpers.set_meshes_visible(self, true)
			AvatarLODHelpers.set_animation_active(self, true)
			AvatarLODHelpers.set_animation_speed(self, 1.0)
			AvatarLODHelpers.set_animation_throttle(self, true)
			AvatarLODHelpers.set_particles_visible(self, false)
			AvatarLODHelpers.set_click_active(self, false)
			_apply_nickname_visibility()
		LODState.FAR:
			AvatarLODHelpers.set_meshes_visible(self, false)
			AvatarLODHelpers.set_animation_active(self, false)
			AvatarLODHelpers.set_animation_speed(self, 1.0)
			AvatarLODHelpers.set_animation_throttle(self, false)
			AvatarLODHelpers.set_particles_visible(self, false)
			AvatarLODHelpers.set_click_active(self, false)
			_apply_nickname_visibility()


# Freeze the AnimationTree when off-frustum, regardless of LOD state. The mesh
# is still visible logically (so re-entry doesn't pop), but Godot's GPU
# frustum cull skips drawing it and the AnimationTree pauses CPU-side. On
# re-entry we restore the state-driven anim setup and advance the tree by the
# wall-clock time we were paused, so emote phase matches what it would have
# been had we not paused — single recompute, no frame-by-frame catch-up.
func _apply_off_frustum_anim_freeze() -> void:
	if animation_tree == null:
		return
	if _off_frustum:
		if not _anim_frozen_off_frustum:
			animation_tree.active = false
			_anim_throttle_active = false
			_anim_freeze_start_ms = Time.get_ticks_msec()
			_anim_frozen_off_frustum = true
	elif _anim_frozen_off_frustum:
		_anim_frozen_off_frustum = false
		var elapsed_s: float = (Time.get_ticks_msec() - _anim_freeze_start_ms) / 1000.0
		match _lod_state:
			LODState.FULL:
				AvatarLODHelpers.set_animation_active(self, true)
				AvatarLODHelpers.set_animation_throttle(self, false)
			LODState.MID, LODState.CROSSFADE:
				AvatarLODHelpers.set_animation_active(self, true)
				AvatarLODHelpers.set_animation_throttle(self, true)
			LODState.FAR:
				AvatarLODHelpers.set_animation_active(self, false)
				AvatarLODHelpers.set_animation_throttle(self, false)
		if animation_tree.active and elapsed_s > 0.0:
			animation_tree.advance(elapsed_s)


func _tick_animation_throttle(delta: float) -> void:
	if not _anim_throttle_active or animation_tree == null:
		return
	_anim_throttle_acc += delta
	_anim_throttle_counter += 1
	if _anim_throttle_counter >= AvatarImpostorConfig.MID_ANIM_ADVANCE_EVERY_N_FRAMES:
		animation_tree.advance(_anim_throttle_acc)
		_anim_throttle_acc = 0.0
		_anim_throttle_counter = 0


func _process(delta):
	# TODO: maybe a gdext crate bug? when process implement the INode3D, super(delta) doesn't work :/
	self.process(delta)

	if nickname_viewport.size != Vector2i(nickname_ui.size):
		nickname_viewport.size = Vector2i(nickname_ui.size)
		_request_nickname_redraw()

	_maybe_update_lod()
	_tick_animation_throttle(delta)

	if _lod_state == LODState.FAR:
		return

	# Skip animations for show_only_wearables avatars (no body to animate)
	if is_avatar_shape and avatar_data != null and avatar_data.get_show_only_wearables():
		animation_tree.active = false
		return

	# Ensure animation tree is active for normal avatars
	if not animation_tree.active:
		animation_tree.active = true

	# #b18: `is_grounded` guard suppresses the all-false condition window at the
	# jump apex (rise/fall ±0.3 deadband) so Idle doesn't leak in mid-air.
	var self_idle = (
		self.is_grounded && !self.jog && !self.walk && !self.run && !self.rise && !self.fall
	)
	emote_controller.process(self_idle)

	var is_emoting = self_idle && emote_controller.is_playing()
	if is_local_player:
		Global.comms.set_emoting(is_emoting)

	animation_tree.set("parameters/conditions/idle", self_idle)
	animation_tree.set("parameters/conditions/emote", emote_controller.playing_single)
	animation_tree.set("parameters/conditions/nemote", not emote_controller.playing_single)
	animation_tree.set("parameters/conditions/emix", emote_controller.playing_mixed)
	animation_tree.set("parameters/conditions/nemix", not emote_controller.playing_mixed)

	animation_tree.set("parameters/conditions/run", self.run)
	animation_tree.set("parameters/conditions/jog", self.jog)
	animation_tree.set("parameters/conditions/walk", self.walk)

	animation_tree.set("parameters/conditions/rise", self.rise)
	animation_tree.set("parameters/conditions/fall", self.fall)
	animation_tree.set("parameters/conditions/land", self.land)
	# #b3: nfall reads is_grounded directly (not `land`). `land` is a short pulse
	# locally (in_grace_time) and was previously overridden to is_grounded for
	# remotes, causing asymmetric behavior. is_grounded is the same shape on
	# both sides, and fall's 1-2 frame deadband at apex is still avoided.
	animation_tree.set("parameters/conditions/nfall", self.is_grounded)

	# Rising-edge detection for one-frame AnimationTree condition pulses.
	var jump_rising_edge: bool = self.jump_count > _last_jump_count and self.jump_count >= 2
	# #b2: on first observation of this avatar (local or remote) suppress the
	# rising edge so we don't retroactively play SFX for state that happened
	# before we started watching.
	if _jump_count_sync_pending:
		jump_rising_edge = false
		_jump_count_sync_pending = false
	_last_jump_count = self.jump_count
	# #b16: start_glide is sustained for the whole OPENING window, not a one-
	# frame edge. An edge pulse is lost when the AnimationTree sits in a state
	# without an outgoing `start_glide` transition (Jump_Mid, Run_Jump_Mid,
	# Idle, Jump_Start, …), leaving the avatar stuck in Jump_Fall while
	# glide_state == GLIDING. Holding the condition for the full 0.5s window
	# gives the state machine time to pass through a source state.
	var glide_opening: bool = self.glide_state == 1
	var gliding_now: bool = self.glide_state == 1 or self.glide_state == 2

	animation_tree.set("parameters/conditions/double_jump", jump_rising_edge)
	animation_tree.set("parameters/conditions/start_glide", glide_opening)
	animation_tree.set("parameters/conditions/gliding", gliding_now)
	animation_tree.set("parameters/conditions/ngliding", not gliding_now)

	var glide_moving: bool = self.walk or self.jog or self.run
	var glide_forward_target: float = 1.0 if glide_moving else 0.0
	_glide_forward_blend = move_toward(_glide_forward_blend, glide_forward_target, delta * 4.0)
	animation_tree.set("parameters/Gliding_Idle/Blend2/blend_amount", _glide_forward_blend)

	if jump_rising_edge:
		audio_player_double_jump.play()

	_update_glider_prop()


# Toggles GliderProp visibility based on glide_state transitions. The prop is
# a persistent child (rotated 180° Y to compensate the Unity→Godot axis flip)
# so audio and AnimationPlayer stay warm across glide cycles.
func _update_glider_prop() -> void:
	if is_avatar_shape:
		if glider_prop.visible:
			glider_prop.visible = false
		_prop_last_glide_state = self.glide_state
		_prop_sync_pending = false
		return

	var curr_state: int = self.glide_state

	# #b1/#b12: first tick — adopt whatever state came in without firing open/close
	# SFX, so a remote seen mid-glide actually shows wings and the idle loop.
	if _prop_sync_pending:
		_prop_sync_pending = false
		_prop_last_glide_state = curr_state
		if curr_state == 1 or curr_state == 2:
			glider_prop.visible = true
			_glider_close_initiated = false
			# Jump straight to Glider_Idle; no Start/Open sfx for state we joined into.
			_play_glider_clip_if_different("Glider_Idle")
			if curr_state == 1:
				_play_glider_audio("AudioPlayer_Idle")
			else:
				_play_glider_audio("AudioPlayer_Idle")
		elif curr_state == 3:
			# Joined during CLOSING: show prop, let the existing CLOSING→CLOSED
			# transition scheduled by the next tick hide it naturally.
			glider_prop.visible = true
			_glider_close_initiated = true
		return

	var prev_state: int = _prop_last_glide_state
	_prop_last_glide_state = curr_state

	if prev_state == 0 and curr_state == 1:
		glider_prop.visible = true
		_glider_close_initiated = false
		_play_glider_audio("AudioPlayer_Open")
		_play_glider_audio("AudioPlayer_Idle")
		_play_glider_clip("Glider_Start")
	elif (prev_state == 1 or prev_state == 2) and curr_state == 3:
		# Start close immediately on CLOSING edge — don't wait for the FSM to
		# reach CLOSED (0.15s later) or the user sees wings-open mid-close.
		_glider_close_initiated = true
		_stop_glider_audio("AudioPlayer_Idle")
		_play_glider_audio("AudioPlayer_Close")
		_play_glider_clip("Glider_End")
	elif prev_state == 3 and curr_state == 0 and glider_prop.visible:
		_schedule_glider_hide()
	elif curr_state == 0 and glider_prop.visible and not _glider_close_initiated:
		# Fallback: state jumped directly to CLOSED without passing CLOSING.
		_glider_close_initiated = true
		_stop_glider_audio("AudioPlayer_Idle")
		_play_glider_audio("AudioPlayer_Close")
		_play_glider_clip("Glider_End")
		_schedule_glider_hide()

	if curr_state == 2 and glider_prop.visible:
		var glider_moving: bool = self.walk or self.jog or self.run
		var glider_clip: String = "Glider_Forward" if glider_moving else "Glider_Idle"
		_play_glider_clip_if_different(glider_clip, 0.25)


func _schedule_glider_hide() -> void:
	var tree := get_tree()
	if tree != null:
		var timer := tree.create_timer(0.25)
		timer.timeout.connect(_hide_glider_prop)
	else:
		_hide_glider_prop()


func _hide_glider_prop() -> void:
	# Skip the hide if the user re-opened glide during the fade-out window.
	if self.glide_state == 0:
		glider_prop.visible = false


func _play_glider_audio(node_name: String) -> void:
	var player := glider_prop.get_node_or_null(node_name) as AudioStreamPlayer3D
	if player != null:
		player.play()


func _stop_glider_audio(node_name: String) -> void:
	var player := glider_prop.get_node_or_null(node_name) as AudioStreamPlayer3D
	if player != null:
		player.stop()


func _play_glider_clip(clip: String) -> void:
	var ap := glider_prop.get_node_or_null("AnimationPlayer") as AnimationPlayer
	if ap == null or not ap.has_animation(clip):
		return
	ap.play(clip)


func _play_glider_clip_if_different(clip: String, blend_time: float = -1.0) -> void:
	var ap := glider_prop.get_node_or_null("AnimationPlayer") as AnimationPlayer
	if ap == null or not ap.has_animation(clip):
		return
	if ap.current_animation != clip:
		ap.play(clip, blend_time)


func spawn_voice_channel(sample_rate, _num_channels, _samples_per_channel):
	voice_chat_audio_player = AudioStreamPlayer.new()
	voice_chat_audio_player.set_bus("VoiceChat")
	voice_chat_audio_player_gen = AudioStreamGenerator.new()

	voice_chat_audio_player.set_stream(voice_chat_audio_player_gen)
	voice_chat_audio_player_gen.mix_rate = sample_rate
	add_child(voice_chat_audio_player)
	voice_chat_audio_player.play()


func push_voice_frame(frame):
	if not voice_chat_audio_player.playing:
		voice_chat_audio_player.play()

	voice_chat_audio_player.get_stream_playback().push_buffer(frame)
	if not nickname_ui.mic_enabled:
		nickname_ui.mic_enabled = true
		_request_nickname_redraw()
	timer_hide_mic.start()


func activate_attach_points():
	generate_attach_points = true
	_add_attach_points()


func _add_attach_points():
	if not generate_attach_points:
		return

	if body_shape_skeleton_3d == null:
		return

	anchor_bone_idx.clear()
	for anchor_id in _ANCHOR_BONE_NAMES:
		var idx := body_shape_skeleton_3d.find_bone(_ANCHOR_BONE_NAMES[anchor_id])
		if idx != -1:
			anchor_bone_idx[anchor_id] = idx
	# Prime the cache so the first frame after activation doesn't read default
	# Transform3D() zeros.
	_attach_point_skeleton_updated()


func _attach_point_skeleton_updated():
	for anchor_id in anchor_bone_idx:
		var t := body_shape_skeleton_3d.get_bone_global_pose(anchor_bone_idx[anchor_id])
		t.basis = t.basis.scaled(100.0 * Vector3.ONE)
		anchor_transform[anchor_id] = t


# Returns the world-space transform of an avatar anchor point for an
# AvatarAttach component. anchor_point_id matches the SDK proto enum
# AvatarAnchorPointType (see avatar_attach.proto). Unknown / unresolved ids
# fall back to the avatar root so attached entities stay glued to the avatar
# instead of teleporting to world origin.
func get_anchor_point_global_transform(anchor_point_id: int) -> Transform3D:
	# Post-rotate 180° around local Y to align with the Unity reference client.
	# Godot's GLTF import flips the skeleton coordinate basis vs Unity, leaving
	# bone-derived +X / +Z inverted relative to the Decentraland SDK / Unity
	# convention; +Y stays correct. Applies to NAME_TAG too because nickname_quad
	# is parented under a BoneAttachment3D on Avatar_Head, so it inherits the
	# same discrepancy.
	if anchor_point_id == 1:  # AAPT_NAME_TAG
		return nickname_quad.global_transform.rotated_local(Vector3.UP, PI)
	if body_shape_skeleton_3d != null and anchor_transform.has(anchor_point_id):
		var cached: Transform3D = anchor_transform[anchor_point_id]
		var t := body_shape_skeleton_3d.global_transform * cached
		return t.rotated_local(Vector3.UP, PI)
	return global_transform


func _on_timer_hide_mic_timeout():
	nickname_ui.mic_enabled = false
	_request_nickname_redraw()


func set_client_version(version: String):
	nickname_ui.client_version = version
	_request_nickname_redraw()


func set_room_debug(info: String):
	nickname_ui.room_debug = info
	_request_nickname_redraw()


func _play_emote_audio(file_hash: String):
	emote_controller.play_emote_audio(file_hash)


func async_play_emote(emote_urn: String):
	await emote_controller.async_play_emote(emote_urn)


## Register scene emote content info for later retrieval.
## Called from Rust before async_play_emote for scene emotes.
func register_scene_emote_content(
	scene_id: String, base_url: String, glb_hash: String, audio_hash: String
) -> void:
	if not _scene_emote_registry.has(scene_id):
		_scene_emote_registry[scene_id] = {"base_url": base_url, "emotes": {}}
	_scene_emote_registry[scene_id]["emotes"][glb_hash] = audio_hash


## Get scene emote content info for loading.
## Returns {base_url, audio_hash} or fallback to realm URL for remote players.
func get_scene_emote_info(scene_id: String, glb_hash: String) -> Dictionary:
	if _scene_emote_registry.has(scene_id):
		var scene_data = _scene_emote_registry[scene_id]
		if scene_data["emotes"].has(glb_hash):
			return {
				"base_url": scene_data["base_url"], "audio_hash": scene_data["emotes"][glb_hash]
			}
	# Fallback for remote players - use realm URL (audio won't be available)
	return {"base_url": Global.realm.content_base_url, "audio_hash": ""}


## Play emote triggered by AvatarShape's expression_trigger_id field.
## Supports: default emotes (e.g. "wave"), URN emotes, and local scene emotes (.glb/.gltf)
func _async_play_expression_trigger(emote_id: String) -> void:
	if emote_id.is_empty():
		return

	# URN emotes (wearable emotes and scene emotes)
	if emote_id.begins_with("urn:"):
		await async_play_emote(emote_id)
	# Default emotes (wave, clap, dance, etc.) - play via emote controller
	else:
		await emote_controller.async_play_emote(emote_id)
