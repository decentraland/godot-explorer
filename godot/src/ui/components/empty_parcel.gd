class_name EmptyParcel
extends Node3D

enum CliffDirection { NONE, NORTH, EAST, SOUTH, WEST, NORTHEAST, NORTHWEST, SOUTHEAST, SOUTHWEST }

@export var cliff: CliffDirection = CliffDirection.NONE

# Preload the cliff scene
const CliffScene = preload("res://assets/cliff/cliff.tscn")


func set_cliff_direction(direction: CliffDirection) -> void:
	cliff = direction
	print(
		(
			"Empty parcel at %s set cliff direction: %s"
			% [global_position, CliffDirection.keys()[cliff]]
		)
	)
	_spawn_cliff_scenes()


func get_cliff_direction() -> CliffDirection:
	return cliff


func add_edge_indicator() -> void:
	# Create a cube with color based on cliff direction
	var mesh_instance = MeshInstance3D.new()
	mesh_instance.name = "EdgeIndicator"

	# Create a small cube mesh
	var box_mesh = BoxMesh.new()
	box_mesh.size = Vector3(1, 1, 1)  # 1x1x1 meter cube
	mesh_instance.mesh = box_mesh

	# Choose color based on cliff direction
	var indicator_color = _get_cliff_color(cliff)

	# Create material with cliff-specific color
	var material = StandardMaterial3D.new()
	material.albedo_color = indicator_color
	material.emission_enabled = true
	material.emission = indicator_color * 0.3  # Slight glow
	mesh_instance.material_override = material

	# Position at center, slightly above ground
	mesh_instance.position = Vector3(0, 1, 0)

	add_child(mesh_instance)
	print(
		(
			"Added %s edge indicator cube to parcel at %s"
			% [CliffDirection.keys()[cliff], global_position]
		)
	)


func _get_cliff_color(direction: CliffDirection) -> Color:
	match direction:
		CliffDirection.NORTH:
			return Color.BLUE  # North = Blue (cool)
		CliffDirection.SOUTH:
			return Color.RED  # South = Red (warm)
		CliffDirection.EAST:
			return Color.YELLOW  # East = Yellow (sunrise)
		CliffDirection.WEST:
			return Color.ORANGE  # West = Orange (sunset)
		CliffDirection.NORTHEAST:
			return Color.CYAN  # Northeast = Cyan (blue + green)
		CliffDirection.NORTHWEST:
			return Color.PURPLE  # Northwest = Purple (blue + red)
		CliffDirection.SOUTHEAST:
			return Color.GREEN  # Southeast = Green (yellow + blue)
		CliffDirection.SOUTHWEST:
			return Color.MAGENTA  # Southwest = Magenta (red + blue)
		_:
			return Color.WHITE  # Default/NONE = White


func remove_edge_indicator() -> void:
	var indicator = get_node_or_null("EdgeIndicator")
	if indicator:
		indicator.queue_free()


func _spawn_cliff_scenes() -> void:
	# Clear existing cliff scenes
	_clear_cliff_scenes()

	# Don't spawn anything if no cliff direction
	if cliff == CliffDirection.NONE:
		return

	# Handle corner cases (spawn 2 cliff scenes)
	if (
		cliff
		in [
			CliffDirection.NORTHEAST,
			CliffDirection.NORTHWEST,
			CliffDirection.SOUTHEAST,
			CliffDirection.SOUTHWEST
		]
	):
		_spawn_corner_cliffs()
	else:
		# Handle single direction (spawn 1 cliff scene)
		_spawn_single_cliff()


func _spawn_corner_cliffs() -> void:
	match cliff:
		CliffDirection.NORTHEAST:
			_create_positioned_cliff(CliffDirection.NORTH)
			_create_positioned_cliff(CliffDirection.EAST)
		CliffDirection.NORTHWEST:
			_create_positioned_cliff(CliffDirection.NORTH)
			_create_positioned_cliff(CliffDirection.WEST)
		CliffDirection.SOUTHEAST:
			_create_positioned_cliff(CliffDirection.SOUTH)
			_create_positioned_cliff(CliffDirection.EAST)
		CliffDirection.SOUTHWEST:
			_create_positioned_cliff(CliffDirection.SOUTH)
			_create_positioned_cliff(CliffDirection.WEST)


func _spawn_single_cliff() -> void:
	_create_positioned_cliff(cliff)


func _create_positioned_cliff(direction: CliffDirection) -> void:
	var cliff_instance = CliffScene.instantiate()
	cliff_instance.name = "Cliff_%s" % CliffDirection.keys()[direction]

	# Position and rotate the cliff based on direction
	var position_offset = Vector3.ZERO
	var rotation_degrees = Vector3.ZERO

	match direction:
		CliffDirection.NORTH:
			position_offset = Vector3(0, 0, 8)  # North outer boundary of parcel
			rotation_degrees = Vector3(0, 0, 0)  # Default orientation faces +X (outward from north)
		CliffDirection.SOUTH:
			position_offset = Vector3(0, 0, -8)  # South outer boundary of parcel
			rotation_degrees = Vector3(0, 180, 0)  # Rotate 180° to face -X (outward from south)
		CliffDirection.EAST:
			position_offset = Vector3(8, 0, 0)  # East outer boundary of parcel
			rotation_degrees = Vector3(0, 90, 0)  # Rotate 90° to face +Z (outward from east)
		CliffDirection.WEST:
			position_offset = Vector3(-8, 0, 0)  # West outer boundary of parcel
			rotation_degrees = Vector3(0, -90, 0)  # Rotate -90° to face -Z (outward from west)

	cliff_instance.position = position_offset
	cliff_instance.rotation_degrees = rotation_degrees

	add_child(cliff_instance)
	print("Spawned cliff at %s with rotation %s" % [position_offset, rotation_degrees])


func _clear_cliff_scenes() -> void:
	# Remove all existing cliff scenes
	for child in get_children():
		if child.name.begins_with("Cliff_"):
			child.queue_free()
