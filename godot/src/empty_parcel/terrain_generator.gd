class_name TerrainGenerator
extends Node3D

signal terrain_generated

const EMPTY_PARCEL_MATERIAL = preload("res://assets/empty-scenes/empty_parcel_material.tres")

@export var terrain_height: float = 3.0

var parent_parcel: EmptyParcel


func _ready():
	parent_parcel = get_parent()


func generate_terrain():
	_create_grid_mesh()
	_generate_floor_collision()
	terrain_generated.emit()


func _create_grid_mesh():
	var existing_mesh = get_node_or_null("GridFloor")
	if existing_mesh:
		existing_mesh.queue_free()

	var mesh_instance = MeshInstance3D.new()
	mesh_instance.name = "GridFloor"
	add_child(mesh_instance)
	mesh_instance.material_override = EMPTY_PARCEL_MATERIAL

	_generate_mesh(mesh_instance)


func _generate_mesh(mesh_instance: MeshInstance3D):
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	_generate_floor_grid(surface_tool)
	surface_tool.generate_normals()
	var generated_mesh = surface_tool.commit()
	mesh_instance.mesh = generated_mesh
	mesh_instance.visible = true


func _generate_floor_grid(surface_tool: SurfaceTool):
	var grid_size = 32
	var cell_size = 0.5
	var start_pos = -8.0

	var noise = FastNoiseLite.new()
	noise.seed = 12345
	noise.frequency = 0.05
	var noise_strength = terrain_height

	parent_parcel.spawn_locations.clear()

	# First, generate the main grid
	for z in range(grid_size):
		for x in range(grid_size):
			var x_pos = start_pos + x * cell_size
			var z_pos = start_pos + z * cell_size

			var world_v1 = global_position + Vector3(x_pos, 0, z_pos)
			var world_v2 = global_position + Vector3(x_pos + cell_size, 0, z_pos)
			var world_v3 = global_position + Vector3(x_pos + cell_size, 0, z_pos + cell_size)
			var world_v4 = global_position + Vector3(x_pos, 0, z_pos + cell_size)

			var v1 = _create_displaced_vertex(
				x_pos, z_pos, world_v1.x, world_v1.z, x, z, grid_size, noise, noise_strength
			)
			var v2 = _create_displaced_vertex(
				x_pos + cell_size,
				z_pos,
				world_v2.x,
				world_v2.z,
				x + 1,
				z,
				grid_size,
				noise,
				noise_strength
			)
			var v3 = _create_displaced_vertex(
				x_pos + cell_size,
				z_pos + cell_size,
				world_v3.x,
				world_v3.z,
				x + 1,
				z + 1,
				grid_size,
				noise,
				noise_strength
			)
			var v4 = _create_displaced_vertex(
				x_pos,
				z_pos + cell_size,
				world_v4.x,
				world_v4.z,
				x,
				z + 1,
				grid_size,
				noise,
				noise_strength
			)

			var u1 = float(x) / float(grid_size)
			var v1_uv = float(z) / float(grid_size)
			var u2 = float(x + 1) / float(grid_size)
			var v2_uv = float(z + 1) / float(grid_size)

			var uv1 = Vector2(u1, v1_uv)
			var uv2 = Vector2(u2, v1_uv)
			var uv3 = Vector2(u2, v2_uv)
			var uv4 = Vector2(u1, v2_uv)

			var normal = Vector3(0, 1, 0)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv1)
			surface_tool.add_vertex(v1)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv2)
			surface_tool.add_vertex(v2)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv3)
			surface_tool.add_vertex(v3)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv1)
			surface_tool.add_vertex(v1)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv3)
			surface_tool.add_vertex(v3)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv4)
			surface_tool.add_vertex(v4)

			var normal1 = (v3 - v1).cross(v2 - v1).normalized()
			var normal2 = (v4 - v1).cross(v3 - v1).normalized()

			var point1 = _get_random_point_in_triangle(v1, v2, v3)
			var grid_x1 = int((point1.x + 8.0) / 0.5)
			var grid_z1 = int((point1.z + 8.0) / 0.5)
			grid_x1 = clamp(grid_x1, 0, grid_size - 1)
			grid_z1 = clamp(grid_z1, 0, grid_size - 1)
			var falloff1 = parent_parcel.calculate_displacement_falloff(grid_x1, grid_z1, grid_size)

			var spawn_loc1 = EmptyParcel.SpawnLocation.new(point1, normal1, falloff1)
			parent_parcel.spawn_locations.append(spawn_loc1)
			var point2 = _get_random_point_in_triangle(v1, v3, v4)
			var grid_x2 = int((point2.x + 8.0) / 0.5)
			var grid_z2 = int((point2.z + 8.0) / 0.5)
			grid_x2 = clamp(grid_x2, 0, grid_size - 1)
			grid_z2 = clamp(grid_z2, 0, grid_size - 1)
			var falloff2 = parent_parcel.calculate_displacement_falloff(grid_x2, grid_z2, grid_size)

			var spawn_loc2 = EmptyParcel.SpawnLocation.new(point2, normal2, falloff2)
			parent_parcel.spawn_locations.append(spawn_loc2)

	# Generate edge strips along loaded parcel boundaries
	_generate_loaded_parcel_edges(
		surface_tool, grid_size, cell_size, start_pos, noise, noise_strength
	)


