class_name Avatar
extends DclAvatar

signal avatar_loaded

@export var skip_process: bool = false
@export var hide_name: bool = false

# Public
var avatar_id: String = ""

# Wearable requesting
var current_content_url: String = ""

# Current wearables equippoed
var wearables_dict: Dictionary = {}

var finish_loading = false
var wearables_by_category

var playing_emote: bool = false

var generate_attach_points: bool = false
var right_hand_idx: int = -1
var right_hand_position: Transform3D
var left_hand_idx: int = -1
var left_hand_position: Transform3D
var index_to_animation_name: Dictionary = {}

var voice_chat_audio_player: AudioStreamPlayer = null
var voice_chat_audio_player_gen: AudioStreamGenerator = null

var mask_material = preload("res://assets/avatar/mask_material.tres")

@onready var animation_tree = $Armature/AnimationTree
@onready var animation_tree_root: AnimationNodeStateMachine = animation_tree.tree_root
@onready var animation_emote_node: AnimationNodeAnimation = animation_tree_root.get_node("Emote")
@onready var animation_player = $Armature/AnimationPlayer
@onready var global_animation_library: AnimationLibrary = animation_player.get_animation_library("")
@onready var label_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name/Label3D_Name
@onready var sprite_3d_mic_enabled = %Sprite3D_MicEnabled
@onready var timer_hide_mic = %Timer_HideMic
@onready var body_shape_root: Node3D = $Armature
@onready var body_shape_skeleton_3d: Skeleton3D = $Armature/Skeleton3D
@onready var bone_attachment_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name


func _on_set_avatar_modifier_area(area: DclAvatarModifierArea3D):
	_unset_avatar_modifier_area()  # Reset state

	for exclude_id in area.exclude_ids:
		if avatar_id == exclude_id:
			return  # the avatar is not going to be modified

	for modifier in area.avatar_modifiers:
		if modifier == 0:  # hide avatar
			hide()
		elif modifier == 1:  # disable passport
			pass  # TODO: Passport (disable functionality)


func _unset_avatar_modifier_area():
	show()
	# TODO: Passport (enable functionality)


func async_update_avatar_from_profile(profile: DclUserProfile):
	var avatar = profile.get_avatar()
	avatar.set_name(profile.get_name())
	await async_update_avatar(avatar)


func async_update_avatar(new_avatar: DclAvatarWireFormat):
	avatar_data = new_avatar
	if new_avatar == null:
		printerr("Trying to update an avatar with an null value")
		return

	var wearable_to_request := []

	current_content_url = "https://peer.decentraland.org/content/"
	if not Global.realm.content_base_url.is_empty():
		current_content_url = Global.realm.content_base_url

	sprite_3d_mic_enabled.hide()
	label_3d_name.text = avatar_data.get_name()
	if hide_name:
		label_3d_name.hide()

	wearable_to_request.append_array(avatar_data.get_wearables())

	for emote_urn in avatar_data.get_emotes():
		if emote_urn.begins_with("urn"):
			wearable_to_request.push_back(emote_urn)

	wearable_to_request.push_back(avatar_data.get_body_shape())

	# TODO: Validate if the current profile can own this wearables
	# tracked at https://github.com/decentraland/godot-explorer/issues/244
	# wearable_to_request = filter_owned_wearables(wearable_to_request)

	_load_default_emotes()

	finish_loading = false

	var promise = Global.content_provider.fetch_wearables(wearable_to_request, current_content_url)
	await PromiseUtils.async_all(promise)
	await async_fetch_wearables_dependencies()


static func from_color_object(color: Variant, default: Color = Color.WHITE) -> Color:
	if color is Dictionary:
		return Color(
			color.get("r", default.r),
			color.get("g", default.g),
			color.get("b", default.b),
			color.get("a", default.a)
		)
	return default


static func to_color_object(color: Color) -> Dictionary:
	return {"color": {"r": color.r, "g": color.g, "b": color.b, "a": color.a}}


func _add_animation(index: int, animation_name: String):
	var animation = Global.animation_importer.get_animation_from_gltf(animation_name)
	if animation:
		global_animation_library.add_animation(animation_name, animation)
	index_to_animation_name[index] = animation_name


