extends Node3D

@export var skip_process: bool = false
@onready var animation_player = $AnimationPlayer
@onready var label_3d_name = $Label3D_Name

var last_position: Vector3 = Vector3.ZERO
var target_position: Vector3 = Vector3.ZERO
var t: float = 0.0
var target_distance: float = 0.0

var first_position = false

func _ready():
	Global.content_manager.wearable_data_loaded.connect(self._on_wearable_data_loaded)

var last_request_id: int = -1
var current_wearables: PackedStringArray
var current_body_shape: String = ""
var current_content_url: String = ""
var content_waiting_hash: Array = []
var content_waiting_hash_signal_connected: bool = false
var wearables_dict: Dictionary = {}

func update_avatar(
	base_url: String,
	avatar_name: String,
	body_shape: String,
	_eyes: Color,
	_hair: Color,
	_skin: Color,
	wearables: PackedStringArray,
	emotes: Array
):
	current_content_url = "https://peer.decentraland.org/content/"
	if not Global.realm.content_base_url.is_empty():
		current_content_url = Global.realm.content_base_url
		
	label_3d_name.text = avatar_name
	current_wearables = wearables
	current_body_shape = body_shape

	wearables.push_back(body_shape)
	for emote in emotes:
		var id: String = emote.get("id", "")
		if not id.is_empty():
			wearables.push_back(id)

	last_request_id = Global.content_manager.fetch_wearables(wearables, current_content_url)
	if last_request_id == -1:
		try_to_load_wearables()

func _on_wearable_data_loaded(id: int):
	if id == -1 or id != last_request_id:
		return
		
	try_to_load_wearables()

func get_representation(representation_array: Array, desired_body_shape: String) -> Dictionary:
	for representation in representation_array:
		var index = representation.get("bodyShapes", []).find(desired_body_shape)
		if index != -1:
			return representation
		
	return representation_array[0]

func try_to_load_wearables():
	wearables_dict.clear()
	
	var wearable_map: Dictionary = {}
	wearable_map[current_body_shape] = Global.content_manager.get_wearable(current_body_shape)
		
	for item in current_wearables:
		wearable_map[item] = Global.content_manager.get_wearable(item)
		
	content_waiting_hash = []
	for wearable_key in wearable_map:
		var wearable = wearable_map[wearable_key]
		if not wearable is Dictionary:
			# TODO: ???
			print("wearable not dictionary")
			continue
		
		var representation_array: Array = wearable.get("metadata", {}).get("data", {}).get("representations", [])
		if representation_array.is_empty():
			print("metadata empty")
			continue
			
		var representation: Dictionary = get_representation(representation_array, current_body_shape)
		var main_file: String = representation.get("mainFile", "").to_lower()
		
		var content_mapping: Dictionary = {
			"content": wearable.get("content", {}),
			"base_url" : "https://peer.decentraland.org/content/contents/"
		}
		
		var file_hash = content_mapping.content.get(main_file, "")
		if file_hash.is_empty():
			continue
			
		wearables_dict[wearable_key] = file_hash
		var fetching_resource: bool
		if main_file.ends_with(".png"):
			fetching_resource = Global.content_manager.fetch_texture(
				main_file, content_mapping
			)
		else:
			fetching_resource = Global.content_manager.fetch_gltf(
				main_file, content_mapping
			)
		
		if fetching_resource:
			content_waiting_hash.push_back(file_hash)

	
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

func load_wearables():
	if content_waiting_hash_signal_connected:
		content_waiting_hash_signal_connected = false
		Global.content_manager.content_loading_finished.disconnect(self._on_content_loading_finished)
	
	var body_shape = wearables_dict.get(current_body_shape)
	if body_shape == null:
		printerr("body shape not found")
		return
		
	body_shape = Global.content_manager.get_resource_from_hash(body_shape.file_hash)
	var body_shape_instance = body_shape.duplicate()
	add_child(body_shape)
	

func set_target(target: Transform3D) -> void:
	if not first_position:
		first_position = true
		self.global_transform = target
		last_position = target.origin
		return

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
