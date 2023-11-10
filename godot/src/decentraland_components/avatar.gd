extends Node3D
class_name Avatar

signal avatar_loaded

@export var skip_process: bool = false
@onready var animation_player = $Armature/AnimationPlayer
@onready var label_3d_name = $Label3D_Name
@onready var body_shape_root: Node3D = $Armature
@onready var body_shape_skeleton_3d: Skeleton3D = $Armature/Skeleton3D

# Public
var avatar_name: String = ""
var playing_emote = false

# Position Lerp
var last_position: Vector3 = Vector3.ZERO
var target_position: Vector3 = Vector3.ZERO
var t: float = 0.0
var target_distance: float = 0.0

# Wearable requesting
var current_content_url: String = ""

# Current wearables equippoed
var current_wearables: PackedStringArray
var current_body_shape: String = ""
var current_eyes_color: Color = Color.BLACK
var current_skin_color: Color = Color.BLACK
var current_hair_color: Color = Color.BLACK
var wearables_dict: Dictionary = {}

var finish_loading = false
var wearables_by_category


func update_avatar(avatar: Dictionary):
	current_content_url = "https://peer.decentraland.org/content/"
	if not Global.realm.content_base_url.is_empty():
		current_content_url = Global.realm.content_base_url

	playing_emote = false
	set_idle()

	avatar_name = avatar.get("name")
	label_3d_name.text = avatar_name
	current_wearables = avatar.get("wearables")
	current_body_shape = avatar.get("body_shape")
	current_eyes_color = avatar.get("eyes")
	current_skin_color = avatar.get("skin")
	current_hair_color = avatar.get("hair")

	var wearable_to_request := PackedStringArray(current_wearables)
	wearable_to_request.push_back(current_body_shape)

	_load_default_emotes()

	finish_loading = false

	var promise = Global.content_manager.fetch_wearables(wearable_to_request, current_content_url)
	await promise.co_awaiter()
	fetch_wearables_dependencies()


@onready var global_animation_library: AnimationLibrary = animation_player.get_animation_library("")
var index_to_animation_name: Dictionary = {}


func _add_animation(index: int, animation_name: String):
	var animation = Global.animation_importer.get_animation_from_gltf(animation_name)
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


func play_emote(emote_id: String):
	if animation_player.has_animation(emote_id):
		animation_player.stop()
		animation_player.play(emote_id)
	else:
		printerr("Emote %s not found from player '%s'" % [emote_id, avatar_name])
	playing_emote = true


func play_remote_emote(emote_src: String, looping: bool):
	# TODO: Implement downloading emote from the scene content, adding to the avatar and then playing the emote
	# Test scene: https://github.com/decentraland/unity-renderer/pull/5501
	pass


func play_emote_by_index(index: int):
	# Play emote
	var emote_id: String = index_to_animation_name[index]
	play_emote(emote_id)


func broadcast_avatar_animation():
	# Send emote
	var timestamp = Time.get_unix_time_from_system() * 1000
	Global.comms.send_chat("␐%s %d" % [animation_player.current_animation, timestamp])


func update_colors(eyes_color: Color, skin_color: Color, hair_color: Color) -> void:
	current_eyes_color = eyes_color
	current_skin_color = skin_color
	current_hair_color = hair_color

	if finish_loading:
		apply_color_and_facial()


func get_representation(representation_array: Array, desired_body_shape: String) -> Dictionary:
	for representation in representation_array:
		var index = representation.get("bodyShapes", []).find(desired_body_shape)
		if index != -1:
			return representation

	return representation_array[0]