func _clear_animations():
	for index in index_to_animation_name:
		var animation_name = index_to_animation_name[index]
		global_animation_library.remove_animation(animation_name)
	index_to_animation_name.clear()


func _load_default_emotes():
	_clear_animations()
	_add_animation(0, "handsair")
	_add_animation(1, "wave")
	_add_animation(2, "fistpump")
	_add_animation(3, "dance")
	_add_animation(4, "raiseHand")
	_add_animation(5, "clap")
	_add_animation(6, "money")
	_add_animation(7, "kiss")
	_add_animation(8, "shrug")
	_add_animation(9, "headexplode")


func is_playing_emote() -> bool:
	return playing_emote


func play_emote(emote_id: String):
	if animation_player.has_animation(emote_id):
		animation_emote_node.animation = emote_id

		var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
		if pb.get_current_node() == "Emote":
			pb.start("Emote", true)
		playing_emote = true
	else:
		printerr("Emote %s not found from player '%s'" % [emote_id, avatar_data.get_name()])


func play_remote_emote(_emote_src: String, _looping: bool):
	# TODO: Implement downloading emote from the scene content, adding to the avatar and then playing the emote
	# Test scene: https://github.com/decentraland/unity-renderer/pull/5501
	pass


func play_emote_by_index(index: int) -> String:
	# Play emote
	var emote_id: String = index_to_animation_name[index]
	play_emote(emote_id)

	return emote_id

func restore_freeze():
	animation_tree.process_mode = Node.PROCESS_MODE_INHERIT
	animation_player.play("Idle")

func freeze_on_idle():
	animation_tree.process_mode = Node.PROCESS_MODE_DISABLED
	animation_player.stop()
	animation_player.play("Idle", -1, 0.0)


func broadcast_avatar_animation(emote_id: String) -> void:
	# Send emote
	var timestamp = Time.get_unix_time_from_system() * 1000
	Global.comms.send_chat("␐%s %d" % [emote_id, timestamp])


func update_colors(eyes_color: Color, skin_color: Color, hair_color: Color) -> void:
	avatar_data.set_eyes_color(eyes_color)
	avatar_data.set_skin_color(skin_color)
	avatar_data.set_hair_color(hair_color)

	if finish_loading:
		apply_color_and_facial()


func get_representation(representation_array: Array, desired_body_shape: String) -> Dictionary:
	for representation in representation_array:
		var index = representation.get("bodyShapes", []).find(desired_body_shape)
		if index != -1:
			return representation

	return representation_array[0]


func async_fetch_wearables_dependencies():
	# Clear last equipped werarables
	wearables_dict.clear()

	# Fill data
	var body_shape_id := avatar_data.get_body_shape()
	wearables_dict[body_shape_id] = Global.content_provider.get_wearable(body_shape_id)
	for item in avatar_data.get_wearables():
		wearables_dict[item] = Global.content_provider.get_wearable(item)

	var async_calls: Array = []
	for wearable_key in wearables_dict.keys():
		if wearables_dict[wearable_key] == null:
			printerr("wearable ", wearable_key, " null")
			continue

		var wearable: DclWearableEntityDefinition = wearables_dict[wearable_key]
		if not Wearables.is_valid_wearable(wearable, body_shape_id, true):
			continue

		var hashes_to_fetch: Array
		if Wearables.is_texture(wearable.get_category()):
			hashes_to_fetch = Wearables.get_wearable_facial_hashes(wearable, body_shape_id)
		else:
			hashes_to_fetch = [Wearables.get_wearable_main_file_hash(wearable, body_shape_id)]

		if hashes_to_fetch.is_empty():
			continue

		var content_mapping: DclContentMappingAndUrl = wearable.get_content_mapping()
		var files: Array = []
		for file_name in content_mapping.get_files():
			for file_hash in hashes_to_fetch:
				if content_mapping.get_hash(file_name) == file_hash:
					files.push_back(file_name)

		for file_name in files:
			async_calls.push_back(_fetch_texture_or_gltf(file_name, content_mapping))

	await PromiseUtils.async_all(async_calls)

	await async_load_wearables()


func _fetch_texture_or_gltf(file_name: String, content_mapping: DclContentMappingAndUrl) -> Promise:
	var promise: Promise

	if file_name.ends_with(".png"):
		promise = Global.content_provider.fetch_texture(file_name, content_mapping)
	else:
		promise = Global.content_provider.fetch_gltf(file_name, content_mapping)

	return promise


