class_name AvatarEmoteController
extends RefCounted


class EmoteSceneUrn:
	var base_url: String
	var glb_hash: String
	var audio_hash: String
	var looping: bool

	func _init(emote_urn: String):
		var urn = emote_urn.split(":")
		# urn:decentraland:off-chain:scene-emote:{fileHash}-{looping}
		if urn.size() != 5:
			return

		var content = urn[4].split("-")
		if urn[4].begins_with("b64"):
			glb_hash = content[0] + "-" + content[1]
			looping = content[2] == "true"
		else:
			glb_hash = content[0]
			looping = content[1] == "true"

		# TODO: define from the urn
		base_url = Global.realm.content_base_url


class EmoteItemData:
	extends RefCounted
	var urn: String = ""
	var default_anim_name: String = ""
	var prop_anim_name: String = ""
	var file_hash: String = ""
	var armature_prop: Node3D = null
	var prop_animation_player: AnimationPlayer = null

	var from_scene: bool
	var looping: bool

	func _init(
		_urn: String,
		_default_anim_name: String,
		_prop_anim_name: String,
		_file_hash: String,
		_armature_prop: Node3D,
		_prop_animation_player: AnimationPlayer = null
	):
		urn = _urn
		default_anim_name = _default_anim_name
		prop_anim_name = _prop_anim_name
		file_hash = _file_hash
		armature_prop = _armature_prop
		prop_animation_player = _prop_animation_player


# Cooldown to prevent rapid emote spam that can crash the animation system
const EMOTE_COOLDOWN_SECONDS: float = 0.5
const MAX_DEFERRED_RETRIES: int = 3

var loaded_emotes_by_urn: Dictionary

var playing_single: bool = false
var playing_mixed: bool = false
var playing_loop: bool = false

# Reference by parent avatar
var avatar: Avatar = null
var animation_player: AnimationPlayer
var animation_tree: AnimationTree

var emotes_animation_library: AnimationLibrary
var idle_anim: Animation

var animation_single_emote_node: AnimationNodeAnimation
var animation_mix_emote_node: AnimationNodeBlendTree

# Guard to prevent concurrent modifications to animation system
var _is_modifying_animations: bool = false
var _queued_emote_urn: String = ""

var _last_emote_time: float = 0.0

# Lock to prevent concurrent async emote loading
var _is_loading_emote: bool = false

# Track prop visibility nodes that need to be hidden on idle
# This avoids modifying idle_anim at runtime which can crash the mixer
var _prop_armature_names: Array[String] = []

var _deferred_retry_count: int = 0


func _init(_avatar: Avatar, _animation_player: AnimationPlayer, _animation_tree: AnimationTree):
	# Core dependencies from avatar
	avatar = _avatar
	animation_player = _animation_player
	animation_tree = _animation_tree

	# TODO: this is a workaround because "Local to scene" is not working when
	#	is selected in the independent nodes.
	#	Maybe related to https://github.com/godotengine/godot/issues/82421
	animation_tree.tree_root = animation_tree.tree_root.duplicate(true)

	# Direct dependencies
	animation_single_emote_node = animation_tree.tree_root.get_node("Emote")
	animation_mix_emote_node = animation_tree.tree_root.get_node("Emote_Mix")
	assert(animation_mix_emote_node.get_node("A") != null)
	assert(animation_mix_emote_node.get_node("B") != null)

	# Set safe default animations to avoid errors when AnimationTree processes
	# The default "AFK" in tscn may not exist in the animation player
	animation_single_emote_node.animation = "idle/Anim"
	animation_mix_emote_node.get_node("A").animation = "idle/Anim"
	animation_mix_emote_node.get_node("B").animation = "idle/Anim"

	# Idle Anim Duplication (so it makes mutable and non-shared-reference)
	var idle_animation_library = animation_player.get_animation_library("idle")
	idle_animation_library = idle_animation_library.duplicate(false)
	idle_anim = idle_animation_library.get_animation("Anim").duplicate()
	idle_animation_library.remove_animation("Anim")
	idle_animation_library.add_animation("Anim", idle_anim)
	animation_player.remove_animation_library("idle")
	animation_player.add_animation_library("idle", idle_animation_library)

	# Emote library
	if not animation_player.has_animation_library("emotes"):
		emotes_animation_library = AnimationLibrary.new()
		animation_player.add_animation_library("emotes", emotes_animation_library)
	else:
		emotes_animation_library = animation_player.get_animation_library("emotes")


