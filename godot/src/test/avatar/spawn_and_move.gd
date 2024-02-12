class_name TestSpawnAndMoveAvatars
extends Node

const TEST_AVATAR_N: int = 100
var moving_avatars: bool = false
var moving_t: float = 0
var spawning_avatars: bool = false
var spawning_i: int = 0
var spawning_position: Array[Vector3] = []

var wearable_data = {}


# gdlint:ignore = async-function-name
func _ready():
	for wearable_id in Wearables.BASE_WEARABLES:
		var key = Wearables.get_base_avatar_urn(wearable_id)
		wearable_data[key] = null

	var promise = Global.content_provider.fetch_wearables(
		wearable_data.keys(), "https://peer.decentraland.org/content/"
	)
	await PromiseUtils.async_all(promise)

	for wearable_id in wearable_data:
		wearable_data[wearable_id] = Global.content_provider.get_wearable(wearable_id)
		if wearable_data[wearable_id] == null:
			printerr("Error loading wearable_id ", wearable_id)

	self.spawning_avatars = true


func set_wearable_data(_wearable_data):
	wearable_data = _wearable_data


func get_random_body():
	var to_pick = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable.get_category() == Wearables.Categories.BODY_SHAPE:
			to_pick.push_back(wearable_id)

	return to_pick.pick_random()


func get_random_wearable(category: String, body_shape_id: String):
	var to_pick = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable.get_category() == category:
			if Wearables.can_equip(wearable, body_shape_id):
				to_pick.push_back(wearable_id)

	return to_pick.pick_random()


func generate_random_address() -> String:
	var address_length = 42  # Adjust the length based on your needs
	var characters = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
	var address = ""
	for i in range(address_length):
		var random_index = randi() % characters.length()
		address += characters.substr(random_index, 1)

	return address


func _process(dt):
	if spawning_avatars and spawning_i < TEST_AVATAR_N:
		var body_shape_id = get_random_body()

		var profile_data: DclUserProfile = DclUserProfile.new()
		var avatar_data: DclAvatarWireFormat = profile_data.get_avatar()

		avatar_data.set_name("Avatar#" + str(spawning_i))
		avatar_data.set_eyes_color(Color(randf(), randf(), randf()))
		avatar_data.set_hair_color(Color(randf(), randf(), randf()))
		avatar_data.set_skin_color(Color(0.8, 0.6078, 0.4667, 1))
		avatar_data.set_body_shape(body_shape_id)
		profile_data.set_avatar(avatar_data)
		var avatar_wearables := PackedStringArray(
			[
				get_random_wearable(Wearables.Categories.MOUTH, body_shape_id),
				get_random_wearable(Wearables.Categories.HAIR, body_shape_id),
				get_random_wearable(Wearables.Categories.UPPER_BODY, body_shape_id),
				get_random_wearable(Wearables.Categories.LOWER_BODY, body_shape_id),
				get_random_wearable(Wearables.Categories.FEET, body_shape_id),
				get_random_wearable(Wearables.Categories.EYES, body_shape_id),
			]
		)
		avatar_data.set_wearables(avatar_wearables)

		var initial_position := (
			Vector3(randf_range(-10, 10), 0.0, randf_range(-10, 10)).normalized()
		)
		var transform = Transform3D(Basis.IDENTITY, initial_position)
		var alias = 10000 + spawning_i
		var address := generate_random_address()
		Global.avatars.add_avatar(alias, address)
		Global.avatars.update_dcl_avatar_by_alias(alias, profile_data)
		Global.avatars.update_avatar_transform_with_godot_transform(alias, transform)

		spawning_position.append(initial_position)

		spawning_i = spawning_i + 1
		if spawning_i >= TEST_AVATAR_N:
			spawning_avatars = false
			moving_avatars = true
	elif moving_avatars:
		moving_t += dt
		if moving_t >= 0.1:
			var walk_speed = 2.0
			var walk_delta = walk_speed * moving_t
			moving_t = 0
			for i in range(spawning_position.size()):
				var delta_position = (
					Vector3(randf_range(-1, 1), 0.0, randf_range(-1, 1)).normalized() * walk_delta
				)
				var current_position = spawning_position[i]
				var target_position = current_position + delta_position
				var transform = Transform3D(Basis.IDENTITY, current_position)
				transform = transform.looking_at(target_position)
				transform.origin = target_position

				spawning_position[i] = target_position
				var alias = 10000 + i
				Global.avatars.update_avatar_transform_with_godot_transform(alias, transform)