func fetch_wearables_dependencies():
	# Clear last equipped werarables
	wearables_dict.clear()

	# Fill data
	wearables_dict[current_body_shape] = Global.content_manager.get_wearable(current_body_shape)
	for item in current_wearables:
		wearables_dict[item] = Global.content_manager.get_wearable(item)

	var async_calls: Array = []
	for wearable_key in wearables_dict.keys():
		if not wearables_dict[wearable_key] is Dictionary:
			printerr("wearable ", wearable_key, " not dictionary")
			continue

		var wearable = wearables_dict[wearable_key]
		if not Wearables.is_valid_wearable(wearable, current_body_shape, true):
			continue

		var hashes_to_fetch: Array
		if Wearables.is_texture(Wearables.get_category(wearable)):
			hashes_to_fetch = Wearables.get_wearable_facial_hashes(wearable, current_body_shape)
		else:
			hashes_to_fetch = [Wearables.get_wearable_main_file_hash(wearable, current_body_shape)]

		if hashes_to_fetch.is_empty():
			continue

		var content: Dictionary = wearable.get("content", {})
		var content_to_fetch := {}
		for file_name in content:
			for file_hash in hashes_to_fetch:
				if content[file_name] == file_hash:
					content_to_fetch[file_name] = file_hash

		var content_mapping: Dictionary = {
			"content": wearable.get("content", {}),
			"base_url": "https://peer.decentraland.org/content/contents/"
		}

		for file_name in content_to_fetch:
			async_calls.push_back(_fetch_texture_or_gltf(file_name, content_mapping))

	await Promise.co_all(async_calls)

	load_wearables()


func _fetch_texture_or_gltf(file_name, content_mapping):
	var promise: Promise

	if file_name.ends_with(".png"):
		promise = Global.content_manager.fetch_texture(file_name, content_mapping)
	else:
		promise = Global.content_manager.fetch_gltf(file_name, content_mapping)

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
	var body_shape: Node3D = Global.content_manager.get_resource_from_hash(body_shape_hash)
	if body_shape == null:
		return

	var skeleton = body_shape.find_child("Skeleton3D")
	if skeleton == null:
		return

	var animation_player_parent = animation_player.get_parent()
	if animation_player_parent != null:
		animation_player_parent.remove_child(animation_player)

	if body_shape_root != null:
		remove_child(body_shape_root)
		_free_old_skeleton.call_deferred(body_shape_root)

	body_shape_root = body_shape.duplicate()
	body_shape_root.name = "BodyShape"

	body_shape_skeleton_3d = body_shape_root.find_child("Skeleton3D", true, false)
	body_shape_skeleton_3d.get_parent().add_child(animation_player)

	for child in body_shape_skeleton_3d.get_children():
		child.name = child.name.to_lower()

	body_shape_skeleton_3d.visible = false
	add_child(body_shape_root)

	_add_attach_points()