func stop_emote():
	playing_single = false
	playing_mixed = false
	playing_loop = false


func play_emote(id: String):
	# If animation system is being modified, queue this request
	if _is_modifying_animations:
		_queued_emote_urn = id
		return

	# Ensure animation tree is active before playing
	if not animation_tree.active:
		animation_tree.active = true

	var triggered: bool = false
	if not id.begins_with("urn"):
		# Check if it's a utility action (local) or base emote (remote)
		if Emotes.is_emote_utility(id):
			triggered = _play_utility_emote(id)
		elif Emotes.is_emote_default(id):
			# Base emotes are loaded remotely, play via URN
			var urn = Emotes.get_base_emote_urn(id)
			triggered = _play_loaded_emote(urn)
		else:
			printerr("Unknown emote: %s" % id)
	else:
		triggered = _play_loaded_emote(id)

	if triggered:
		avatar.call_deferred("emit_signal", "emote_triggered", id, playing_loop)


func _play_utility_emote(utility_emote_id: String) -> bool:
	# Utility emotes are local animations from default_actions library
	var anim_name = "default_actions/" + utility_emote_id
	if not animation_player.has_animation(anim_name):
		printerr(
			(
				"Utility emote %s not found from player '%s'"
				% [utility_emote_id, avatar.get_avatar_name()]
			)
		)
		return false

	animation_single_emote_node.animation = anim_name
	var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
	var cur_node = pb.get_current_node()
	if cur_node == "Emote":
		pb.start("Emote", true)
	else:
		pb.travel("Emote")

	playing_single = true
	playing_mixed = false
	playing_loop = false
	return true


func _play_loaded_emote(emote_urn: String) -> bool:
	if not _has_emote(emote_urn):
		printerr("Emote %s not found from player '%s'" % [emote_urn, avatar.get_avatar_name()])
		return false

	var emote_item_data: EmoteItemData = loaded_emotes_by_urn[emote_urn]

	# Validate animation name exists
	if emote_item_data.default_anim_name.is_empty():
		printerr("Emote %s has no animation" % emote_urn)
		return false

	if emote_item_data.from_scene:
		playing_loop = emote_item_data.looping
	else:
		var emote_data = Global.content_provider.get_wearable(emote_item_data.urn)
		if emote_data == null:
			return false
		playing_loop = emote_data.get_emote_loop()

	# Reset avatar state before playing new emote
	_hide_all_props()
	_reset_skeleton_to_rest_pose()

	playing_single = true
	playing_mixed = false

	var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")

	# Play merged animation (avatar + prop tracks combined) through AnimationTree
	var anim_path = "emotes/" + emote_item_data.default_anim_name
	if not animation_player.has_animation(anim_path):
		printerr("Animation not found in player: %s" % anim_path)
		return false

	animation_single_emote_node.animation = anim_path

	# Ensure animation tree is active
	if not animation_tree.active:
		animation_tree.active = true

	# Ensure state machine is initialized - if current_node is empty, it hasn't started
	var cur_state = pb.get_current_node()
	if cur_state.is_empty():
		pb.start("Idle", true)
		# Need to wait a frame for state machine to initialize
		# Use call_deferred to retry the play
		_deferred_play_emote.call_deferred(emote_urn)
		return true

	# Set the emote condition BEFORE travel - the transition requires this condition
	# (avatar.gd's _process() also sets this, but too late for immediate travel)
	animation_tree.set("parameters/conditions/emote", true)
	animation_tree.set("parameters/conditions/nemote", false)

	# Use travel() to follow state machine transitions, then start() to restart if already there
	if pb.get_current_node() == "Emote":
		pb.start("Emote", true)
	else:
		pb.travel("Emote")

	# Reset retry counter on success
	_deferred_retry_count = 0
	return true


