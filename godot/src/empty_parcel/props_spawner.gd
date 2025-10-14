class_name PropsSpawner
extends Node3D

@export_range(0, 100, 1) var min_props: int = 0
@export_range(0, 100, 1) var max_props: int = 1

var parent_parcel: EmptyParcel


func _ready():
	parent_parcel = get_parent()


func populate_props():
	for child in get_children():
		if child.name.begins_with("Prop_"):
			child.queue_free()

	if parent_parcel.spawn_locations.is_empty():
		return

	var prop_count = randi_range(min_props, max_props)
	if prop_count == 0:
		return

	var props_parent = EmptyParcelProps.get_node("%Props")
	var available_props = props_parent.get_child_count()

	var available_indices = []
	for i in range(parent_parcel.spawn_locations.size()):
		available_indices.append(i)

	for i in range(prop_count):
		if available_indices.is_empty():
			break

		var random_idx = randi() % available_indices.size()
		var spawn_idx = available_indices[random_idx]
		available_indices.remove_at(random_idx)

		var spawn_location = parent_parcel.spawn_locations[spawn_idx]
		var prop_pos = spawn_location.position
		var floor_normal = spawn_location.normal
		var prop_normal = floor_normal.slerp(Vector3.UP, 0.5)

		var prop_to_duplicate = props_parent.get_child(randi() % available_props)
		var chosen_prop = prop_to_duplicate.duplicate()
		chosen_prop.name = "Prop_%d" % i
		add_child(chosen_prop)

		var final_scale = 0.8 + randf() * 0.4
		chosen_prop.transform = ParcelUtils.create_aligned_transform(
			prop_pos, prop_normal, true, final_scale
		)
		for child in chosen_prop.get_children():
			if child is StaticBody3D:
				child.set_collision_layer_value(2, true)
