class_name RockSpawner
extends Node3D

@export_range(0, 100, 1) var min_rocks: int = 0
@export_range(0, 100, 1) var max_rocks: int = 1
@export_range(0.0, 1.0, 0.1) var min_falloff_for_spawning: float = 0.3

var parent_parcel: EmptyParcel


func _ready():
	parent_parcel = get_parent()


func populate_rocks():
	for child in get_children():
		if child.name.begins_with("Rock_"):
			child.queue_free()

	if parent_parcel.spawn_locations.is_empty():
		return

	var rock_count = randi_range(min_rocks, max_rocks)
	if rock_count == 0:
		return

	var available_indices = []
	for i in range(parent_parcel.spawn_locations.size()):
		if parent_parcel.spawn_locations[i].falloff > min_falloff_for_spawning:
			available_indices.append(i)

	for i in range(rock_count):
		if available_indices.is_empty():
			break

		var random_idx = randi() % available_indices.size()
		var spawn_idx = available_indices[random_idx]
		available_indices.remove_at(random_idx)

		var spawn_location = parent_parcel.spawn_locations[spawn_idx]
		var rock_pos = spawn_location.position
		var rock_normal = spawn_location.normal
		var rocks_parent = EmptyParcelProps.get_node("%Rocks")
		var available_rocks = rocks_parent.get_child_count()

		var rock_to_duplicate = rocks_parent.get_child(randi() % available_rocks)
		# Use DUPLICATE_USE_INSTANTIATION to share materials and reduce descriptor set allocations
		var chosen_rock = rock_to_duplicate.duplicate(DUPLICATE_USE_INSTANTIATION)
		chosen_rock.name = "Rock_%d" % i
		add_child(chosen_rock)

		var final_scale = 1.0 + randf() * 1.0
		chosen_rock.transform = ParcelUtils.create_aligned_transform(
			rock_pos, rock_normal, true, final_scale
		)
		var collision_body = chosen_rock.get_node("StaticBody3D")
		if collision_body:
			collision_body.set_collision_layer_value(2, true)
