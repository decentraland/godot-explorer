class_name Avatar
extends DclAvatar

signal avatar_loaded

@export var skip_process: bool = false
@export var hide_name: bool = false
@export var non_3d_audio: bool = false


class EmoteItemData:
	extends RefCounted
	var emote_prefix_id: String = ""
	var urn: String = ""
	var default_anim_name: String = ""
	var prop_anim_name: String = ""
	var file_hash: String = ""
	var armature_prop: Node3D = null

	func _init(
		_emote_prefix_id: String,
		_urn: String,
		_default_anim_name: String,
		_prop_anim_name: String,
		_file_hash: String,
		_armature_prop: Node3D
	):
		emote_prefix_id = _emote_prefix_id
		urn = _urn
		default_anim_name = _default_anim_name
		prop_anim_name = _prop_anim_name
		file_hash = _file_hash
		armature_prop = _armature_prop


var loaded_emotes: Array[EmoteItemData] = []

# Public
var avatar_id: String = ""

# Current wearables equippoed
var wearables_dict: Dictionary = {}

var finish_loading = false
var wearables_by_category

var playing_emote_single: bool = false
var playing_emote_mixed: bool = false
var playing_emote_loop: bool = false
var emotes_animation_library: AnimationLibrary
var idle_anim: Animation

var generate_attach_points: bool = false
var right_hand_idx: int = -1
var right_hand_position: Transform3D
var left_hand_idx: int = -1
var left_hand_position: Transform3D

var voice_chat_audio_player: AudioStreamPlayer = null
var voice_chat_audio_player_gen: AudioStreamGenerator = null

var mask_material = preload("res://assets/avatar/mask_material.tres")

@onready var animation_tree = $AnimationTree
@onready
var animation_single_emote_node: AnimationNodeAnimation = animation_tree.tree_root.get_node("Emote")
@onready var animation_mix_emote_node: AnimationNodeBlendTree = animation_tree.tree_root.get_node(
	"Emote_Mix"
)
@onready var animation_player = $AnimationPlayer
@onready var label_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name/Label3D_Name
@onready var sprite_3d_mic_enabled = %Sprite3D_MicEnabled
@onready var timer_hide_mic = %Timer_HideMic
@onready var body_shape_skeleton_3d: Skeleton3D = $Armature/Skeleton3D
@onready var bone_attachment_3d_name = $Armature/Skeleton3D/BoneAttachment3D_Name
@onready var audio_player_emote = $AudioPlayer_Emote


func _ready():
	body_shape_skeleton_3d.bone_pose_changed.connect(self._attach_point_bone_pose_changed)

	# Idle Anim Duplication (so it makes mutable and non-shared-reference)
	var idle_animation_library = animation_player.get_animation_library("idle")
	idle_animation_library = idle_animation_library.duplicate(true)
	idle_anim = idle_animation_library.get_animation("Anim")
	animation_player.remove_animation_library("idle")
	animation_player.add_animation_library("idle", idle_animation_library)

	# Emote library
	emotes_animation_library = AnimationLibrary.new()
	animation_player.add_animation_library("emotes", emotes_animation_library)

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
	avatar.set_name(profile.get_name())
	await async_update_avatar(avatar)


func async_update_avatar(new_avatar: DclAvatarWireFormat):
	# TODO: Hardcoded emotes, change those with the configured emotes
	var emotes = PackedStringArray(
		[
			"handsair",
			"wave",
			"urn:decentraland:matic:collections-v2:0x0b472c2c04325a545a43370b54e93c87f3d5badf:0",
			"urn:decentraland:matic:collections-v2:0x54bf16bed39a02d5f8bda33664c72c59d367caf7:0",
			"urn:decentraland:matic:collections-v2:0x70eb032d4621a51945b913c3f9488d50fc1fca38:0",
			"urn:decentraland:matic:collections-v2:0x875146d1d26e91c80f25f5966a84b098d3db1fc8:1:105312291668557186697918027683670432318895095400549111254310981119",
			"urn:decentraland:matic:collections-v2:0xa25c20f58ac447621a5f854067b857709cbd60eb:7:737186041679900306885426193785693026232265667803843778780176846151",
			"urn:decentraland:matic:collections-v2:0xbada8a315e84e4d78e3b6914003647226d9b4001:10:1053122916685571866979180276836704323188950954005491112543109777455",
			"urn:decentraland:matic:collections-v2:0xbada8a315e84e4d78e3b6914003647226d9b4001:11:1158435208354129053677098304520374755507846049406040223797420753072",
			"urn:decentraland:matic:collections-v2:0x0c956c74518ed34afb7b137d9ddfdaea7ca13751:0"
		]
	)
	new_avatar.set_emotes(emotes)
	avatar_data = new_avatar
	if new_avatar == null:
		printerr("Trying to update an avatar with an null value")
		return

	var wearable_to_request := []

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

	finish_loading = false

	var promise = Global.content_provider.fetch_wearables(
		wearable_to_request, Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)
	await async_fetch_wearables_dependencies()


