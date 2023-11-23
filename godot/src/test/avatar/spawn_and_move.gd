class_name TestSpawnAndMoveAvatars
extends Node

const TEST_AVATAR_N: int = 100
var moving_avatars: bool = false
var moving_t: float = 0
var spawning_avatars: bool = false
var spawning_i: int = 0
var spawning_position: Array[Vector3] = []

var wearable_data = {}


func set_wearable_data(_wearable_data):
	wearable_data = _wearable_data


func get_random_body():
	var to_pick = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if Wearables.get_category(wearable) == Wearables.Categories.BODY_SHAPE:
			to_pick.push_back(wearable_id)

	return to_pick.pick_random()


func get_random_wearable(category: String, body_shape_id: String):
	var to_pick = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if Wearables.get_category(wearable) == category:
			if Wearables.can_equip(wearable, body_shape_id):
				to_pick.push_back(wearable_id)

	return to_pick.pick_random()


func _process(dt):
	if spawning_avatars and spawning_i < TEST_AVATAR_N:
		var body_shape_id = get_random_body()
		var avatar_data = {
			"base_url": "https://peer.decentraland.org/content",
			"name": "Avatar#" + str(spawning_i),
			"body_shape": body_shape_id,
			"eyes": Color(randf(), randf(), randf()),
			"hair": Color(randf(), randf(), randf()),
			"skin": Color(0.8, 0.6078, 0.4667, 1),
			"wearables":
			[
				get_random_wearable(Wearables.Categories.MOUTH, body_shape_id),
				get_random_wearable(Wearables.Categories.HAIR, body_shape_id),
				get_random_wearable(Wearables.Categories.UPPER_BODY, body_shape_id),
				get_random_wearable(Wearables.Categories.LOWER_BODY, body_shape_id),
				get_random_wearable(Wearables.Categories.FEET, body_shape_id),
				get_random_wearable(Wearables.Categories.EYES, body_shape_id),
			],
			"emotes": []
		}

		var initial_position := (
			Vector3(randf_range(-10, 10), 0.0, randf_range(-10, 10)).normalized()
		)
		var transform = Transform3D(Basis.IDENTITY, initial_position)
		var alias = 10000 + spawning_i
		Global.avatars.add_avatar(alias, "")
		Global.avatars.update_avatar_profile(alias, avatar_data)
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
