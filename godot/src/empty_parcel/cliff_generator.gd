class_name CliffGenerator
extends Node3D

const CLIFF_MATERIAL = preload("res://assets/empty-scenes/cliff_material.tres")
const GRASS_OVERHANG_MATERIAL = preload(
	"res://assets/empty-scenes/empty_parcel_grass_overhang_material.tres"
)

var parent_parcel: EmptyParcel

var floor_noise: FastNoiseLite
var cliff_noise: FastNoiseLite


func _ready():
	parent_parcel = get_parent()

	floor_noise = FastNoiseLite.new()
	floor_noise.seed = 12345
	floor_noise.frequency = 0.05

	cliff_noise = FastNoiseLite.new()
	cliff_noise.seed = 54321
	cliff_noise.frequency = 0.3


func generate_cliffs():
	for child in get_children():
		if child.name.begins_with("CliffMesh_") or child.name.begins_with("CliffRock_"):
			child.queue_free()

	var corner_config = parent_parcel.corner_config
	if not corner_config.has_any_out_of_bounds_neighbor():
		return

	# Define cliff configurations
	var cliff_edges = [
		{
			"name": "North",
			"state": corner_config.north,
			"pos": Vector3(0, 0, -8),
			"normal": Vector3(0, 0, -1)
		},
		{
			"name": "South",
			"state": corner_config.south,
			"pos": Vector3(0, 0, 8),
			"normal": Vector3(0, 0, 1)
		},
		{
			"name": "East",
			"state": corner_config.east,
			"pos": Vector3(8, 0, 0),
			"normal": Vector3(1, 0, 0)
		},
		{
			"name": "West",
			"state": corner_config.west,
			"pos": Vector3(-8, 0, 0),
			"normal": Vector3(-1, 0, 0)
		}
	]

	# Generate cliff components for each out-of-bounds edge
	for edge in cliff_edges:
		if edge.state == CornerConfiguration.ParcelState.NOTHING:
			_generate_cliff_mesh(edge.name, edge.pos, edge.normal, corner_config)
			_generate_grass_overhang(edge.name, edge.pos, edge.normal, corner_config)
			_generate_cliff_rocks(edge.name, edge.pos, edge.normal)