func is_playing_emote() -> bool:
	return playing_emote_single || playing_emote_mixed


func play_default_emote(default_emote_name: String) -> bool:
	if not animation_player.has(default_emote_name):
		return false
		
	animation_single_emote_node.animation = default_emote_name
	var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
	if pb.get_current_node() == "Emote":
		pb.start("Emote", true)

	playing_emote_single = true
	playing_emote_mixed = false
	playing_emote_loop = false
	return true


func _play_loaded_emote_by_urn(urn: String):
	var emote_data = Global.content_provider.get_wearable(urn)
	if emote_data == null:
		return
	play_loaded_emote(emote_data.get_emote_prefix_id())


func play_loaded_emote(emote_prefix_id: String) -> bool:
	if emote_prefix_id.begins_with("default"):
		return play_default_emote(emote_prefix_id)

	var values = loaded_emotes.filter(func(item): return item.emote_prefix_id == emote_prefix_id)
	if values.is_empty():
		printerr("Emote %s not found from player '%s'" % [emote_prefix_id, avatar_data.get_name()])
		return false

	var emote_item_data: EmoteItemData = values[0]
	var emote_data = Global.content_provider.get_wearable(emote_item_data.urn)

	if emote_data == null:
		return false

	playing_emote_loop = emote_data.get_emote_loop()
	playing_emote_single = emote_item_data.prop_anim_name.is_empty()
	playing_emote_mixed = not playing_emote_single

	# Single Animation
	if playing_emote_single:
		animation_single_emote_node.animation = "emotes/" + emote_item_data.default_anim_name
		var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
		if pb.get_current_node() == "Emote":
			pb.start("Emote", true)
	elif playing_emote_mixed:
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

func play_remote_emote(urn: String):
	# TODO: Implement downloading emote from the scene content, adding to the avatar and then playing the emote
	# Test scene: https://github.com/decentraland/unity-renderer/pull/5501
	pass


func play_emote(id: String) -> bool:
	if id.contains("scene-remote"):
		return play_remote_emote(id)
	else:
		return play_loaded_emote(id)

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
			var emote_promises = async_fetch_emote(emote_urn, body_shape_id)
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
		if new_child is MeshInstance3D:
			new_child.skeleton = body_shape_skeleton_3d.get_path()

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
			add_animation_from_dcl_emote_gltf(emote, emote_urn, obj, file_hash)

	clean_unused_emotes()
	emit_signal("avatar_loaded")