func _free_old_skeleton(skeleton: Node):
	for child in skeleton.get_children():
		child.free()
#		if child is MeshInstance3D:
#			for i in child.get_surface_override_material_count():
#				var material = child.mesh.surface_get_material(i)
#				material.free()

	skeleton.free()


func try_to_set_body_shape(body_shape_hash):
	var body_shape: Node3D = Global.content_provider.get_gltf_from_hash(body_shape_hash)
	if body_shape == null:
		return

	var skeleton = body_shape.find_child("Skeleton3D")
	if skeleton == null:
		return

	var animation_player_parent = animation_player.get_parent()
	if animation_player_parent != null:
		animation_player_parent.remove_child(animation_tree)
		animation_player_parent.remove_child(animation_player)

	var bone_attachment_3d_name_parent = bone_attachment_3d_name.get_parent()
	if bone_attachment_3d_name_parent != null:
		bone_attachment_3d_name_parent.remove_child(bone_attachment_3d_name)

	if body_shape_root != null:
		remove_child(body_shape_root)
		_free_old_skeleton.call_deferred(body_shape_root)

	body_shape_root = body_shape.duplicate()
	body_shape_root.name = "BodyShape"

	body_shape_skeleton_3d = body_shape_root.find_child("Skeleton3D", true, false)
	body_shape_skeleton_3d.get_parent().add_child(animation_player)
	body_shape_skeleton_3d.get_parent().add_child(animation_tree)
	body_shape_skeleton_3d.add_child(bone_attachment_3d_name)
	bone_attachment_3d_name.bone_name = "Avatar_Head"

	for child in body_shape_skeleton_3d.get_children():
		child.name = child.name.to_lower()

	body_shape_skeleton_3d.visible = false
	add_child(body_shape_root)

	_add_attach_points()


func async_load_wearables():
	var curated_wearables = Wearables.get_curated_wearable_list(
		avatar_data.get_body_shape(), avatar_data.get_wearables(), []
	)
	if curated_wearables.is_empty():
		printerr("couldn't get curated wearables")
		return

	wearables_by_category = curated_wearables[0]

	var body_shape_wearable = wearables_by_category.get(Wearables.Categories.BODY_SHAPE)
	if body_shape_wearable == null:
		printerr("body shape not found")
		return

	try_to_set_body_shape(
		Wearables.get_wearable_main_file_hash(body_shape_wearable, avatar_data.get_body_shape())
	)
	wearables_by_category.erase(Wearables.Categories.BODY_SHAPE)

	var has_own_skin = false
	var has_own_upper_body = false
	var has_own_lower_body = false
	var has_own_feet = false
	var has_own_head = false

	for category in wearables_by_category:
		var wearable = wearables_by_category[category]

		# Skip
		if Wearables.is_texture(category):
			continue

		var file_hash = Wearables.get_wearable_main_file_hash(
			wearable, avatar_data.get_body_shape()
		)
		var obj = Global.content_provider.get_gltf_from_hash(file_hash)
		var wearable_skeleton: Skeleton3D = obj.find_child("Skeleton3D")
		for child in wearable_skeleton.get_children():
			var new_wearable = child.duplicate()
			new_wearable.name = new_wearable.name.to_lower() + "_" + category
			body_shape_skeleton_3d.add_child(new_wearable)

		match category:
			Wearables.Categories.UPPER_BODY:
				has_own_upper_body = true
			Wearables.Categories.LOWER_BODY:
				has_own_lower_body = true
			Wearables.Categories.FEET:
				has_own_feet = true
			Wearables.Categories.HEAD:
				has_own_head = true
			Wearables.Categories.SKIN:
				has_own_skin = true

	var hidings = {
		"ubody_basemesh": has_own_skin or has_own_upper_body,
		"lbody_basemesh": has_own_skin or has_own_lower_body,
		"feet_basemesh": has_own_skin or has_own_feet,
		"head": has_own_skin or has_own_head,
		"head_basemesh": has_own_skin or has_own_head,
		"mask_eyes": has_own_skin or has_own_head,
		"mask_eyebrows": has_own_skin or has_own_head,
		"mask_mouth": has_own_skin or has_own_head,
	}

	for child in body_shape_skeleton_3d.get_children():
		var should_hide = false
		for ends_with in hidings:
			if child.name.ends_with(ends_with) and hidings[ends_with]:
				should_hide = true

		if should_hide:
			child.hide()

	var meshes: Array[Dictionary] = []
	for child in body_shape_skeleton_3d.get_children():
		if child.visible and child is MeshInstance3D:
			child.mesh = child.mesh.duplicate(true)
			meshes.push_back({"n": child.get_surface_override_material_count(), "mesh": child.mesh})

	var promise: Promise = Global.content_provider.duplicate_materials(meshes)
	await PromiseUtils.async_awaiter(promise)
	apply_color_and_facial()
	body_shape_skeleton_3d.visible = true
	finish_loading = true

	emit_signal("avatar_loaded")