func _generate_cliff_mesh(
	cliff_name: String,
	edge_position: Vector3,
	outward_normal: Vector3,
	corner_config: CornerConfiguration
) -> void:
	var cliff_mesh_instance = MeshInstance3D.new()
	cliff_mesh_instance.name = "CliffMesh_%s" % cliff_name
	add_child(cliff_mesh_instance)

	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	var cliff_height = 30.0
	var cliff_segments = 32
	var vertical_segments = 20

	var noise = FastNoiseLite.new()
	noise.seed = 54321
	noise.frequency = 0.3
	var noise_strength = 0.8

	var is_horizontal = abs(outward_normal.z) > 0.5
	var cliff_length = 16.0

	for v in range(vertical_segments + 1):
		var v_ratio = float(v) / float(vertical_segments)
		var y_pos = -v_ratio * cliff_height
		var vertical_falloff = 1.0

		for h in range(cliff_segments + 1):
			var h_ratio = float(h) / float(cliff_segments)
			var horizontal_pos = (h_ratio - 0.5) * cliff_length

			var vertex_pos: Vector3
			var world_x: float
			var world_z: float
			var is_corner_edge = false
			var corner_normal = Vector3.ZERO

			if is_horizontal:
				vertex_pos = edge_position + Vector3(horizontal_pos, y_pos, 0)
				world_x = global_position.x + horizontal_pos
				world_z = global_position.z + edge_position.z

				# Check for corner edges
				if (
					abs(horizontal_pos - (-8.0)) < 0.01
					and corner_config.west == CornerConfiguration.ParcelState.NOTHING
				):
					is_corner_edge = true
					corner_normal = Vector3(-1, 0, outward_normal.z).normalized()
				elif (
					abs(horizontal_pos - 8.0) < 0.01
					and corner_config.east == CornerConfiguration.ParcelState.NOTHING
				):
					is_corner_edge = true
					corner_normal = Vector3(1, 0, outward_normal.z).normalized()
			else:
				vertex_pos = edge_position + Vector3(0, y_pos, horizontal_pos)
				world_x = global_position.x + edge_position.x
				world_z = global_position.z + horizontal_pos

				# Check for corner edges
				if (
					abs(horizontal_pos - (-8.0)) < 0.01
					and corner_config.north == CornerConfiguration.ParcelState.NOTHING
				):
					is_corner_edge = true
					corner_normal = Vector3(outward_normal.x, 0, -1).normalized()
				elif (
					abs(horizontal_pos - 8.0) < 0.01
					and corner_config.south == CornerConfiguration.ParcelState.NOTHING
				):
					is_corner_edge = true
					corner_normal = Vector3(outward_normal.x, 0, 1).normalized()

			# Check if this vertex is at a boundary with an EMPTY neighbor
			var epsilon = 0.01
			var boundary_checks = []

			if is_horizontal:
				boundary_checks = [
					{"pos": -8.0, "config": corner_config.west},
					{"pos": 8.0, "config": corner_config.east}
				]
			else:
				boundary_checks = [
					{"pos": -8.0, "config": corner_config.north},
					{"pos": 8.0, "config": corner_config.south}
				]

			var skip_displacement = false
			for check in boundary_checks:
				if (
					abs(horizontal_pos - check.pos) < epsilon
					and check.config == CornerConfiguration.ParcelState.EMPTY
				):
					skip_displacement = true
					break

			# For top vertices (y=0), calculate floor position
			if abs(y_pos) < 0.01:
				var grid_x: int
				var grid_z: int
				var local_x: float = horizontal_pos
				var local_z: float = 0

				if is_horizontal:
					grid_x = clamp(int(round((horizontal_pos + 8.0) * 2.0)), 0, 32)
					grid_z = 0 if abs(edge_position.z - (-8.0)) < 0.01 else 32
					local_z = edge_position.z
				else:
					grid_z = clamp(int(round((horizontal_pos + 8.0) * 2.0)), 0, 32)
					grid_x = 32 if abs(edge_position.x - 8.0) < 0.01 else 0
					local_x = edge_position.x
					local_z = horizontal_pos

				var floor_position = _calculate_floor_edge_position(
					grid_x, grid_z, local_x, local_z, world_x, world_z, corner_config
				)
				vertex_pos = floor_position
			# For corner edges, use corner displacement along entire height
			elif is_corner_edge and not skip_displacement:
				var noise_value = noise.get_noise_2d(world_x, world_z)
				var displacement = noise_value * noise_strength * vertical_falloff
				vertex_pos -= corner_normal * displacement
			# For non-top vertices, apply regular cliff displacement
			elif not skip_displacement:
				var displacement_normal = outward_normal
				var noise_value = noise.get_noise_2d(world_x, world_z)
				var displacement = noise_value * noise_strength * vertical_falloff
				vertex_pos -= displacement_normal * displacement

			surface_tool.set_normal(outward_normal)
			surface_tool.set_uv(Vector2(h_ratio, v_ratio))
			surface_tool.add_vertex(vertex_pos)

	var needs_reversed = outward_normal.z > 0.5 or outward_normal.x < -0.5

	for v in range(vertical_segments):
		for h in range(cliff_segments):
			var idx = v * (cliff_segments + 1) + h
			var idx_next_row = (v + 1) * (cliff_segments + 1) + h

			if needs_reversed:
				surface_tool.add_index(idx)
				surface_tool.add_index(idx_next_row + 1)
				surface_tool.add_index(idx_next_row)
				surface_tool.add_index(idx)
				surface_tool.add_index(idx + 1)
				surface_tool.add_index(idx_next_row + 1)
			else:
				surface_tool.add_index(idx)
				surface_tool.add_index(idx_next_row)
				surface_tool.add_index(idx_next_row + 1)
				surface_tool.add_index(idx)
				surface_tool.add_index(idx_next_row + 1)
				surface_tool.add_index(idx + 1)

	surface_tool.generate_normals()
	var cliff_mesh = surface_tool.commit()
	cliff_mesh_instance.mesh = cliff_mesh
	cliff_mesh_instance.material_override = CLIFF_MATERIAL


