extends Node3D

@export var skip_process: bool = false
@onready var animation_player = $AnimationPlayer
@onready var label_3d_name = $Label3D_Name
@onready var body_shape_skeleton_3d: Skeleton3D = $Skeleton3D

# Position Lerp
var last_position: Vector3 = Vector3.ZERO
var target_position: Vector3 = Vector3.ZERO
var t: float = 0.0
var target_distance: float = 0.0

# Wearable requesting
var last_request_id: int = -1
var current_content_url: String = ""
var content_waiting_hash: Array = []
var content_waiting_hash_signal_connected: bool = false

# Current wearables equippoed
var current_wearables: PackedStringArray
var current_body_shape: String = ""
var current_eyes_color: Color = Color.BLACK
var current_skin_color: Color = Color.BLACK
var current_hair_color: Color = Color.BLACK
var wearables_dict: Dictionary = {}


func _ready():
	Global.content_manager.wearable_data_loaded.connect(self._on_wearable_data_loaded)


func update_avatar(avatar: Dictionary):
	current_content_url = "https://peer.decentraland.org/content/"
	if not Global.realm.content_base_url.is_empty():
		current_content_url = Global.realm.content_base_url

	label_3d_name.text = avatar.get("name")
	current_wearables = avatar.get("wearables")
	current_body_shape = avatar.get("body_shape")
	current_eyes_color = avatar.get("eyes")
	current_skin_color = avatar.get("skin")
	current_hair_color = avatar.get("hair")

	var wearable_to_request := PackedStringArray(current_wearables)
	wearable_to_request.push_back(current_body_shape)
#	for emote in emotes:
#		var id: String = emote.get("id", "")
#		if not id.is_empty():
#			wearable_to_request.push_back(id)

	last_request_id = Global.content_manager.fetch_wearables(
		wearable_to_request, current_content_url
	)
	if last_request_id == -1:
		fetch_wearables()


func _on_wearable_data_loaded(id: int):
	if id == -1 or id != last_request_id:
		return

	fetch_wearables()


func get_representation(representation_array: Array, desired_body_shape: String) -> Dictionary:
	for representation in representation_array:
		var index = representation.get("bodyShapes", []).find(desired_body_shape)
		if index != -1:
			return representation

	return representation_array[0]


func fetch_wearables():
	# Clear last equipped werarables
	wearables_dict.clear()

	# Fill data
	wearables_dict[current_body_shape] = Global.content_manager.get_wearable(current_body_shape)
	for item in current_wearables:
		wearables_dict[item] = Global.content_manager.get_wearable(item)

	content_waiting_hash = []
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
			var fetching_resource: bool
			# TODO: should be there more extensions?
			if file_name.ends_with(".png"):
				fetching_resource = Global.content_manager.fetch_texture(file_name, content_mapping)
			else:
				fetching_resource = Global.content_manager.fetch_gltf(file_name, content_mapping)

			if fetching_resource:
				content_waiting_hash.push_back(content_to_fetch[file_name])

	if content_waiting_hash.is_empty():
		load_wearables()
	else:
		content_waiting_hash_signal_connected = true
		Global.content_manager.content_loading_finished.connect(self._on_content_loading_finished)


func _on_content_loading_finished(resource_hash: String):
	if resource_hash in content_waiting_hash:
		content_waiting_hash.erase(resource_hash)
		if content_waiting_hash.is_empty():
			load_wearables()


func try_to_set_body_shape(body_shape_hash):
	var body_shape: Node3D = Global.content_manager.get_resource_from_hash(body_shape_hash)
	if body_shape == null:
		return

	var skeleton = body_shape.find_child("Skeleton3D")
	if skeleton == null:
		return

	if body_shape_skeleton_3d != null:
		remove_child(body_shape_skeleton_3d)
		body_shape_skeleton_3d.free()

	body_shape_skeleton_3d = skeleton.duplicate()
	body_shape_skeleton_3d.name = "Skeleton3D"
	body_shape_skeleton_3d.scale = 0.01 * Vector3.ONE
	body_shape_skeleton_3d.rotate_y(-PI)
	body_shape_skeleton_3d.rotate_x(-PI / 2)

	for child in body_shape_skeleton_3d.get_children():
		child.name = child.name.to_lower()

	add_child(body_shape_skeleton_3d)


func load_wearables():
	if content_waiting_hash_signal_connected:
		content_waiting_hash_signal_connected = false
		Global.content_manager.content_loading_finished.disconnect(
			self._on_content_loading_finished
		)

	var curated_wearables = Wearables.get_curated_wearable_list(
		current_body_shape, current_wearables, []
	)
	if curated_wearables.is_empty():
		printerr("couldn't get curated wearables")
		return

	var wearables_by_category = curated_wearables[0]
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
			continue

		if child is MeshInstance3D:
			var mat_name: String = child.mesh.get("surface_0/name").to_lower()
			var material = child.mesh.surface_get_material(0)

			if material is StandardMaterial3D:
				material.metallic = 0
				material.metallic_specular = 0
				if mat_name.find("skin"):
					material.albedo_color = current_skin_color
					material.metallic = 0
				elif mat_name.find("hair"):
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
	var main_texture := ImageTexture.create_from_image(
		Global.content_manager.get_resource_from_hash(textures[0])
	)
	if textures.size() > 1:
		var mask_texture := ImageTexture.create_from_image(
			Global.content_manager.get_resource_from_hash(textures[1])
		)
		var current_material = mask_material.duplicate()

		current_material.set_shader_parameter("base_texture", main_texture)
		current_material.set_shader_parameter("mask_texture", mask_texture)
		current_material.set_shader_parameter("material_color", color)
		mesh.mesh.surface_set_material(0, current_material)
	else:
		var current_material = mesh.mesh.surface_get_material(0)
		if current_material is BaseMaterial3D:
			current_material.albedo_texture = main_texture
			current_material.albedo_color = color


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


func set_running():
	if animation_player.current_animation != "Run":
		animation_player.play("Run")


func set_idle():
	if animation_player.current_animation != "Idle":
		animation_player.play("Idle")