func apply_color_and_facial():
	for child in body_shape_skeleton_3d.get_children():
		if child.visible and child is MeshInstance3D:
			for i in range(child.get_surface_override_material_count()):
				var mat_name = child.mesh.get("surface_" + str(i) + "/name").to_lower()
				var material = child.mesh.surface_get_material(i)

				if material is StandardMaterial3D:
					material.metallic = 0
					material.metallic_specular = 0
					if mat_name.find("skin") != -1:
						material.albedo_color = avatar_data.get_skin_color()
						material.metallic = 0
					elif mat_name.find("hair") != -1:
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


func apply_texture_and_mask(
	mesh: MeshInstance3D, textures: Array[String], color: Color, mask_color: Color
):
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

	mesh.mesh.surface_set_material(0, current_material)


func _process(delta):
	# TODO: maybe a gdext crate bug? when process implement the INode3D, super(delta) doesn't work :/
	self.process(delta)
	var self_idle = !self.jog && !self.walk && !self.run && !self.rise && !self.fall

	if playing_emote:
		if not self_idle:
			playing_emote = false
		else:
			var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
			if pb.get_current_node() == "Emote":
				# BUG: Looks like pb.is_playing() is not working well
				var is_emote_playing = pb.get_current_play_position() < pb.get_current_length()
				if pb.get_current_play_position() > 0 and not is_emote_playing:
					playing_emote = false

	animation_tree.set("parameters/conditions/idle", self_idle)
	animation_tree.set("parameters/conditions/emote", playing_emote)
	animation_tree.set("parameters/conditions/nemote", not playing_emote)

	animation_tree.set("parameters/conditions/run", self.run)
	animation_tree.set("parameters/conditions/jog", self.jog)
	animation_tree.set("parameters/conditions/walk", self.walk)

	animation_tree.set("parameters/conditions/rise", self.rise)
	animation_tree.set("parameters/conditions/fall", self.fall)
	animation_tree.set("parameters/conditions/land", self.land)

	animation_tree.set("parameters/conditions/nfall", !self.fall)


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
	sprite_3d_mic_enabled.show()
	timer_hide_mic.start()


func activate_attach_points():
	generate_attach_points = true
	_add_attach_points()


func _add_attach_points():
	if not generate_attach_points:
		return

	if body_shape_skeleton_3d == null:
		return

	right_hand_idx = body_shape_skeleton_3d.find_bone("Avatar_RightHand")
	left_hand_idx = body_shape_skeleton_3d.find_bone("Avatar_LeftHand")
	body_shape_skeleton_3d.bone_pose_changed.connect(self._attach_point_bone_pose_changed)


func _attach_point_bone_pose_changed(bone_idx: int):
	match bone_idx:
		left_hand_idx:
			left_hand_position = body_shape_skeleton_3d.get_bone_global_pose(bone_idx)
			left_hand_position.basis = left_hand_position.basis.scaled(100.0 * Vector3.ONE)

		right_hand_idx:
			right_hand_position = body_shape_skeleton_3d.get_bone_global_pose(bone_idx)
			right_hand_position.basis = right_hand_position.basis.scaled(100.0 * Vector3.ONE)


func _on_timer_hide_mic_timeout():
	sprite_3d_mic_enabled.hide()


func get_avatar_name() -> String:
	if avatar_data != null:
		return avatar_data.get_name()
	return ""