func _get_random_point_in_triangle(v1: Vector3, v2: Vector3, v3: Vector3) -> Vector3:
	var r1 = randf()
	var r2 = randf()
	if r1 + r2 > 1.0:
		r1 = 1.0 - r1
		r2 = 1.0 - r2
	return v1 + r1 * (v2 - v1) + r2 * (v3 - v1)


func _create_displaced_vertex(
	local_x: float,
	local_z: float,
	world_x: float,
	world_z: float,
	grid_x: int,
	grid_z: int,
	grid_size: int,
	noise: FastNoiseLite,
	noise_strength: float
) -> Vector3:
	var is_edge_vertex = false
	var cliff_normal = Vector3.ZERO
	var corner_config = parent_parcel.corner_config

	var on_empty_boundary = false
	if grid_z == 0 and corner_config.north == CornerConfiguration.ParcelState.EMPTY:
		on_empty_boundary = true
	if grid_z == grid_size and corner_config.south == CornerConfiguration.ParcelState.EMPTY:
		on_empty_boundary = true
	if grid_x == grid_size and corner_config.east == CornerConfiguration.ParcelState.EMPTY:
		on_empty_boundary = true
	if grid_x == 0 and corner_config.west == CornerConfiguration.ParcelState.EMPTY:
		on_empty_boundary = true

	if not on_empty_boundary:
		if grid_z == 0 and corner_config.north == CornerConfiguration.ParcelState.NOTHING:
			is_edge_vertex = true
			cliff_normal = Vector3(0, 0, -1)

		if grid_z == grid_size and corner_config.south == CornerConfiguration.ParcelState.NOTHING:
			is_edge_vertex = true
			cliff_normal = Vector3(0, 0, 1)

		if grid_x == grid_size and corner_config.east == CornerConfiguration.ParcelState.NOTHING:
			is_edge_vertex = true
			cliff_normal = Vector3(1, 0, 0)

		if grid_x == 0 and corner_config.west == CornerConfiguration.ParcelState.NOTHING:
			is_edge_vertex = true
			cliff_normal = Vector3(-1, 0, 0)

	if not on_empty_boundary:
		if (
			grid_x == 0
			and grid_z == 0
			and corner_config.northwest == CornerConfiguration.ParcelState.NOTHING
		):
			cliff_normal = Vector3(-1, 0, -1).normalized()
		elif (
			grid_x == grid_size
			and grid_z == 0
			and corner_config.northeast == CornerConfiguration.ParcelState.NOTHING
		):
			cliff_normal = Vector3(1, 0, -1).normalized()
		elif (
			grid_x == 0
			and grid_z == grid_size
			and corner_config.southwest == CornerConfiguration.ParcelState.NOTHING
		):
			cliff_normal = Vector3(-1, 0, 1).normalized()
		elif (
			grid_x == grid_size
			and grid_z == grid_size
			and corner_config.southeast == CornerConfiguration.ParcelState.NOTHING
		):
			cliff_normal = Vector3(1, 0, 1).normalized()

	if is_edge_vertex and cliff_normal != Vector3.ZERO:
		var cliff_noise = FastNoiseLite.new()
		cliff_noise.seed = 54321
		cliff_noise.frequency = 0.3
		var cliff_noise_strength = 0.8

		var noise_value = cliff_noise.get_noise_2d(world_x, world_z)
		var cliff_displacement = noise_value * cliff_noise_strength
		var displaced_pos = Vector3(local_x, 0, local_z) - cliff_normal * cliff_displacement
		return displaced_pos

	var noise_value = noise.get_noise_2d(world_x, world_z)
	var base_displacement = (noise_value + 1.0) * 0.5 * noise_strength
	var falloff_multiplier = parent_parcel.calculate_displacement_falloff(grid_x, grid_z, grid_size)
	var displacement = base_displacement * falloff_multiplier

	return Vector3(local_x, displacement, local_z)