func _generate_grass_overhang(
	overhang_name: String,
	edge_position: Vector3,
	outward_normal: Vector3,
	corner_config: CornerConfiguration
) -> void:
	var overhang_mesh_instance = MeshInstance3D.new()
	overhang_mesh_instance.name = "GrassOverhang_%s" % overhang_name
	add_child(overhang_mesh_instance)

	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	var overhang_segments = 32
	var overhang_distance = 0.3
	var overhang_droop = 0.1

	var is_horizontal = abs(outward_normal.z) > 0.5

	var cliff_length = 16.0
	var cliff_start = -8.0

	for h in range(overhang_segments + 1):
		var h_ratio = float(h) / float(overhang_segments)
		var horizontal_pos = cliff_start + h_ratio * cliff_length

		var world_x: float
		var world_z: float
		var grid_x: int
		var grid_z: int
		var local_x: float
		var local_z: float

		# Determine if this is a corner vertex and calculate positions
		var is_corner = false
		var corner_x: float = 0.0
		var corner_z: float = 0.0

		# Corner detection configurations
		var corner_checks = []
		if is_horizontal:
			corner_checks = [
				{
					"pos": -8.0,
					"cx": -8.0,
					"cz": edge_position.z,
					"gx": 0,
					"gz": 0 if edge_position.z < 0 else 32
				},
				{
					"pos": 8.0,
					"cx": 8.0,
					"cz": edge_position.z,
					"gx": 32,
					"gz": 0 if edge_position.z < 0 else 32
				}
			]
		else:
			corner_checks = [
				{
					"pos": -8.0,
					"cx": edge_position.x,
					"cz": -8.0,
					"gx": 0 if edge_position.x < 0 else 32,
					"gz": 0
				},
				{
					"pos": 8.0,
					"cx": edge_position.x,
					"cz": 8.0,
					"gx": 0 if edge_position.x < 0 else 32,
					"gz": 32
				}
			]

		# Check for corner match
		for check in corner_checks:
			if abs(horizontal_pos - check.pos) < 0.01:
				is_corner = true
				corner_x = check.cx
				corner_z = check.cz
				grid_x = check.gx
				grid_z = check.gz
				break

		# Calculate positions for non-corner vertices
		if not is_corner:
			if is_horizontal:
				world_x = global_position.x + horizontal_pos
				world_z = global_position.z + edge_position.z
				grid_x = clamp(int(round((horizontal_pos + 8.0) * 2.0)), 0, 32)
				grid_z = 0 if abs(edge_position.z - (-8.0)) < 0.01 else 32
				local_x = horizontal_pos
				local_z = edge_position.z
			else:
				world_x = global_position.x + edge_position.x
				world_z = global_position.z + horizontal_pos
				grid_z = clamp(int(round((horizontal_pos + 8.0) * 2.0)), 0, 32)
				grid_x = 32 if abs(edge_position.x - 8.0) < 0.01 else 0
				local_x = edge_position.x
				local_z = horizontal_pos
		else:
			world_x = global_position.x + corner_x
			world_z = global_position.z + corner_z
			local_x = corner_x
			local_z = corner_z

		# Get the exact floor edge position
		var inner_pos = _calculate_floor_edge_position(
			grid_x, grid_z, local_x, local_z, world_x, world_z, corner_config
		)

		# Apply noise to the outer edge for a more natural look
		var outer_noise_value = cliff_noise.get_noise_2d(world_x * 1.5, world_z * 1.5)
		var outer_displacement = outer_noise_value * 0.3  # Less displacement for outer edge
		var varied_distance = overhang_distance + outer_displacement

		var outer_pos: Vector3

		# Calculate outer position based on corner or edge type
		var displacement_dir = Vector3.ZERO

		if is_corner:
			# Corner configurations with their diagonal directions and required edge states
			var corner_configs = [
				{
					"x": -8.0,
					"z": -8.0,
					"dir": Vector3(-1, 0, -1).normalized(),
					"edges": [corner_config.north, corner_config.west]
				},
				{
					"x": 8.0,
					"z": -8.0,
					"dir": Vector3(1, 0, -1).normalized(),
					"edges": [corner_config.north, corner_config.east]
				},
				{
					"x": -8.0,
					"z": 8.0,
					"dir": Vector3(-1, 0, 1).normalized(),
					"edges": [corner_config.south, corner_config.west]
				},
				{
					"x": 8.0,
					"z": 8.0,
					"dir": Vector3(1, 0, 1).normalized(),
					"edges": [corner_config.south, corner_config.east]
				}
			]

			# Find matching corner and check if both edges have cliffs
			for config in corner_configs:
				if abs(corner_x - config.x) < 0.01 and abs(corner_z - config.z) < 0.01:
					var both_cliffs = (
						config.edges[0] == CornerConfiguration.ParcelState.NOTHING
						and config.edges[1] == CornerConfiguration.ParcelState.NOTHING
					)
					displacement_dir = config.dir if both_cliffs else outward_normal
					break
		else:
			displacement_dir = outward_normal

		# Apply the displacement
		var horizontal_displacement = displacement_dir * varied_distance
		outer_pos = (
			inner_pos
			+ Vector3(horizontal_displacement.x, -overhang_droop, horizontal_displacement.z)
		)

		# Add some vertical variation to the droop as well
		var droop_variation = cliff_noise.get_noise_2d(world_x * 2.0, world_z * 2.0) * 0.1
		outer_pos.y -= abs(droop_variation)

		surface_tool.set_normal(Vector3(0, 1, 0))
		surface_tool.set_uv(Vector2(0.0, h_ratio))
		surface_tool.set_color(Color.WHITE)  # Floor-touching vertices are white
		surface_tool.add_vertex(inner_pos)

		surface_tool.set_normal(Vector3(0, 1, 0))
		surface_tool.set_uv(Vector2(1.0, h_ratio))
		surface_tool.set_color(Color.BLACK)  # Hanging vertices are black
		surface_tool.add_vertex(outer_pos)

	for h in range(overhang_segments):
		var idx = h * 2
		surface_tool.add_index(idx)
		surface_tool.add_index(idx + 2)
		surface_tool.add_index(idx + 3)
		surface_tool.add_index(idx)
		surface_tool.add_index(idx + 3)
		surface_tool.add_index(idx + 1)

	surface_tool.generate_normals()
	var overhang_mesh = surface_tool.commit()
	overhang_mesh_instance.mesh = overhang_mesh
	overhang_mesh_instance.material_override = GRASS_OVERHANG_MATERIAL


