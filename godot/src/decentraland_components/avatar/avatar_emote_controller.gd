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

	var from_scene: bool
	var looping: bool

	func _init(
		_urn: String,
		_default_anim_name: String,
		_prop_anim_name: String,
		_file_hash: String,
		_armature_prop: Node3D
	):
		urn = _urn
		default_anim_name = _default_anim_name
		prop_anim_name = _prop_anim_name
		file_hash = _file_hash
		armature_prop = _armature_prop


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

	# Idle Anim Duplication (so it makes mutable and non-shared-reference)
	var idle_animation_library = animation_player.get_animation_library("idle")
	idle_animation_library = idle_animation_library.duplicate(true)
	idle_anim = idle_animation_library.get_animation("Anim")
	animation_player.remove_animation_library("idle")
	animation_player.add_animation_library("idle", idle_animation_library)

	# Emote library
	emotes_animation_library = AnimationLibrary.new()
	animation_player.add_animation_library("emotes", emotes_animation_library)


func play_emote(id: String):
	var triggered: bool = false
	if not id.begins_with("urn"):
		triggered = _play_default_emote(id)
	else:
		triggered = _play_loaded_emote(id)

	if triggered:
		avatar.call_deferred("emit_signal", "emote_triggered", id, playing_loop)


func _play_default_emote(default_emote_id: String) -> bool:
	var anim_name = "default_emotes/" + default_emote_id
	if not animation_player.has_animation(anim_name):
		printerr(
			(
				"Emote %s not found from player '%s'"
				% [default_emote_id, avatar.avatar_data.get_name()]
			)
		)
		return false

	animation_single_emote_node.animation = anim_name
	var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
	if pb.get_current_node() == "Emote":
		pb.start("Emote", true)

	playing_single = true
	playing_mixed = false
	playing_loop = false
	return true


func _play_loaded_emote(emote_urn: String) -> bool:
	if not _has_emote(emote_urn):
		printerr("Emote %s not found from player '%s'" % [emote_urn, avatar.avatar_data.get_name()])
		return false

	var emote_item_data: EmoteItemData = loaded_emotes_by_urn[emote_urn]

	if emote_item_data.from_scene:
		playing_loop = emote_item_data.looping
	else:
		var emote_data = Global.content_provider.get_wearable(emote_item_data.urn)
		if emote_data == null:
			return false
		playing_loop = emote_data.get_emote_loop()

	playing_single = emote_item_data.prop_anim_name.is_empty()
	playing_mixed = not playing_single

	# Single Animation
	if playing_single:
		animation_single_emote_node.animation = "emotes/" + emote_item_data.default_anim_name
		var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
		if pb.get_current_node() == "Emote":
			pb.start("Emote", true)
	elif playing_mixed:
		animation_mix_emote_node.get_node("A").animation = (
			"emotes/" + emote_item_data.default_anim_name
		)
		animation_mix_emote_node.get_node("B").animation = (
			"emotes/" + emote_item_data.prop_anim_name
		)

		var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
		if pb.get_current_node() == "Emote_Mix":
			pb.start("Emote_Mix", true)
	return true


func async_play_emote(emote_urn: String) -> void:
	if not emote_urn.begins_with("urn"):
		play_emote(emote_urn)
		return

	# Does it need to be loaded?
	if _has_emote(emote_urn):
		play_emote(emote_urn)
		return

	if emote_urn.contains("scene-emote"):
		await _async_load_scene_emote(emote_urn)
	else:
		await _async_load_emote(emote_urn)

	play_emote(emote_urn)


