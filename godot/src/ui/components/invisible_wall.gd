class_name InvisibleWall
extends StaticBody3D

@export var wall_size: Vector3 = Vector3(1, 10, 1)
@export var wall_position: Vector3 = Vector3.ZERO

var collision_shape: CollisionShape3D
var box_shape: BoxShape3D


func _ready():
	_ensure_collision_setup()
	update_wall_configuration()


func _ensure_collision_setup():
	# Set collision layers - walls should block players but not interfere with other systems
	collision_layer = 2  # Layer 2 for walls - this is what player collides with
	collision_mask = 0  # Walls don't need to detect anything

	# Create collision shape if not already created
	if not collision_shape:
		collision_shape = CollisionShape3D.new()
		collision_shape.name = "WallCollision"
		add_child(collision_shape)

	if not box_shape:
		box_shape = BoxShape3D.new()
		collision_shape.shape = box_shape

	# Ensure the collision shape is properly enabled
	set_collision_layer_value(2, true)  # Enable layer 2
	set_collision_mask_value(1, false)  # Don't detect anything


func configure_wall(size: Vector3, pos: Vector3):
	wall_size = size
	wall_position = pos
	global_position = pos

	# Ensure collision components are set up
	_ensure_collision_setup()
	update_wall_configuration()


func update_wall_configuration():
	if box_shape:
		box_shape.size = wall_size
	if collision_shape:
		collision_shape.position = Vector3.ZERO  # Shape is centered on the StaticBody3D

	print(
		(
			"InvisibleWall '%s' configured: pos=%s, size=%s, collision_layer=%d"
			% [name, global_position, wall_size, collision_layer]
		)
	)


func _add_debug_visualization():
	# Remove existing debug mesh if any
	var existing_debug = get_node_or_null("DebugMesh")
	if existing_debug:
		existing_debug.queue_free()

	# Create a semi-transparent debug mesh to visualize walls in debug mode
	var mesh_instance = MeshInstance3D.new()
	mesh_instance.name = "DebugMesh"

	var box_mesh = BoxMesh.new()
	box_mesh.size = wall_size
	mesh_instance.mesh = box_mesh

	# Create semi-transparent red material
	var material = StandardMaterial3D.new()
	material.albedo_color = Color(1, 0, 0, 0.3)  # Semi-transparent red
	material.flags_transparent = true
	material.no_depth_test = true
	mesh_instance.material_override = material

	add_child(mesh_instance)


func remove_debug_visualization():
	var debug_mesh = get_node_or_null("DebugMesh")
	if debug_mesh:
		debug_mesh.queue_free()