func _deferred_play_emote(emote_urn: String):
	# Called after state machine is initialized, retry the play
	_deferred_retry_count += 1
	if _deferred_retry_count > MAX_DEFERRED_RETRIES:
		_deferred_retry_count = 0
		_force_play_emote(emote_urn)
		return
	play_emote(emote_urn)


func _force_play_emote(emote_urn: String):
	# Force play when state machine won't initialize (e.g., avatar hidden in backpack)
	if not _has_emote(emote_urn):
		return

	var emote_item_data: EmoteItemData = loaded_emotes_by_urn[emote_urn]
	if emote_item_data.default_anim_name.is_empty():
		return

	var anim_path = "emotes/" + emote_item_data.default_anim_name
	if not animation_player.has_animation(anim_path):
		return

	# Reset state and play directly via AnimationPlayer
	_hide_all_props()
	_reset_skeleton_to_rest_pose()

	# Set up the animation node
	animation_single_emote_node.animation = anim_path

	# Set the emote conditions for the state machine
	animation_tree.set("parameters/conditions/emote", true)
	animation_tree.set("parameters/conditions/nemote", false)

	# Force the state machine to the Emote state
	var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
	pb.start("Emote", true)

	playing_single = true
	playing_mixed = false
	playing_loop = false


func _hide_all_props():
	# Hide all prop armatures to ensure clean state before playing new emote
	if not is_instance_valid(avatar):
		return

	# Use tracked prop names instead of iterating all children
	for prop_name in _prop_armature_names:
		var prop = avatar.get_node_or_null(NodePath(prop_name))
		if prop != null and is_instance_valid(prop):
			prop.hide()

	# Also hide any legacy props (fallback for props added before tracking)
	for child in avatar.get_children():
		if is_instance_valid(child) and child.name.begins_with("Armature_Prop"):
			child.hide()


func _reset_skeleton_to_rest_pose():
	# Reset skeleton bones to rest pose to clear any transforms from previous emotes
	# (e.g., head scale from "head explode" emote)
	# Note: Only reset if avatar is valid and not being freed
	if not is_instance_valid(avatar):
		return

	var armature = avatar.get_node_or_null("Armature")
	if armature == null or not is_instance_valid(armature):
		return

	var skeleton = armature.get_node_or_null("Skeleton3D")
	if skeleton == null or not is_instance_valid(skeleton):
		return

	for i in range(skeleton.get_bone_count()):
		skeleton.reset_bone_pose(i)


func async_play_emote(emote_id_or_urn: String) -> void:
	# Cooldown check to prevent rapid emote spam
	var current_time = Time.get_ticks_msec() / 1000.0
	if current_time - _last_emote_time < EMOTE_COOLDOWN_SECONDS:
		return
	_last_emote_time = current_time

	# Prevent concurrent async loading operations
	if _is_loading_emote:
		return

	var emote_urn: String = emote_id_or_urn

	# Handle non-URN emote IDs
	if not emote_id_or_urn.begins_with("urn"):
		# Utility emotes are local, play directly
		if Emotes.is_emote_utility(emote_id_or_urn):
			play_emote(emote_id_or_urn)
			return
		# Base emotes need to be converted to URN for remote fetch
		if Emotes.is_emote_default(emote_id_or_urn):
			emote_urn = Emotes.get_base_emote_urn(emote_id_or_urn)
		else:
			printerr("Unknown emote: %s" % emote_id_or_urn)
			return

	# Does it need to be loaded?
	if _has_emote(emote_urn):
		play_emote(emote_urn)
		return

	# Set loading lock
	_is_loading_emote = true

	if emote_urn.contains("scene-emote"):
		await _async_load_scene_emote(emote_urn)
	else:
		await _async_load_emote(emote_urn)

	# Wait a frame for any deferred calls (load_emote_from_dcl_emote_gltf) to complete
	await avatar.get_tree().process_frame

	# Clear loading lock
	_is_loading_emote = false

	# Use call_deferred to ensure playback happens on main thread after async loading
	play_emote.call_deferred(emote_urn)