func _async_load_emote(emote_urn: String):
	var emote_data_promises = Global.content_provider.fetch_wearables(
		[emote_urn], Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(emote_data_promises)

	var emote_content_promises = async_fetch_emote(emote_urn, avatar.avatar_data.get_body_shape())
	await PromiseUtils.async_all(emote_content_promises)

	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote == null:
		printerr("Error loading emote " + emote_urn)
		return

	var file_hash = Wearables.get_item_main_file_hash(emote, avatar.avatar_data.get_body_shape())
	var obj = Global.content_provider.get_emote_gltf_from_hash(file_hash)
	if obj != null:
		_load_emote_from_dcl_emote_gltf(emote_urn, obj, file_hash)


func _async_load_scene_emote(urn: String):
	var emote_scene_urn = EmoteSceneUrn.new(urn)
	if emote_scene_urn.glb_hash.is_empty():
		printerr("Error loading scene-emote ", urn)
		return

	var content_mapping: DclContentMappingAndUrl = DclContentMappingAndUrl.from_values(
		Global.realm.content_base_url + "contents/", {"emote.glb": emote_scene_urn.glb_hash}
	)

	var gltf_promise: Promise = Global.content_provider.fetch_gltf("emote.glb", content_mapping, 2)
	var obj = await PromiseUtils.async_awaiter(gltf_promise)

	if obj is PromiseError:
		printerr("Error loading emote '", urn, "': ", obj.get_error())
		return

	# TODO: implement also audio for this
	# var audio_promise: Promise = Global.content_provider.fetch_gltf("emote.mp3", content_mapping, 2)
	# await PromiseUtils.async_awaiter(audio_promise)

	#var obj = Global.content_provider.get_emote_gltf_from_hash(emote_scene_urn.glb_hash)
	if obj != null:
		_load_emote_from_dcl_emote_gltf(urn, obj, emote_scene_urn.glb_hash)
		if _has_emote(urn):
			loaded_emotes_by_urn[urn].looping = emote_scene_urn.looping
			loaded_emotes_by_urn[urn].from_scene = true


func _has_emote(emote_urn: String) -> bool:
	return loaded_emotes_by_urn.has(emote_urn)


func _load_emote_from_dcl_emote_gltf(urn: String, obj: DclEmoteGltf, file_hash: String):
	# Avoid adding the emote twice
	if _has_emote(urn):
		return

	var armature_prop: Node3D = null
	if obj.armature_prop != null:
		if not avatar.has_node(NodePath(obj.armature_prop.name)):
			armature_prop = obj.armature_prop.duplicate()
			avatar.add_child(armature_prop)

			var track_id = idle_anim.add_track(Animation.TYPE_VALUE)
			idle_anim.track_set_path(track_id, NodePath(armature_prop.name + ":visible"))
			idle_anim.track_insert_key(track_id, 0.0, false)

	var emote_item_data = EmoteItemData.new(urn, "", "", file_hash, armature_prop)
	if obj.default_animation != null:
		emotes_animation_library.add_animation(
			obj.default_animation.get_name(), obj.default_animation
		)
		emote_item_data.default_anim_name = obj.default_animation.get_name()

	if obj.prop_animation != null:
		emotes_animation_library.add_animation(obj.prop_animation.get_name(), obj.prop_animation)
		emote_item_data.prop_anim_name = obj.prop_animation.get_name()

	loaded_emotes_by_urn[urn] = emote_item_data


func clean_unused_emotes():
	var emotes = avatar.avatar_data.get_emotes()
	var to_delete_emote_urns = loaded_emotes_by_urn.keys().filter(
		func(urn): return not emotes.has(urn)
	)

	for urn in to_delete_emote_urns:
		var emote_item_data: EmoteItemData = loaded_emotes_by_urn[urn]

		if emotes_animation_library.has_animation(emote_item_data.default_anim_name):
			emotes_animation_library.remove_animation(emote_item_data.default_anim_name)
		if emotes_animation_library.has_animation(emote_item_data.prop_anim_name):
			emotes_animation_library.remove_animation(emote_item_data.prop_anim_name)

		if emote_item_data.armature_prop != null:
			avatar.remove_child(emote_item_data.armature_prop)

		loaded_emotes_by_urn.erase(urn)


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


func broadcast_avatar_animation(emote_id: String) -> void:
	# Send emote
	var timestamp = Time.get_unix_time_from_system() * 1000
	Global.comms.send_chat("␐%s %d" % [emote_id, timestamp])


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
		var promise: Promise = Global.content_provider.fetch_gltf(file_name, content_mapping, 2)
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