func _calculate_floor_edge_position(
	grid_x: int,
	grid_z: int,
	local_x: float,
	local_z: float,
	world_x: float,
	world_z: float,
	corner_config: CornerConfiguration
) -> Vector3:
	# Start with base local position
	var position = Vector3(local_x, 0, local_z)

	# Calculate height displacement
	var falloff = parent_parcel.calculate_displacement_falloff(grid_x, grid_z, 32)
	var terrain_height = 3.0
	var floor_noise_value = floor_noise.get_noise_2d(world_x, world_z)
	var base_displacement = (floor_noise_value + 1.0) * 0.5 * terrain_height
	position.y = base_displacement * falloff

	# Check if on boundary with EMPTY neighbor
	var on_empty_boundary = (
		(grid_z == 0 and corner_config.north == CornerConfiguration.ParcelState.EMPTY)
		or (grid_z == 32 and corner_config.south == CornerConfiguration.ParcelState.EMPTY)
		or (grid_x == 32 and corner_config.east == CornerConfiguration.ParcelState.EMPTY)
		or (grid_x == 0 and corner_config.west == CornerConfiguration.ParcelState.EMPTY)
	)

	# Apply cliff displacement if not on EMPTY boundary
	if not on_empty_boundary:
		var cliff_normal = Vector3.ZERO
		var has_cliff = false

		# Check corners first
		var corner_checks = [
			{
				"x": 0,
				"z": 0,
				"config": corner_config.northwest,
				"normal": Vector3(-1, 0, -1).normalized()
			},
			{
				"x": 32,
				"z": 0,
				"config": corner_config.northeast,
				"normal": Vector3(1, 0, -1).normalized()
			},
			{
				"x": 0,
				"z": 32,
				"config": corner_config.southwest,
				"normal": Vector3(-1, 0, 1).normalized()
			},
			{
				"x": 32,
				"z": 32,
				"config": corner_config.southeast,
				"normal": Vector3(1, 0, 1).normalized()
			}
		]

		for corner in corner_checks:
			if (
				grid_x == corner.x
				and grid_z == corner.z
				and corner.config == CornerConfiguration.ParcelState.NOTHING
			):
				cliff_normal = corner.normal
				has_cliff = true
				break

		# Check edges if not a corner
		if not has_cliff:
			var edge_checks = [
				{"check": grid_z == 0, "config": corner_config.north, "normal": Vector3(0, 0, -1)},
				{"check": grid_z == 32, "config": corner_config.south, "normal": Vector3(0, 0, 1)},
				{"check": grid_x == 32, "config": corner_config.east, "normal": Vector3(1, 0, 0)},
				{"check": grid_x == 0, "config": corner_config.west, "normal": Vector3(-1, 0, 0)}
			]

			for edge in edge_checks:
				if edge.check and edge.config == CornerConfiguration.ParcelState.NOTHING:
					cliff_normal = edge.normal
					has_cliff = true
					break

		# Apply cliff displacement
		if has_cliff:
			var cliff_noise_value = cliff_noise.get_noise_2d(world_x, world_z)
			var cliff_displacement = cliff_noise_value * 0.8
			position -= cliff_normal * cliff_displacement

	return position


