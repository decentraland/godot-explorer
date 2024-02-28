class_name Avatar
extends DclAvatar

signal avatar_loaded

@export var skip_process: bool = false
@export var hide_name: bool = false
@export var non_3d_audio: bool = false

# Public
var avatar_id: String = ""

# Current wearables equippoed
var wearables_dict: Dictionary = {}

var finish_loading = false
var wearables_by_category

var emote_controller: AvatarEmoteController

var generate_attach_points: bool = false
var right_hand_idx: int = -1
var right_hand_position: Transform3D
var left_hand_idx: int = -1
var left_hand_position: Transform3D

var voice_chat_audio_player: AudioStreamPlayer = null
var voice_chat_audio_player_gen: AudioStreamGenerator = null

var mask_material = preload("res://assets/avatar/mask_material.tres")

@onready var animation_tree = $AnimationTree
@onready var animation_player = $AnimationPlayer
@onready var label_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name/Label3D_Name
@onready var sprite_3d_mic_enabled = %Sprite3D_MicEnabled
@onready var timer_hide_mic = %Timer_HideMic
@onready var body_shape_skeleton_3d: Skeleton3D = $Armature/Skeleton3D
@onready var bone_attachment_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name
@onready var audio_player_emote = $AudioPlayer_Emote


func _ready():
	emote_controller = AvatarEmoteController.new(self, animation_player, animation_tree)
	body_shape_skeleton_3d.bone_pose_changed.connect(self._attach_point_bone_pose_changed)

	if non_3d_audio:
		var audio_player_name = audio_player_emote.get_name()
		remove_child(audio_player_emote)
		audio_player_emote = AudioStreamPlayer.new()
		add_child(audio_player_emote)
		audio_player_emote.name = audio_player_name


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
	var new_avatar_name: String = profile.get_name()
	if not profile.has_claimed_name():
		new_avatar_name += "#" + profile.get_ethereum_address().right(4)
	label_3d_name.modulate = Color.GOLD if profile.has_claimed_name() else Color.WHITE

	await async_update_avatar(avatar, new_avatar_name)


func async_update_avatar(new_avatar: DclAvatarWireFormat, new_avatar_name: String):
	set_avatar_data(new_avatar)
	set_avatar_name(new_avatar_name)
	if new_avatar == null:
		printerr("Trying to update an avatar with an null value")
		return

	var wearable_to_request := []

	sprite_3d_mic_enabled.hide()
	label_3d_name.text = new_avatar_name
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

	finish_loading = false

	var promise = Global.content_provider.fetch_wearables(
		wearable_to_request, Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)
	await async_fetch_wearables_dependencies()


func update_colors(eyes_color: Color, skin_color: Color, hair_color: Color) -> void:
	avatar_data.set_eyes_color(eyes_color)
	avatar_data.set_skin_color(skin_color)
	avatar_data.set_hair_color(hair_color)

	if finish_loading:
		apply_color_and_facial()


func async_fetch_wearables_dependencies():
	# Clear last equipped werarables
	wearables_dict.clear()

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

	for wearable_key in wearables_dict.keys():
		if wearables_dict[wearable_key] == null:
			printerr("wearable ", wearable_key, " null")
			continue

		var wearable: DclItemEntityDefinition = wearables_dict[wearable_key]
		if not Wearables.is_valid_wearable(wearable, body_shape_id, true):
			continue

		var hashes_to_fetch: Array
		if Wearables.is_texture(wearable.get_category()):
			hashes_to_fetch = Wearables.get_wearable_facial_hashes(wearable, body_shape_id)
		else:
			hashes_to_fetch = [Wearables.get_item_main_file_hash(wearable, body_shape_id)]

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
			async_calls_info.push_back(wearable_key)

	var promises_result: Array = await PromiseUtils.async_all(async_calls)
	for i in range(promises_result.size()):
		if promises_result[i] is PromiseError:
			printerr("Error loading ", async_calls_info[i], ":", promises_result[i].get_error())

	await async_load_wearables()


func _fetch_texture_or_gltf(file_name: String, content_mapping: DclContentMappingAndUrl) -> Promise:
	var promise: Promise

	if file_name.ends_with(".png"):
		promise = Global.content_provider.fetch_texture(file_name, content_mapping)
	else:
		promise = Global.content_provider.fetch_gltf(file_name, content_mapping, 1)

	return promise


func try_to_set_body_shape(body_shape_hash):
	var body_shape: Node3D = Global.content_provider.get_gltf_from_hash(body_shape_hash)
	if body_shape == null:
		return

	var new_skeleton = body_shape.find_child("Skeleton3D")
	if new_skeleton == null:
		return

	for child in body_shape_skeleton_3d.get_children():
		if child is MeshInstance3D:
			body_shape_skeleton_3d.remove_child(child)

	for child in new_skeleton.get_children():
		var new_child = child.duplicate()
		new_child.name = "bodyshape_" + child.name.to_lower()
		body_shape_skeleton_3d.add_child(new_child)

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
		Wearables.get_item_main_file_hash(body_shape_wearable, avatar_data.get_body_shape())
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

		var file_hash = Wearables.get_item_main_file_hash(wearable, avatar_data.get_body_shape())
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

	# Emotes
	for emote_urn in avatar_data.get_emotes():
		if not emote_urn.begins_with("urn"):
			# Default
			continue

		var emote = Global.content_provider.get_wearable(emote_urn)
		var file_hash = Wearables.get_item_main_file_hash(emote, avatar_data.get_body_shape())
		var obj = Global.content_provider.get_emote_gltf_from_hash(file_hash)
		if obj != null:
			emote_controller.load_emote_from_dcl_emote_gltf(emote_urn, obj, file_hash)

	emote_controller.clean_unused_emotes()
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
	else:
		current_material.set_shader_parameter("mask_texture", null)

	mesh.mesh.surface_set_material(0, current_material)


func _process(delta):
	# TODO: maybe a gdext crate bug? when process implement the INode3D, super(delta) doesn't work :/
	self.process(delta)

	var self_idle = !self.jog && !self.walk && !self.run && !self.rise && !self.fall
	emote_controller.process(self_idle)

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


func _play_emote_audio(file_hash: String):
	emote_controller.play_emote_audio(file_hash)


func async_play_emote(emote_urn: String):
	await emote_controller.async_play_emote(emote_urn)


func broadcast_avatar_animation(emote_id: String) -> void:
	emote_controller.broadcast_avatar_animation(emote_id)