func _async_load_emote(emote_urn: String):
	await WearableRequest.async_fetch_emote(emote_urn)

	var emote_content_promises = async_fetch_emote(emote_urn, avatar.avatar_data.get_body_shape())
	await PromiseUtils.async_all(emote_content_promises)

	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote == null:
		printerr("Error: emote is null for URN: " + emote_urn)
		return

	var file_hash = Wearables.get_item_main_file_hash(emote, avatar.avatar_data.get_body_shape())
	if file_hash.is_empty():
		printerr("Error: file_hash is empty for emote: ", emote_urn)
		return

	var obj = Global.content_provider.get_emote_gltf_from_hash(file_hash)
	if obj is DclEmoteGltf:
		# Use call_deferred to ensure animation modifications happen on main thread
		load_emote_from_dcl_emote_gltf.call_deferred(emote_urn, obj, file_hash)


func _async_load_scene_emote(urn: String):
	var emote_scene_urn = EmoteSceneUrn.new(urn)
	if emote_scene_urn.glb_hash.is_empty():
		printerr("Error loading scene-emote ", urn)
		return

	var content_mapping: DclContentMappingAndUrl = DclContentMappingAndUrl.from_values(
		Global.realm.content_base_url + "contents/", {"emote.glb": emote_scene_urn.glb_hash}
	)

	var gltf_promise: Promise = Global.content_provider.fetch_emote_gltf(
		"emote.glb", content_mapping
	)
	var obj = await PromiseUtils.async_awaiter(gltf_promise)

	if obj is PromiseError:
		printerr("Error loading emote '", urn, "': ", obj.get_error())
		return

	# TODO: implement also audio for this
	# var audio_promise: Promise = Global.content_provider.fetch_emote_gltf("emote.mp3", content_mapping)
	# await PromiseUtils.async_awaiter(audio_promise)

	#var obj = Global.content_provider.get_emote_gltf_from_hash(emote_scene_urn.glb_hash)
	if obj is DclEmoteGltf:
		# Use call_deferred to ensure animation modifications happen on main thread
		load_emote_from_dcl_emote_gltf.call_deferred(urn, obj, emote_scene_urn.glb_hash)
		# Wait a frame for the deferred call to execute before checking
		await avatar.get_tree().process_frame
		if _has_emote(urn):
			loaded_emotes_by_urn[urn].looping = emote_scene_urn.looping
			loaded_emotes_by_urn[urn].from_scene = true


func _has_emote(emote_urn: String) -> bool:
	return loaded_emotes_by_urn.has(emote_urn)