func _generate_cliff_rocks(
	cliff_name: String, edge_position: Vector3, outward_normal: Vector3
) -> void:
	# Spawn between 0 and 3 rocks embedded in the cliff
	var rock_count = randi_range(0, 3)
	if rock_count == 0:
		return

	# Get rock meshes from the EmptyParcelProps resource
	var rocks_parent = EmptyParcelProps.get_node("%Rocks")
	var available_rocks = rocks_parent.get_child_count()
	if available_rocks == 0:
		return

	var cliff_height = 30.0
	var is_horizontal = abs(outward_normal.z) > 0.5

	for i in range(rock_count):
		# Random position along the cliff
		var h_position = randf_range(-7.0, 7.0)  # Leave some margin from edges
		var v_position = randf_range(-cliff_height * 0.8, -cliff_height * 0.2)  # Embed in upper portion

		# Calculate rock position
		var rock_pos: Vector3
		if is_horizontal:
			rock_pos = edge_position + Vector3(h_position, v_position, 0)
		else:
			rock_pos = edge_position + Vector3(0, v_position, h_position)

		# Apply some noise displacement to make it look more natural
		var world_x = global_position.x + rock_pos.x
		var world_z = global_position.z + rock_pos.z
		var noise_value = cliff_noise.get_noise_2d(world_x, world_z)
		var displacement = noise_value * 0.5
		rock_pos -= outward_normal * displacement

		# Choose a random rock mesh
		var rock_to_duplicate = rocks_parent.get_child(randi() % available_rocks)
		# Use DUPLICATE_USE_INSTANTIATION to share materials and reduce descriptor set allocations
		var chosen_rock = rock_to_duplicate.duplicate(DUPLICATE_USE_INSTANTIATION)
		chosen_rock.name = "CliffRock_%s_%d" % [cliff_name, i]
		add_child(chosen_rock)

		# Random scale and rotation
		var scale_variation = randf_range(0.8, 1.5)
		var random_rotation = randf() * TAU  # Random rotation around normal

		# Create transform with rock's up vector as cliff normal (rocks stick out perpendicular)
		var up_vector = -outward_normal  # Rock's Y axis points out from cliff
		var right_vector = Vector3.UP.cross(up_vector).normalized()
		if right_vector.length() < 0.01:
			right_vector = Vector3.RIGHT
		var forward_vector = right_vector.cross(up_vector).normalized()

		var basis = Basis(right_vector, up_vector, forward_vector)
		basis = basis.rotated(up_vector, random_rotation)  # Rotate around the rock's up axis
		basis = basis.scaled(Vector3.ONE * scale_variation)

		chosen_rock.transform = Transform3D(basis, rock_pos)