func async_play_emote(emote_urn: String):
	if has_emote(emote_urn):
		_play_loaded_emote_by_urn(emote_urn)
		return

	var emote_data_promises = Global.content_provider.fetch_wearables(
		[emote_urn], Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(emote_data_promises)

	var emote_content_promises = async_fetch_emote(emote_urn, avatar_data.get_body_shape())
	await PromiseUtils.async_all(emote_content_promises)

	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote == null:
		printerr("Error loading emote " + emote_urn)
		return

	var file_hash = Wearables.get_item_main_file_hash(emote, avatar_data.get_body_shape())
	var obj = Global.content_provider.get_emote_gltf_from_hash(file_hash)
	if obj != null:
		add_animation_from_dcl_emote_gltf(emote, emote_urn, obj, file_hash)
		play_loaded_emote(emote.get_emote_prefix_id())


func has_emote(emote_urn: String) -> bool:
	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote == null:
		return false

	var emote_prefix_id: String = emote.get_emote_prefix_id()
	for loaded_emote: EmoteItemData in loaded_emotes:
		if loaded_emote.emote_prefix_id == emote_prefix_id:
			return true
	return false


func add_animation_from_dcl_emote_gltf(
	emote: DclItemEntityDefinition, urn: String, obj: DclEmoteGltf, file_hash: String
):
	var armature_prop: Node3D = null
	if obj.armature_prop != null:
		if not has_node(NodePath(obj.armature_prop.name)):
			armature_prop = obj.armature_prop.duplicate()
			self.add_child(armature_prop)

			var track_id = idle_anim.add_track(Animation.TYPE_VALUE)
			idle_anim.track_set_path(track_id, NodePath(armature_prop.name + ":visible"))
			idle_anim.track_insert_key(track_id, 0.0, false)

	var emote_prefix_id = emote.get_emote_prefix_id()
	var emote_item_data = EmoteItemData.new(emote_prefix_id, urn, "", "", file_hash, armature_prop)
	if obj.default_animation != null:
		emotes_animation_library.add_animation(
			obj.default_animation.get_name(), obj.default_animation
		)
		emote_item_data.default_anim_name = obj.default_animation.get_name()

	if obj.prop_animation != null:
		emotes_animation_library.add_animation(obj.prop_animation.get_name(), obj.prop_animation)
		emote_item_data.prop_anim_name = obj.prop_animation.get_name()

	loaded_emotes.push_back(emote_item_data)


func clean_unused_emotes():
	var emotes = avatar_data.get_emotes()
	var to_delete_emotes = loaded_emotes.filter(func(item): return not emotes.has(item.urn))
	for emote_item_data: EmoteItemData in to_delete_emotes:
		if emotes_animation_library.has_animation(emote_item_data.default_anim_name):
			emotes_animation_library.remove_animation(emote_item_data.default_anim_name)
		if emotes_animation_library.has_animation(emote_item_data.prop_anim_name):
			emotes_animation_library.remove_animation(emote_item_data.prop_anim_name)

		if emote_item_data.armature_prop != null:
			remove_child(emote_item_data.armature_prop)

		loaded_emotes.erase(emote_item_data)


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

	if playing_emote_single or playing_emote_mixed:
		if not self_idle:
			playing_emote_single = false
			playing_emote_mixed = false
		else:
			var pb: AnimationNodeStateMachinePlayback = animation_tree.get("parameters/playback")
			var cur_node: StringName = pb.get_current_node()
			if cur_node == "Emote" or cur_node == "Emote_Mix":
				# BUG: Looks like pb.is_playing() is not working well
				var is_emote_playing = pb.get_current_play_position() < pb.get_current_length()
				if pb.get_current_play_position() > 0 and not is_emote_playing:
					if playing_emote_loop:
						pb.start(cur_node, true)
					else:
						playing_emote_single = false
						playing_emote_mixed = false

	animation_tree.set("parameters/conditions/idle", self_idle)
	animation_tree.set("parameters/conditions/emote", playing_emote_single)
	animation_tree.set("parameters/conditions/nemote", not playing_emote_single)
	animation_tree.set("parameters/conditions/emix", playing_emote_mixed)
	animation_tree.set("parameters/conditions/nemix", not playing_emote_mixed)

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


func get_avatar_name() -> String:
	if avatar_data != null:
		return avatar_data.get_name()
	return ""


func _play_emote_audio(file_hash: String):
	audio_player_emote.stop()

	var values = loaded_emotes.filter(func(item): return item.file_hash == file_hash)
	if values.is_empty():
		return

	var emote = Global.content_provider.get_wearable(values[0].urn)
	if emote == null:
		return

	var audio_file_name = emote.get_emote_audio(avatar_data.get_body_shape())
	if audio_file_name.is_empty():
		return

	var audio_file_hash = emote.get_content_mapping().get_hash(audio_file_name)
	var audio_stream = Global.content_provider.get_audio_from_hash(audio_file_hash)
	if audio_stream != null:
		audio_player_emote.stream = audio_stream
		audio_player_emote.play(0)
