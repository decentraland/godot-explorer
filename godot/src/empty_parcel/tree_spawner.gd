class_name TreeSpawner
extends Node3D

const MIN_TREE_SCALE: float = 1.0
const MAX_TREE_SCALE: float = 2.0

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
		var final_scale = randf_range(MIN_TREE_SCALE, MAX_TREE_SCALE)
		var final_transform = ParcelUtils.create_aligned_transform(
			tree_pos, tree_normal, true, final_scale
		)

		# Check if tree would overlap with loaded adjacent parcels before spawning
		if _tree_would_overlap_loaded_parcel(tree_to_duplicate, final_transform):
			continue

		var chosen_tree = tree_to_duplicate.duplicate()
		chosen_tree.name = "Tree_%d" % i
		add_child(chosen_tree)

		chosen_tree.transform = final_transform
		for child in chosen_tree.get_children():
			if child is StaticBody3D:
				child.set_collision_layer_value(parent_parcel.OBSTACLE_COLLISION_LAYER, true)


func _tree_would_overlap_loaded_parcel(tree_template: Node3D, tree_transform: Transform3D) -> bool:
	# Transform tree_transform to world space by combining with parent parcel's global transform
	var world_transform = parent_parcel.global_transform * tree_transform

	# Get the tree's combined AABB from all VisualInstance3D children and transform to world space
	var world_aabb = _collect_tree_aabb_in_world_space(tree_template, world_transform)

	if world_aabb == null:
		return false

	# Get the parcel's world position and corner configuration
	var parcel_pos = parent_parcel.global_position
	var config = parent_parcel.corner_config

	# Define parcel boundaries in world space
	# Parcel is centered at parcel_pos with size PARCEL_SIZE x PARCEL_SIZE
	var parcel_min = Vector3(
		parcel_pos.x - parent_parcel.PARCEL_HALF_SIZE,
		parcel_pos.y - parent_parcel.PARCEL_HEIGHT_BOUND,
		parcel_pos.z - parent_parcel.PARCEL_HALF_SIZE
	)
	var parcel_max = Vector3(
		parcel_pos.x + parent_parcel.PARCEL_HALF_SIZE,
		parcel_pos.y + parent_parcel.PARCEL_HEIGHT_BOUND,
		parcel_pos.z + parent_parcel.PARCEL_HALF_SIZE
	)

	# Check each adjacent direction that has a LOADED parcel
	# North is -Z direction
	if config.north == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_min.x, parcel_min.y, parcel_min.z - parent_parcel.PARCEL_SIZE),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	# South is +Z direction
	if config.south == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_min.x, parcel_min.y, parcel_max.z),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	# East is +X direction
	if config.east == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_max.x, parcel_min.y, parcel_min.z),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	# West is -X direction
	if config.west == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_min.x - parent_parcel.PARCEL_SIZE, parcel_min.y, parcel_min.z),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	# Check corner parcels
	if config.northwest == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_min.x - parent_parcel.PARCEL_SIZE, parcel_min.y, parcel_min.z - parent_parcel.PARCEL_SIZE),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	if config.northeast == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_max.x, parcel_min.y, parcel_min.z - parent_parcel.PARCEL_SIZE),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	if config.southwest == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_min.x - parent_parcel.PARCEL_SIZE, parcel_min.y, parcel_max.z),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	if config.southeast == CornerConfiguration.ParcelState.LOADED:
		var adjacent_aabb = AABB(
			Vector3(parcel_max.x, parcel_min.y, parcel_max.z),
			Vector3(parent_parcel.PARCEL_SIZE, parent_parcel.PARCEL_FULL_HEIGHT, parent_parcel.PARCEL_SIZE)
		)
		if world_aabb.intersects(adjacent_aabb):
			return true

	return false


func _collect_tree_aabb_in_world_space(node: Node3D, accumulated_transform: Transform3D):
	var combined_aabb = null

	if node is VisualInstance3D:
		var local_aabb = node.get_aabb()
		var full_transform = accumulated_transform * node.transform

		# Transform the 8 corners of this AABB to world space
		var corners = [
			local_aabb.position,
			local_aabb.position + Vector3(local_aabb.size.x, 0, 0),
			local_aabb.position + Vector3(0, local_aabb.size.y, 0),
			local_aabb.position + Vector3(0, 0, local_aabb.size.z),
			local_aabb.position + Vector3(local_aabb.size.x, local_aabb.size.y, 0),
			local_aabb.position + Vector3(local_aabb.size.x, 0, local_aabb.size.z),
			local_aabb.position + Vector3(0, local_aabb.size.y, local_aabb.size.z),
			local_aabb.end
		]

		for corner in corners:
			var world_corner = full_transform * corner
			if combined_aabb == null:
				combined_aabb = AABB(world_corner, Vector3.ZERO)
			else:
				combined_aabb = combined_aabb.expand(world_corner)

	# Recurse through children
	for child in node.get_children():
		if child is Node3D:
			var child_aabb = _collect_tree_aabb_in_world_space(
				child, accumulated_transform * node.transform
			)
			if child_aabb != null:
				if combined_aabb == null:
					combined_aabb = child_aabb
				else:
					combined_aabb = combined_aabb.merge(child_aabb)

	return combined_aabb