func _generate_loaded_parcel_edges(
	surface_tool: SurfaceTool,
	grid_size: int,
	cell_size: float,
	start_pos: float,
	noise: FastNoiseLite,
	noise_strength: float
):
	var corner_config = parent_parcel.corner_config
	var base_floor_height = -0.05
	var terrain_top_height = 0.0

	if corner_config.north == CornerConfiguration.ParcelState.LOADED:
		var z_pos = start_pos
		var x_start = start_pos
		var x_end = start_pos + grid_size * cell_size

		var base_bottom_left = Vector3(x_start, base_floor_height, z_pos)
		var base_bottom_right = Vector3(x_end, base_floor_height, z_pos)
		var terrain_top_right = Vector3(x_end, terrain_top_height, z_pos)
		var terrain_top_left = Vector3(x_start, terrain_top_height, z_pos)

		_add_quad_to_surface(
			surface_tool, base_bottom_left, base_bottom_right, terrain_top_right, terrain_top_left
		)

	if corner_config.south == CornerConfiguration.ParcelState.LOADED:
		var z_pos = start_pos + grid_size * cell_size
		var x_start = start_pos
		var x_end = start_pos + grid_size * cell_size

		var base_bottom_left = Vector3(x_start, base_floor_height, z_pos)
		var base_bottom_right = Vector3(x_end, base_floor_height, z_pos)
		var terrain_top_right = Vector3(x_end, terrain_top_height, z_pos)
		var terrain_top_left = Vector3(x_start, terrain_top_height, z_pos)

		_add_quad_to_surface(
			surface_tool, terrain_top_left, terrain_top_right, base_bottom_right, base_bottom_left
		)

	if corner_config.east == CornerConfiguration.ParcelState.LOADED:
		var x_pos = start_pos + grid_size * cell_size
		var z_start = start_pos
		var z_end = start_pos + grid_size * cell_size

		var base_bottom_near = Vector3(x_pos, base_floor_height, z_start)
		var base_bottom_far = Vector3(x_pos, base_floor_height, z_end)
		var terrain_top_far = Vector3(x_pos, terrain_top_height, z_end)
		var terrain_top_near = Vector3(x_pos, terrain_top_height, z_start)

		_add_quad_to_surface(
			surface_tool, base_bottom_near, base_bottom_far, terrain_top_far, terrain_top_near
		)

	if corner_config.west == CornerConfiguration.ParcelState.LOADED:
		var x_pos = start_pos
		var z_start = start_pos
		var z_end = start_pos + grid_size * cell_size

		var base_bottom_near = Vector3(x_pos, base_floor_height, z_start)
		var base_bottom_far = Vector3(x_pos, base_floor_height, z_end)
		var terrain_top_far = Vector3(x_pos, terrain_top_height, z_end)
		var terrain_top_near = Vector3(x_pos, terrain_top_height, z_start)

		_add_quad_to_surface(
			surface_tool, terrain_top_near, terrain_top_far, base_bottom_far, base_bottom_near
		)


func _add_quad_to_surface(
	surface_tool: SurfaceTool, v1: Vector3, v2: Vector3, v3: Vector3, v4: Vector3
):
	var normal1 = (v2 - v1).cross(v3 - v1).normalized()
	var normal2 = (v3 - v1).cross(v4 - v1).normalized()

	var uv1 = Vector2((v1.x + 8.0) / 16.0, (v1.z + 8.0) / 16.0)
	var uv2 = Vector2((v2.x + 8.0) / 16.0, (v2.z + 8.0) / 16.0)
	var uv3 = Vector2((v3.x + 8.0) / 16.0, (v3.z + 8.0) / 16.0)
	var uv4 = Vector2((v4.x + 8.0) / 16.0, (v4.z + 8.0) / 16.0)

	# First triangle (v1, v2, v3)
	surface_tool.set_normal(normal1)
	surface_tool.set_uv(uv1)
	surface_tool.add_vertex(v1)

	surface_tool.set_normal(normal1)
	surface_tool.set_uv(uv2)
	surface_tool.add_vertex(v2)

	surface_tool.set_normal(normal1)
	surface_tool.set_uv(uv3)
	surface_tool.add_vertex(v3)

	# Second triangle (v1, v3, v4)
	surface_tool.set_normal(normal2)
	surface_tool.set_uv(uv1)
	surface_tool.add_vertex(v1)

	surface_tool.set_normal(normal2)
	surface_tool.set_uv(uv3)
	surface_tool.add_vertex(v3)

	surface_tool.set_normal(normal2)
	surface_tool.set_uv(uv4)
	surface_tool.add_vertex(v4)


func _generate_floor_collision():
	var existing_body = get_node_or_null("CollisionBody")
	if existing_body:
		existing_body.queue_free()

	var static_body = StaticBody3D.new()
	static_body.name = "CollisionBody"
	static_body.collision_layer = 2
	add_child(static_body)

	var collision_shape = CollisionShape3D.new()
	collision_shape.name = "CollisionShape"
	static_body.add_child(collision_shape)

	var mesh_instance = get_node("GridFloor")
	if mesh_instance and mesh_instance.mesh:
		var shape = mesh_instance.mesh.create_trimesh_shape()
		collision_shape.shape = shape
