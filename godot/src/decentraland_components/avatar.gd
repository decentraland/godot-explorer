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

var last_request_id: int = -1
var current_wearables: PackedStringArray
var current_body_shape: String = ""

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
	if Global.realm.content_base_url.is_empty():
		await Global.realm.realm_changed
		
	label_3d_name.text = avatar_name
	current_wearables = wearables
	current_body_shape = body_shape

	wearables.push_back(body_shape)
	for emote in emotes:
		var id: String = emote.get("id", "")
		if not id.is_empty():
			wearables.push_back(id)

	last_request_id = Global.content_manager.fetch_wearables(wearables, Global.realm.content_base_url)
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
	var wearable_map: Dictionary = {}
	wearable_map[current_body_shape] = Global.content_manager.get_wearable(current_body_shape)
	for item in current_wearables:
		wearable_map[item] = Global.content_manager.get_wearable(item)
		
	for wearable in wearable_map.values():
		if not wearable is Dictionary:
			# TODO: ???
			continue
		
		var representation_array: Array = wearable.get("metadata", {}).get("representations", [])
		if representation_array.is_empty():
			continue
			
		var representation = get_representation(representation_array, current_body_shape)
		var main_file = representation.get("mainFile", "")
		
		print(main_file)
		
