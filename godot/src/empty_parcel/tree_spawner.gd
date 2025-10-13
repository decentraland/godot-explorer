class_name TreeSpawner
extends Node3D

@export_range(0, 100, 1) var min_trees: int = 0
@export_range(0, 100, 1) var max_trees: int = 0
@export_range(0.0, 1.0, 0.1) var min_falloff_for_spawning: float = 0.3

var parent_parcel: EmptyParcel


func _ready():
	parent_parcel = get_parent()


func populate_trees():
	for child in get_children():
		if child.name.begins_with("Tree_"):
			child.queue_free()

	if parent_parcel.spawn_locations.is_empty():
		return

	var tree_count = randi_range(min_trees, max_trees)
	if tree_count == 0:
		return

	var trees_parent = EmptyParcelProps.get_node("%Trees")
	var available_trees = trees_parent.get_child_count()

	var available_indices = []
	for i in range(parent_parcel.spawn_locations.size()):
		if parent_parcel.spawn_locations[i].falloff > min_falloff_for_spawning:
			available_indices.append(i)

	for i in range(tree_count):
		if available_indices.is_empty():
			break

		var random_idx = randi() % available_indices.size()
		var spawn_idx = available_indices[random_idx]
		available_indices.remove_at(random_idx)

		var spawn_location = parent_parcel.spawn_locations[spawn_idx]
		var tree_pos = spawn_location.position
		var tree_normal = Vector3.UP

		var tree_to_duplicate = trees_parent.get_child(randi() % available_trees)
		var chosen_tree = tree_to_duplicate.duplicate()
		chosen_tree.name = "Tree_%d" % i
		add_child(chosen_tree)

		var final_scale = randf_range(1.0, 2.0)
		chosen_tree.transform = ParcelUtils.create_aligned_transform(
			tree_pos, tree_normal, true, final_scale
		)
		for child in chosen_tree.get_children():
			if child is StaticBody3D:
				child.set_collision_layer_value(2, true)
