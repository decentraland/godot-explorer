class_name FloatingIslandWalls
extends Node3D

# Wall configuration
@export var wall_height: float = 120.0  # Increased to accommodate 100m cliffs
@export var wall_thickness: float = 2.0
@export var debug_mode: bool = false

# Current walls
var active_walls: Array[Node3D] = []


func create_walls_for_bounds(
	scene_min_x: int, scene_max_x: int, scene_min_z: int, scene_max_z: int, padding: int
):
	# Clear any existing walls first
	clear_walls()

	# Calculate the bounds of the floating island (including padding)
	var island_min_x = scene_min_x - padding
	var island_max_x = scene_max_x + padding
	var island_min_z = scene_min_z - padding
	var island_max_z = scene_max_z + padding

	# Calculate island dimensions in world units (each parcel is 16x16 meters)
	var island_width = (island_max_x - island_min_x + 1) * 16
	var island_height = (island_max_z - island_min_z + 1) * 16

	# Calculate wall positions at the exact edges of the island
	# Parcels go from (coord * 16) to (coord * 16 + 16), so edges are at coord * 16 and (coord + 1) * 16
	var island_center_x = (island_min_x + island_max_x + 1) * 8.0
	var island_center_z = -(island_min_z + island_max_z + 1) * 8.0  # Z is flipped in Godot

	# Create 4 walls: North, South, East, West - positioned at parcel boundaries
	var wall_configs = [
		# North wall - positioned at the edge
		{
			"position":
			Vector3(
				island_center_x, -wall_height / 2 + 10, -(island_min_z * 16) + wall_thickness / 2  # Start 10m above ground, go down
			),
			"size": Vector3(island_width, wall_height, wall_thickness),
			"name": "NorthWall"
		},
		# South wall - positioned at the edge
		{
			"position":
			Vector3(
				island_center_x,
				-wall_height / 2 + 10,  # Start 10m above ground, go down
				-((island_max_z + 1) * 16) - wall_thickness / 2
			),
			"size": Vector3(island_width, wall_height, wall_thickness),
			"name": "SouthWall"
		},
		# West wall - positioned at the edge
		{
			"position":  # Start 10m above ground, go down
			Vector3(island_min_x * 16 - wall_thickness / 2, -wall_height / 2 + 10, island_center_z),
			"size": Vector3(wall_thickness, wall_height, island_height),
			"name": "WestWall"
		},
		# East wall - positioned at the edge
		{
			"position":
			Vector3(
				(island_max_x + 1) * 16 + wall_thickness / 2, -wall_height / 2 + 10, island_center_z  # Start 10m above ground, go down
			),
			"size": Vector3(wall_thickness, wall_height, island_height),
			"name": "EastWall"
		}
	]

	# Create and configure each wall
	for config in wall_configs:
		var wall = _create_wall(config.position, config.size, config.name)
		active_walls.append(wall)


func _create_wall(position: Vector3, size: Vector3, wall_name: String) -> Node3D:
	# Create wall using InvisibleWall class directly
	var wall = InvisibleWall.new()
	wall.name = wall_name

	add_child(wall)
	wall.configure_wall(size, position)

	return wall


func clear_walls():
	if active_walls.size() > 0:
		for wall in active_walls:
			if is_instance_valid(wall):
				remove_child(wall)
				wall.queue_free()

		active_walls.clear()


func set_debug_mode(enabled: bool):
	debug_mode = enabled

	# Apply debug mode to existing walls
	for wall in active_walls:
		if is_instance_valid(wall):
			if enabled and wall.has_method("_add_debug_visualization"):
				wall._add_debug_visualization()
			elif not enabled and wall.has_method("remove_debug_visualization"):
				wall.remove_debug_visualization()


func get_wall_count() -> int:
	return active_walls.size()


func _exit_tree():
	# Clean up when this node is removed
	clear_walls()