func load_emote_from_dcl_emote_gltf(urn: String, obj: DclEmoteGltf, file_hash: String):
	# Avoid adding the emote twice
	if _has_emote(urn):
		return

	# Set guard to prevent concurrent operations
	_is_modifying_animations = true

	# IMPORTANT: Stop all animation processing while modifying animations
	# This prevents crashes when the AnimationMixer tries to access animations being modified
	var was_tree_active = animation_tree.active
	animation_tree.active = false

	# Also stop AnimationPlayer to ensure no iteration over animations
	animation_player.stop()

	# Reset all animation nodes to safe defaults before modifying the library
	# This prevents "!has_animation" errors when reactivating the tree
	animation_single_emote_node.animation = "idle/Anim"
	animation_mix_emote_node.get_node("A").animation = "idle/Anim"
	animation_mix_emote_node.get_node("B").animation = "idle/Anim"

	var armature_prop: Node3D = null

	if obj.armature_prop != null:
		if not avatar.has_node(NodePath(obj.armature_prop.name)):
			armature_prop = obj.armature_prop.duplicate()

			# Stop and remove any AnimationPlayer on the prop to prevent independent animation
			# The prop animation should be controlled by the avatar's AnimationTree via merged tracks
			var prop_anim_player = armature_prop.get_node_or_null("AnimationPlayer")
			if prop_anim_player != null:
				prop_anim_player.stop()
				prop_anim_player.queue_free()

			avatar.add_child(armature_prop)
			armature_prop.hide()  # Start hidden

			# Track the prop name for hiding during idle - DON'T modify idle_anim at runtime
			# Modifying idle_anim while animation system could access it causes crashes
			if not _prop_armature_names.has(armature_prop.name):
				_prop_armature_names.append(armature_prop.name)
		else:
			armature_prop = avatar.get_node(NodePath(obj.armature_prop.name))

	var emote_item_data = EmoteItemData.new(urn, "", "", file_hash, armature_prop, null)

	if obj.default_animation != null:
		var anim_name = obj.default_animation.get_name()
		if anim_name.is_empty():
			anim_name = "emote_" + file_hash.substr(0, 8)

		# If we have both avatar and prop animations, merge them into one
		# Always duplicate the animation to avoid sharing references between avatar instances
		var final_animation: Animation
		if obj.prop_animation != null:
			final_animation = _merge_animations(obj.default_animation, obj.prop_animation)
		else:
			final_animation = obj.default_animation.duplicate()

		if not emotes_animation_library.has_animation(anim_name):
			emotes_animation_library.add_animation(anim_name, final_animation)
		emote_item_data.default_anim_name = anim_name
	else:
		printerr("Error: default_animation is NULL for emote: ", urn)

	loaded_emotes_by_urn[urn] = emote_item_data

	# Reactivate animation system after modifications are complete
	# Do this via call_deferred to ensure all modifications are fully applied
	# before the animation system starts processing again
	_reactivate_animation_system.call_deferred(was_tree_active)


func _reactivate_animation_system(_was_active: bool):
	# Reactivate animation system after modifications
	# This is called via call_deferred to ensure all changes are applied
	# Always set to true - the tree should be active for animations to play
	animation_tree.active = true

	# Clear the modification guard
	_is_modifying_animations = false

	# Process any queued emote request
	if not _queued_emote_urn.is_empty():
		var queued = _queued_emote_urn
		_queued_emote_urn = ""
		# Use another deferred call to ensure tree is fully ready
		play_emote.call_deferred(queued)


func _merge_animations(avatar_anim: Animation, prop_anim: Animation) -> Animation:
	# Create a new animation that combines both avatar and prop tracks
	var merged = avatar_anim.duplicate()

	# Use the longer duration
	if prop_anim.length > merged.length:
		merged.length = prop_anim.length

	# Copy all tracks from prop animation to merged animation
	for i in range(prop_anim.get_track_count()):
		var track_path = prop_anim.track_get_path(i)
		var track_type = prop_anim.track_get_type(i)

		# Add the track
		var new_track_idx = merged.add_track(track_type)
		merged.track_set_path(new_track_idx, track_path)
		merged.track_set_interpolation_type(
			new_track_idx, prop_anim.track_get_interpolation_type(i)
		)

		# Copy all keys
		var key_count = prop_anim.track_get_key_count(i)
		for k in range(key_count):
			var time = prop_anim.track_get_key_time(i, k)
			var value = prop_anim.track_get_key_value(i, k)
			var transition = prop_anim.track_get_key_transition(i, k)
			merged.track_insert_key(new_track_idx, time, value, transition)

	return merged