func load_wearables():
	var curated_wearables = Wearables.get_curated_wearable_list(
		current_body_shape, current_wearables, []
	)
	if curated_wearables.is_empty():
		printerr("couldn't get curated wearables")
		return

	wearables_by_category = curated_wearables[0]
	# var hidden_categories = curated_wearables[1]

	var body_shape = wearables_by_category.get(Wearables.Categories.BODY_SHAPE)
	if body_shape == null:
		printerr("body shape not found")
		return

	try_to_set_body_shape(Wearables.get_wearable_main_file_hash(body_shape, current_body_shape))
	wearables_by_category.erase(Wearables.Categories.BODY_SHAPE)

	var has_skin = false
	var hide_upper_body = false
	var hide_lower_body = false
	var hide_feet = false
	var hide_head = false

	for category in wearables_by_category:
		var wearable = wearables_by_category[category]

		# Skip
		if Wearables.is_texture(category):
			continue

		var file_hash = Wearables.get_wearable_main_file_hash(wearable, current_body_shape)
		var obj = Global.content_manager.get_resource_from_hash(file_hash)
		var wearable_skeleton: Skeleton3D = obj.find_child("Skeleton3D")
		for child in wearable_skeleton.get_children():
			var new_wearable = child.duplicate()
			new_wearable.name = new_wearable.name.to_lower()
			body_shape_skeleton_3d.add_child(new_wearable)

		match category:
			Wearables.Categories.UPPER_BODY:
				hide_upper_body = true
			Wearables.Categories.LOWER_BODY:
				hide_lower_body = true
			Wearables.Categories.FEET:
				hide_feet = true
			Wearables.Categories.HEAD:
				hide_head = true
			Wearables.Categories.SKIN:
				has_skin = true

	var hidings = {
		"ubody_basemesh": has_skin or hide_upper_body,
		"lbody_basemesh": has_skin or hide_lower_body,
		"feet_basemesh": has_skin or hide_feet,
		"head": has_skin or hide_head,
		"head_basemesh": has_skin or hide_head,
		"mask_eyes": has_skin or hide_head,
		"mask_eyebrows": has_skin or hide_head,
		"mask_mouth": has_skin or hide_head,
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

	var promise = Global.content_manager.duplicate_materials(meshes)
	await promise.co_awaiter()
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
						material.albedo_color = current_skin_color
						material.metallic = 0
					elif mat_name.find("hair") != -1:
						material.roughness = 1
						material.albedo_color = current_hair_color

	var eyes = wearables_by_category.get(Wearables.Categories.EYES)
	var eyebrows = wearables_by_category.get(Wearables.Categories.EYEBROWS)
	var mouth = wearables_by_category.get(Wearables.Categories.MOUTH)
	self.apply_facial_features_to_meshes(eyes, eyebrows, mouth)


func apply_facial_features_to_meshes(wearable_eyes, wearable_eyebrows, wearable_mouth):
	var eyes = Wearables.get_wearable_facial_hashes(wearable_eyes, current_body_shape)
	var eyebrows = Wearables.get_wearable_facial_hashes(wearable_eyebrows, current_body_shape)
	var mouth = Wearables.get_wearable_facial_hashes(wearable_mouth, current_body_shape)

	for child in body_shape_skeleton_3d.get_children():
		if not child.visible or not child is MeshInstance3D:
			continue

		if child.name.ends_with("mask_eyes"):
			if not eyes.is_empty():
				apply_texture_and_mask(child, eyes, current_eyes_color, Color.WHITE)
			else:
				child.hide()
		elif child.name.ends_with("mask_eyebrows"):
			if not eyebrows.is_empty():
				apply_texture_and_mask(child, eyebrows, current_hair_color, Color.BLACK)
			else:
				child.hide()
			pass
		elif child.name.ends_with("mask_mouth"):
			if not mouth.is_empty():
				apply_texture_and_mask(child, mouth, current_skin_color, Color.BLACK)
			else:
				child.hide()


var mask_material = preload("res://assets/avatar/mask_material.tres")


func apply_texture_and_mask(
	mesh: MeshInstance3D, textures: Array[String], color: Color, mask_color: Color
):
	var current_material = mask_material.duplicate()
	current_material.set_shader_parameter(
		"base_texture", Global.content_manager.get_resource_from_hash(textures[0])
	)
	current_material.set_shader_parameter("material_color", color)
	current_material.set_shader_parameter("mask_color", mask_color)

	if textures.size() > 1:
		current_material.set_shader_parameter(
			"mask_texture", Global.content_manager.get_resource_from_hash(textures[1])
		)

	mesh.mesh.surface_set_material(0, current_material)


func set_target(target: Transform3D) -> void:
	target_distance = target_position.distance_to(target.origin)

	last_position = target_position
	target_position = target.origin

	self.global_rotation = target.basis.get_euler()
	self.global_position = last_position

	t = 0


func _process(delta):
	if skip_process:
		return

	if t < 2:
		t += 10 * delta
		if t < 1:
			if t > 1.0:
				t = 1.0

			self.global_position = last_position.lerp(target_position, t)
			if target_distance > 0:
				if target_distance > 0.6:
					set_running()
				else:
					set_walking()

		elif t > 1.5:
			self.set_idle()


func set_walking():
	if animation_player.current_animation != "Walk":
		animation_player.play("Walk")
		playing_emote = false


func set_running():
	if animation_player.current_animation != "Run":
		animation_player.play("Run")
		playing_emote = false


func set_idle():
	if animation_player.current_animation != "Idle" and playing_emote == false:
		animation_player.play("Idle")


var audio_stream_player: AudioStreamPlayer = null
var audio_stream_player_gen: AudioStreamGenerator = null


func spawn_voice_channel(sample_rate, num_channels, samples_per_channel):
	printt("init voice chat ", sample_rate, num_channels, samples_per_channel)
	audio_stream_player = AudioStreamPlayer.new()
	audio_stream_player_gen = AudioStreamGenerator.new()

	audio_stream_player.set_stream(audio_stream_player_gen)
	audio_stream_player_gen.mix_rate = sample_rate
	add_child(audio_stream_player)
	audio_stream_player.play()


func push_voice_frame(frame):
#	print("voice chat ", frame)
	if not audio_stream_player.playing:
		audio_stream_player.play()

	audio_stream_player.get_stream_playback().push_buffer(frame)


var generate_attach_points: bool = false
var right_hand_idx: int = -1
var right_hand_position: Transform3D
var left_hand_idx: int = -1
var left_hand_position: Transform3D


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