func clean_unused_emotes():
	var emotes = avatar.avatar_data.get_emotes()
	var to_delete_emote_urns = loaded_emotes_by_urn.keys().filter(
		func(urn): return not emotes.has(urn)
	)

	if to_delete_emote_urns.is_empty():
		return

	# Set guard to prevent concurrent operations
	_is_modifying_animations = true

	# Stop all animation processing while modifying animations
	var was_tree_active = animation_tree.active
	animation_tree.active = false
	animation_player.stop()

	# Reset all animation nodes to safe defaults before modifying the library
	animation_single_emote_node.animation = "idle/Anim"
	animation_mix_emote_node.get_node("A").animation = "idle/Anim"
	animation_mix_emote_node.get_node("B").animation = "idle/Anim"

	for urn in to_delete_emote_urns:
		var emote_item_data: EmoteItemData = loaded_emotes_by_urn[urn]

		if emotes_animation_library.has_animation(emote_item_data.default_anim_name):
			emotes_animation_library.remove_animation(emote_item_data.default_anim_name)
		if emotes_animation_library.has_animation(emote_item_data.prop_anim_name):
			emotes_animation_library.remove_animation(emote_item_data.prop_anim_name)

		if emote_item_data.armature_prop != null:
			# Remove from tracked props
			var prop_name = emote_item_data.armature_prop.name
			var prop_idx = _prop_armature_names.find(prop_name)
			if prop_idx >= 0:
				_prop_armature_names.remove_at(prop_idx)

			avatar.remove_child(emote_item_data.armature_prop)
			emote_item_data.armature_prop.queue_free()

		loaded_emotes_by_urn.erase(urn)

	# Reactivate animation system via deferred call
	_reactivate_animation_system.call_deferred(was_tree_active)


func play_emote_audio(file_hash: String):
	avatar.audio_player_emote.stop()

	var values = loaded_emotes_by_urn.values().filter(
		func(item): return item.file_hash == file_hash
	)
	if values.is_empty():
		return

	var emote = Global.content_provider.get_wearable(values[0].urn)
	if emote == null:
		return

	var audio_file_name = emote.get_emote_audio(avatar.avatar_data.get_body_shape())
	if audio_file_name.is_empty():
		return

	var audio_file_hash = emote.get_content_mapping().get_hash(audio_file_name)
	var audio_stream = Global.content_provider.get_audio_from_hash(audio_file_hash)
	if audio_stream != null:
		avatar.audio_player_emote.stream = audio_stream
		avatar.audio_player_emote.play(0)


func freeze_on_idle():
	animation_tree.process_mode = Node.PROCESS_MODE_DISABLED

	animation_player.stop()
	animation_player.play("Idle", -1, 0.0)

	# Idle animation hides all the extra emotes
	for child in avatar.get_children():
		if child.name.begins_with("Armature_Prop"):
			child.hide()


func async_fetch_emote(emote_urn: String, body_shape_id: String) -> Array:
	var ret = []
	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote != null:
		var file_name: String = emote.get_representation_main_file(body_shape_id)
		if file_name.is_empty():
			return ret
		var content_mapping: DclContentMappingAndUrl = emote.get_content_mapping()
		var promise: Promise = Global.content_provider.fetch_emote_gltf(file_name, content_mapping)
		ret.push_back(promise)

		for audio_file in content_mapping.get_files():
			if audio_file.ends_with(".mp3") or audio_file.ends_with(".ogg"):
				var audio_promise: Promise = Global.content_provider.fetch_audio(
					audio_file, content_mapping
				)
				ret.push_back(audio_promise)
				break
	return ret


func is_playing() -> bool:
	return playing_single || playing_mixed


func process(idle: bool):
	if playing_single or playing_mixed:
		if not idle:
			playing_single = false
			playing_mixed = false
			# Hide props when interrupted
			_hide_all_props()
		else:
			var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
			var cur_node: StringName = pb.get_current_node()
			if cur_node == "Emote" or cur_node == "Emote_Mix":
				# BUG: Looks like pb.is_playing() is not working well
				var is_emote_playing = pb.get_current_play_position() < pb.get_current_length()
				if pb.get_current_play_position() > 0 and not is_emote_playing:
					if playing_loop:
						pb.start(cur_node, true)
					else:
						playing_single = false
						playing_mixed = false
						# Hide props when emote ends
						_hide_all_props()
