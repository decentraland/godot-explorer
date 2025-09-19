class_name EmptyParcel
extends Node3D

enum EmptyParcelType {
	NONE,
	NORTH,
	EAST,
	SOUTH,
	WEST,
	NORTHEAST,
	NORTHWEST,
	SOUTHEAST,
	SOUTHWEST,
	INNER_NORTH,
	INNER_EAST,
	INNER_SOUTH,
	INNER_WEST,
	INNER_NORTHEAST,
	INNER_NORTHWEST,
	INNER_SOUTHEAST,
	INNER_SOUTHWEST
}

@export var parcel_type: EmptyParcelType = EmptyParcelType.NONE

# Cliff meshes are now dynamically generated
# const CliffScene = preload("res://assets/cliff/cliff.tscn")


func _ready():
	# Mesh will be created when parcel type is set
	pass


func set_parcel_type(type: EmptyParcelType) -> void:
	parcel_type = type
	print(
		(
			"Empty parcel at %s set parcel type: %s"
			% [global_position, EmptyParcelType.keys()[parcel_type]]
		)
	)
	# Regenerate mesh with the new parcel type
	_create_grid_mesh()
	# Generate cliff meshes based on parcel type
	_create_cliff_meshes()


func get_parcel_type() -> EmptyParcelType:
	return parcel_type


func _create_cliff_meshes() -> void:
	# Clear any existing cliff meshes
	for child in get_children():
		if child.name.begins_with("CliffMesh_"):
			child.queue_free()

	# Don't create cliffs if no parcel type
	if parcel_type == EmptyParcelType.NONE:
		return

	# Create cliff meshes based on parcel type
	match parcel_type:
		# Outer single edges - no corner adjustments needed
		EmptyParcelType.NORTH:
			_generate_cliff_mesh(
				"North", Vector3(0, 0, -8), Vector3(0, 0, -1), Vector3.ZERO, Vector3.ZERO
			)
		EmptyParcelType.SOUTH:
			_generate_cliff_mesh(
				"South", Vector3(0, 0, 8), Vector3(0, 0, 1), Vector3.ZERO, Vector3.ZERO
			)
		EmptyParcelType.EAST:
			_generate_cliff_mesh(
				"East", Vector3(8, 0, 0), Vector3(1, 0, 0), Vector3.ZERO, Vector3.ZERO
			)
		EmptyParcelType.WEST:
			_generate_cliff_mesh(
				"West", Vector3(-8, 0, 0), Vector3(-1, 0, 0), Vector3.ZERO, Vector3.ZERO
			)

		# Outer corners - pass corner normals for diagonal displacement
		# Horizontal cliffs (North/South): h=0 is west (-8), h=32 is east (+8)
		# Vertical cliffs (East/West): h=0 is south (-8), h=32 is north (+8)
		EmptyParcelType.NORTHEAST:
			var ne_corner_normal = Vector3(1, 0, -1).normalized()
			# North cliff: corner at east end (h=32)
			_generate_cliff_mesh(
				"North", Vector3(0, 0, -8), Vector3(0, 0, -1), Vector3.ZERO, ne_corner_normal
			)
			# East cliff: corner at north end (h=32) - but north is at z=-8, which is h=0!
			_generate_cliff_mesh(
				"East", Vector3(8, 0, 0), Vector3(1, 0, 0), ne_corner_normal, Vector3.ZERO
			)
		EmptyParcelType.NORTHWEST:
			var nw_corner_normal = Vector3(-1, 0, -1).normalized()
			# North cliff: corner at west end (h=0)
			_generate_cliff_mesh(
				"North", Vector3(0, 0, -8), Vector3(0, 0, -1), nw_corner_normal, Vector3.ZERO
			)
			# West cliff: corner at north end (h=32) - but north is at z=-8, which is h=0!
			_generate_cliff_mesh(
				"West", Vector3(-8, 0, 0), Vector3(-1, 0, 0), nw_corner_normal, Vector3.ZERO
			)
		EmptyParcelType.SOUTHEAST:
			var se_corner_normal = Vector3(1, 0, 1).normalized()
			# South cliff: corner at east end (h=32)
			_generate_cliff_mesh(
				"South", Vector3(0, 0, 8), Vector3(0, 0, 1), Vector3.ZERO, se_corner_normal
			)
			# East cliff: corner at south end (h=32) - south is at z=+8, which is h=32!
			_generate_cliff_mesh(
				"East", Vector3(8, 0, 0), Vector3(1, 0, 0), Vector3.ZERO, se_corner_normal
			)
		EmptyParcelType.SOUTHWEST:
			var sw_corner_normal = Vector3(-1, 0, 1).normalized()
			# South cliff: corner at west end (h=0)
			_generate_cliff_mesh(
				"South", Vector3(0, 0, 8), Vector3(0, 0, 1), sw_corner_normal, Vector3.ZERO
			)
			# West cliff: corner at south end (h=32) - south is at z=+8, which is h=32!
			_generate_cliff_mesh(
				"West", Vector3(-8, 0, 0), Vector3(-1, 0, 0), Vector3.ZERO, sw_corner_normal
			)


func _generate_cliff_mesh(
	cliff_name: String,
	edge_position: Vector3,
	outward_normal: Vector3,
	corner_normal_start: Vector3,
	corner_normal_end: Vector3
) -> void:
	# Create a new mesh instance for this cliff
	var cliff_mesh_instance = MeshInstance3D.new()
	cliff_mesh_instance.name = "CliffMesh_%s" % cliff_name
	add_child(cliff_mesh_instance)

	# Create surface tool for mesh generation
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	# Generate the cliff face
	var cliff_height = 10.0  # Height of the cliff
	var cliff_segments = 32  # Match floor grid vertex count (32 cells = 33 vertices)
	var vertical_segments = 8  # Number of vertical segments

	# Noise for displacement
	var noise = FastNoiseLite.new()
	noise.seed = 54321  # Different seed from floor
	noise.frequency = 0.3
	var noise_strength = 0.8  # Maximum displacement into the cliff

	# Determine cliff orientation and generate vertices
	var is_horizontal = abs(outward_normal.z) > 0.5  # North/South cliffs
	var cliff_length = 16.0  # Full parcel width

	# Generate cliff vertices
	for v in range(vertical_segments + 1):
		var v_ratio = float(v) / float(vertical_segments)
		var y_pos = -v_ratio * cliff_height  # Start at 0, go down

		# No vertical falloff - uniform displacement across the cliff face
		var vertical_falloff = 1.0

		for h in range(cliff_segments + 1):
			var h_ratio = float(h) / float(cliff_segments)
			var horizontal_pos = (h_ratio - 0.5) * cliff_length

			# Calculate vertex position based on cliff orientation
			var vertex_pos: Vector3
			var world_x: float
			var world_z: float

			if is_horizontal:  # North/South cliff
				vertex_pos = edge_position + Vector3(horizontal_pos, y_pos, 0)
				world_x = global_position.x + horizontal_pos
				world_z = global_position.z + edge_position.z
			else:  # East/West cliff
				vertex_pos = edge_position + Vector3(0, y_pos, horizontal_pos)
				world_x = global_position.x + edge_position.x
				world_z = global_position.z + horizontal_pos

			# Determine which normal to use for displacement
			var displacement_normal = outward_normal

			# Use corner normals at the ends if provided
			if h == 0 and corner_normal_start != Vector3.ZERO:
				displacement_normal = corner_normal_start
			elif h == cliff_segments and corner_normal_end != Vector3.ZERO:
				displacement_normal = corner_normal_end

			# Apply noise displacement in negative normal direction
			# Use consistent world coordinates for noise sampling
			var noise_value = noise.get_noise_2d(world_x, world_z)
			var displacement = noise_value * noise_strength * vertical_falloff
			vertex_pos -= displacement_normal * displacement

			# Add vertex with normal pointing outward
			surface_tool.set_normal(outward_normal)
			surface_tool.set_uv(Vector2(h_ratio, v_ratio))
			surface_tool.add_vertex(vertex_pos)

	# Generate triangles for the cliff face
	# North and East need normal winding, South and West need reversed
	var needs_reversed = outward_normal.z > 0.5 or outward_normal.x < -0.5  # South or West

	for v in range(vertical_segments):
		for h in range(cliff_segments):
			var idx = v * (cliff_segments + 1) + h
			var idx_next_row = (v + 1) * (cliff_segments + 1) + h

			if needs_reversed:
				# Reversed winding for South and West
				# First triangle
				surface_tool.add_index(idx)
				surface_tool.add_index(idx_next_row + 1)
				surface_tool.add_index(idx_next_row)

				# Second triangle
				surface_tool.add_index(idx)
				surface_tool.add_index(idx + 1)
				surface_tool.add_index(idx_next_row + 1)
			else:
				# Normal winding for North and East
				# First triangle
				surface_tool.add_index(idx)
				surface_tool.add_index(idx_next_row)
				surface_tool.add_index(idx_next_row + 1)

				# Second triangle
				surface_tool.add_index(idx)
				surface_tool.add_index(idx_next_row + 1)
				surface_tool.add_index(idx + 1)

	# Generate normals and commit
	surface_tool.generate_normals()
	var cliff_mesh = surface_tool.commit()
	cliff_mesh_instance.mesh = cliff_mesh

	# Apply a material (reuse the empty parcel material for now)
	var material = preload("res://assets/empty-scenes/empty_parcel_material.tres")
	if material:
		cliff_mesh_instance.material_override = material


# Old cliff spawning functions have been removed
# Dynamic cliff mesh generation is now used instead (see _create_cliff_meshes)


func _create_grid_mesh():
	# Create a MeshInstance3D node for the grid
	var mesh_instance = MeshInstance3D.new()
	mesh_instance.name = "GridFloor"
	add_child(mesh_instance)

	# Load the material
	var material = load("res://assets/empty-scenes/empty_parcel_material.tres")
	mesh_instance.material_override = material

	_generate_mesh(mesh_instance)


func _regenerate_mesh():
	# Find the existing mesh instance and regenerate it
	var mesh_instance = get_node_or_null("GridFloor")
	if mesh_instance:
		_generate_mesh(mesh_instance)


func _generate_mesh(mesh_instance: MeshInstance3D):
	# Create the grid mesh using SurfaceTool
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	# Generate the floor grid
	_generate_floor_grid(surface_tool)

	# Generate parcel_type walls if needed (hidden for now)
	# _generate_parcel_type_walls(surface_tool)

	# Generate normals and commit the mesh
	surface_tool.generate_normals()
	var generated_mesh = surface_tool.commit()
	mesh_instance.mesh = generated_mesh
	mesh_instance.visible = true  # Show mesh

	# Apply the empty parcel material with shader
	var material = preload("res://assets/empty-scenes/empty_parcel_material.tres")
	if material:
		mesh_instance.material_override = material
		print(
			"Applied material to parcel at ",
			global_position,
			" - parcel type: ",
			EmptyParcelType.keys()[parcel_type]
		)
	else:
		print("ERROR: Could not load empty_parcel_material.tres")

	# Generate collision for the floor only (no parcel_type collision)
	_generate_floor_collision()


func _generate_floor_grid(surface_tool: SurfaceTool):
	# Create a 16x16 grid with 0.5x0.5 cell size (32x32 cells total)
	var grid_size = 32  # 16 units / 0.5 = 32 cells
	var cell_size = 0.5
	var start_pos = -8.0  # Center the grid from -8 to +8

	# Noise parameters - smoother with lower frequency and bigger displacement
	var noise = FastNoiseLite.new()
	noise.seed = 12345
	noise.frequency = 0.05  # Much lower frequency for smoother noise
	var noise_strength = 3.0  # Bigger displacement for more dramatic terrain

	# Generate vertices and triangles for the grid
	for z in range(grid_size):
		for x in range(grid_size):
			var x_pos = start_pos + x * cell_size
			var z_pos = start_pos + z * cell_size

			# Create quad vertices (4 corners) with noise displacement
			# Use world position for consistent noise across parcels
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

			# Get debug color for this parcel type
			var debug_color = _get_parcel_type_debug_color()

			# UV coordinates for this cell
			var u1 = float(x) / float(grid_size)
			var v1_uv = float(z) / float(grid_size)
			var u2 = float(x + 1) / float(grid_size)
			var v2_uv = float(z + 1) / float(grid_size)

			var uv1 = Vector2(u1, v1_uv)
			var uv2 = Vector2(u2, v1_uv)
			var uv3 = Vector2(u2, v2_uv)
			var uv4 = Vector2(u1, v2_uv)

			# Normal pointing up (will be recalculated later)
			var normal = Vector3(0, 1, 0)

			# First triangle (v1, v2, v3)
			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv1)
			surface_tool.set_color(debug_color)
			surface_tool.add_vertex(v1)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv2)
			surface_tool.set_color(debug_color)
			surface_tool.add_vertex(v2)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv3)
			surface_tool.set_color(debug_color)
			surface_tool.add_vertex(v3)

			# Second triangle (v1, v3, v4)
			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv1)
			surface_tool.set_color(debug_color)
			surface_tool.add_vertex(v1)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv3)
			surface_tool.set_color(debug_color)
			surface_tool.add_vertex(v3)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv4)
			surface_tool.set_color(debug_color)
			surface_tool.add_vertex(v4)


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
	# Check if this is an edge vertex that should match cliff displacement
	var is_edge_vertex = false
	var cliff_normal = Vector3.ZERO

	# Determine if vertex is on an outer edge based on parcel type
	# Note: grid indices can be 0 to grid_size (inclusive) due to quad vertex generation
	if parcel_type in [EmptyParcelType.NORTH, EmptyParcelType.NORTHEAST, EmptyParcelType.NORTHWEST]:
		if grid_z == 0:  # North edge (z=-8 in local space)
			is_edge_vertex = true
			cliff_normal = Vector3(0, 0, -1)
	if parcel_type in [EmptyParcelType.SOUTH, EmptyParcelType.SOUTHEAST, EmptyParcelType.SOUTHWEST]:
		if grid_z == grid_size:  # South edge - vertices at grid_size due to z+1 in quad generation
			is_edge_vertex = true
			cliff_normal = Vector3(0, 0, 1)
	if parcel_type in [EmptyParcelType.EAST, EmptyParcelType.NORTHEAST, EmptyParcelType.SOUTHEAST]:
		if grid_x == grid_size:  # East edge - vertices at grid_size due to x+1 in quad generation
			is_edge_vertex = true
			cliff_normal = Vector3(1, 0, 0)
	if parcel_type in [EmptyParcelType.WEST, EmptyParcelType.NORTHWEST, EmptyParcelType.SOUTHWEST]:
		if grid_x == 0:  # West edge (x=-8 in local space)
			is_edge_vertex = true
			cliff_normal = Vector3(-1, 0, 0)

	# Handle corner vertices with diagonal normals
	if parcel_type == EmptyParcelType.NORTHEAST and grid_x == grid_size and grid_z == 0:
		cliff_normal = Vector3(1, 0, -1).normalized()
	elif parcel_type == EmptyParcelType.NORTHWEST and grid_x == 0 and grid_z == 0:
		cliff_normal = Vector3(-1, 0, -1).normalized()
	elif parcel_type == EmptyParcelType.SOUTHEAST and grid_x == grid_size and grid_z == grid_size:
		cliff_normal = Vector3(1, 0, 1).normalized()
	elif parcel_type == EmptyParcelType.SOUTHWEST and grid_x == 0 and grid_z == grid_size:
		cliff_normal = Vector3(-1, 0, 1).normalized()

	if is_edge_vertex and cliff_normal != Vector3.ZERO:
		# Use cliff-style displacement for edge vertices
		var cliff_noise = FastNoiseLite.new()
		cliff_noise.seed = 54321  # Same seed as cliffs
		cliff_noise.frequency = 0.3  # Same frequency as cliffs
		var cliff_noise_strength = 0.8  # Same strength as cliffs

		var noise_value = cliff_noise.get_noise_2d(world_x, world_z)
		var cliff_displacement = noise_value * cliff_noise_strength

		# Apply displacement in the negative cliff normal direction
		var displaced_pos = Vector3(local_x, 0, local_z) - cliff_normal * cliff_displacement
		return displaced_pos
	else:
		# Regular floor displacement
		var noise_value = noise.get_noise_2d(world_x, world_z)
		# Convert noise from [-1, 1] to [0, 1] for positive-only displacement
		var base_displacement = (noise_value + 1.0) * 0.5 * noise_strength

		# Calculate falloff based on parcel type and position within grid
		var falloff_multiplier = _calculate_displacement_falloff(grid_x, grid_z, grid_size)
		var displacement = base_displacement * falloff_multiplier

		return Vector3(local_x, displacement, local_z)


func _calculate_displacement_falloff(grid_x: int, grid_z: int, grid_size: int) -> float:
	# Normalize grid coordinates to [0, 1] range
	# Standard coordinate system (directions now correctly assigned):
	# grid_x: 0 = west, 1 = east
	# grid_z: 0 = south, 1 = north (in mesh local space)
	var norm_x = float(grid_x) / float(grid_size - 1)  # 0 = west, 1 = east
	var norm_z = float(grid_z) / float(grid_size - 1)  # 0 = south, 1 = north

	var falloff = 1.0

	match parcel_type:
		# Outer cliff types - zero displacement at outer border (the cliff edge)
		EmptyParcelType.NORTH:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0)  # Zero at north edge (grid_z=31, norm_z=1) - FLIPPED
		EmptyParcelType.SOUTH:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)  # Zero at south edge (grid_z=0, norm_z=0) - FLIPPED
		EmptyParcelType.EAST:
			falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)  # Zero at east edge (grid_x=31, norm_x=1)
		EmptyParcelType.WEST:
			falloff = clamp(norm_x * 2.0, 0.0, 1.0)  # Zero at west edge (grid_x=0, norm_x=0)

		# Outer corners - zero displacement at both adjacent borders
		EmptyParcelType.NORTHEAST:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0) * clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)  # Zero at north and east - NORTH FLIPPED
		EmptyParcelType.NORTHWEST:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0) * clamp(norm_x * 2.0, 0.0, 1.0)  # Zero at north and west - NORTH FLIPPED
		EmptyParcelType.SOUTHEAST:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0) * clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)  # Zero at south and east - SOUTH FLIPPED
		EmptyParcelType.SOUTHWEST:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0) * clamp(norm_x * 2.0, 0.0, 1.0)  # Zero at south and west - SOUTH FLIPPED

		# Inner types - zero displacement at inner border (side facing content)
		EmptyParcelType.INNER_NORTH:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)  # Zero at south edge (facing content) - FLIPPED
		EmptyParcelType.INNER_SOUTH:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0)  # Zero at north edge (facing content) - FLIPPED
		EmptyParcelType.INNER_EAST:
			falloff = clamp(norm_x * 2.0, 0.0, 1.0)  # Zero at west edge (facing content)
		EmptyParcelType.INNER_WEST:
			falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)  # Zero at east edge (facing content)

		# Inner corners - use maximum of the two adjacent inner edge falloffs
		EmptyParcelType.INNER_NORTHEAST:
			# Maximum of INNER_NORTH and INNER_EAST falloffs
			var north_falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)  # INNER_NORTH: zero at south
			var east_falloff = clamp(norm_x * 2.0, 0.0, 1.0)  # INNER_EAST: zero at west
			falloff = max(north_falloff, east_falloff)
		EmptyParcelType.INNER_NORTHWEST:
			# Maximum of INNER_NORTH and INNER_WEST falloffs
			var north_falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)  # INNER_NORTH: zero at south
			var west_falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)  # INNER_WEST: zero at east
			falloff = max(north_falloff, west_falloff)
		EmptyParcelType.INNER_SOUTHEAST:
			# Maximum of INNER_SOUTH and INNER_EAST falloffs
			var south_falloff = clamp(norm_z * 2.0, 0.0, 1.0)  # INNER_SOUTH: zero at north
			var east_falloff = clamp(norm_x * 2.0, 0.0, 1.0)  # INNER_EAST: zero at west
			falloff = max(south_falloff, east_falloff)
		EmptyParcelType.INNER_SOUTHWEST:
			# Maximum of INNER_SOUTH and INNER_WEST falloffs
			var south_falloff = clamp(norm_z * 2.0, 0.0, 1.0)  # INNER_SOUTH: zero at north
			var west_falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)  # INNER_WEST: zero at east
			falloff = max(south_falloff, west_falloff)

		# Default case (NONE) - no falloff
		_:
			falloff = 1.0

	return falloff


func _get_parcel_type_debug_color() -> Color:
	# Systematic color mapping:
	# Red = North (0.0 = South, 1.0 = North)
	# Green = East (0.0 = West, 1.0 = East)
	# Blue = Inner vs Outer (0.0 = Outer/Cliff, 1.0 = Inner)

	var red = 0.0  # North component
	var green = 0.0  # East component
	var blue = 0.0  # Inner component

	match parcel_type:
		# Outer cliff types (blue = 0.0)
		EmptyParcelType.SOUTH:
			red = 0.0
			green = 0.5
			blue = 0.0  # Black with slight green
		EmptyParcelType.SOUTHWEST:
			red = 0.0
			green = 0.0
			blue = 0.0  # Pure black (south + west)
		EmptyParcelType.WEST:
			red = 0.5
			green = 0.0
			blue = 0.0  # Dark red
		EmptyParcelType.NORTHWEST:
			red = 1.0
			green = 0.0
			blue = 0.0  # Pure red (north + west)
		EmptyParcelType.NORTH:
			red = 1.0
			green = 0.5
			blue = 0.0  # Red with green
		EmptyParcelType.NORTHEAST:
			red = 1.0
			green = 1.0
			blue = 0.0  # Yellow (north + east)
		EmptyParcelType.EAST:
			red = 0.5
			green = 1.0
			blue = 0.0  # Green with red
		EmptyParcelType.SOUTHEAST:
			red = 0.0
			green = 1.0
			blue = 0.0  # Pure green (south + east)

		# Inner types (blue = 1.0)
		EmptyParcelType.INNER_SOUTH:
			red = 0.0
			green = 0.5
			blue = 1.0  # Blue with slight green
		EmptyParcelType.INNER_SOUTHWEST:
			red = 0.0
			green = 0.0
			blue = 1.0  # Pure blue (south + west)
		EmptyParcelType.INNER_WEST:
			red = 0.5
			green = 0.0
			blue = 1.0  # Magenta
		EmptyParcelType.INNER_NORTHWEST:
			red = 1.0
			green = 0.0
			blue = 1.0  # Pure magenta (north + west)
		EmptyParcelType.INNER_NORTH:
			red = 1.0
			green = 0.5
			blue = 1.0  # Magenta with green
		EmptyParcelType.INNER_NORTHEAST:
			red = 1.0
			green = 1.0
			blue = 1.0  # White (north + east + inner)
		EmptyParcelType.INNER_EAST:
			red = 0.5
			green = 1.0
			blue = 1.0  # Cyan with red
		EmptyParcelType.INNER_SOUTHEAST:
			red = 0.0
			green = 1.0
			blue = 1.0  # Pure cyan (south + east + inner)

		# Default/NONE - Gray (middle values)
		EmptyParcelType.NONE:
			red = 0.5
			green = 0.5
			blue = 0.5  # Gray

		_:
			return Color.MAGENTA  # Fallback/error color

	return Color(red, green, blue, 1.0)


func _generate_floor_collision():
	# Remove existing collision body if it exists
	var existing_body = get_node_or_null("CollisionBody")
	if existing_body:
		existing_body.queue_free()

	# Create a StaticBody3D for collision
	var static_body = StaticBody3D.new()
	static_body.name = "CollisionBody"
	static_body.collision_layer = 2  # Layer 1 (bit 1 = 2^1 = 2)
	add_child(static_body)

	# Create collision shape from the floor mesh only
	var collision_shape = CollisionShape3D.new()
	collision_shape.name = "CollisionShape"
	static_body.add_child(collision_shape)

	# Generate floor-only mesh for collision
	var floor_surface_tool = SurfaceTool.new()
	floor_surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	_generate_floor_grid(floor_surface_tool)
	floor_surface_tool.generate_normals()
	var floor_mesh = floor_surface_tool.commit()

	# Generate trimesh collision shape from the floor mesh only
	var shape = floor_mesh.create_trimesh_shape()
	collision_shape.shape = shape


func _generate_parcel_type_walls(surface_tool: SurfaceTool):
	if parcel_type == EmptyParcelType.NONE:
		return

	var wall_height = 4.0  # Height of parcel_type walls
	var parcel_half_size = 8.0  # Half the parcel size (16/2)

	# Handle corner cases (generate 2 walls)
	if (
		parcel_type
		in [
			EmptyParcelType.NORTHEAST,
			EmptyParcelType.NORTHWEST,
			EmptyParcelType.SOUTHEAST,
			EmptyParcelType.SOUTHWEST
		]
	):
		_generate_corner_parcel_type_walls(surface_tool, wall_height, parcel_half_size)
	else:
		# Handle single direction (generate 1 wall)
		_generate_single_parcel_type_wall(surface_tool, wall_height, parcel_half_size)


func _generate_single_parcel_type_wall(
	surface_tool: SurfaceTool, wall_height: float, parcel_half_size: float
):
	match parcel_type:
		EmptyParcelType.NORTH:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(-parcel_half_size, 0, parcel_half_size),
				Vector3(parcel_half_size, 0, parcel_half_size),
				wall_height,
				Vector3(0, 0, 1)
			)
		EmptyParcelType.SOUTH:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(parcel_half_size, 0, -parcel_half_size),
				Vector3(-parcel_half_size, 0, -parcel_half_size),
				wall_height,
				Vector3(0, 0, -1)
			)
		EmptyParcelType.EAST:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(parcel_half_size, 0, parcel_half_size),
				Vector3(parcel_half_size, 0, -parcel_half_size),
				wall_height,
				Vector3(1, 0, 0)
			)
		EmptyParcelType.WEST:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(-parcel_half_size, 0, -parcel_half_size),
				Vector3(-parcel_half_size, 0, parcel_half_size),
				wall_height,
				Vector3(-1, 0, 0)
			)


func _generate_corner_parcel_type_walls(
	surface_tool: SurfaceTool, wall_height: float, parcel_half_size: float
):
	match parcel_type:
		EmptyParcelType.NORTHEAST:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(-parcel_half_size, 0, parcel_half_size),
				Vector3(parcel_half_size, 0, parcel_half_size),
				wall_height,
				Vector3(0, 0, 1)
			)  # North wall
			_create_parcel_type_wall(
				surface_tool,
				Vector3(parcel_half_size, 0, parcel_half_size),
				Vector3(parcel_half_size, 0, -parcel_half_size),
				wall_height,
				Vector3(1, 0, 0)
			)  # East wall
		EmptyParcelType.NORTHWEST:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(-parcel_half_size, 0, parcel_half_size),
				Vector3(parcel_half_size, 0, parcel_half_size),
				wall_height,
				Vector3(0, 0, 1)
			)  # North wall
			_create_parcel_type_wall(
				surface_tool,
				Vector3(-parcel_half_size, 0, -parcel_half_size),
				Vector3(-parcel_half_size, 0, parcel_half_size),
				wall_height,
				Vector3(-1, 0, 0)
			)  # West wall
		EmptyParcelType.SOUTHEAST:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(parcel_half_size, 0, -parcel_half_size),
				Vector3(-parcel_half_size, 0, -parcel_half_size),
				wall_height,
				Vector3(0, 0, -1)
			)  # South wall
			_create_parcel_type_wall(
				surface_tool,
				Vector3(parcel_half_size, 0, parcel_half_size),
				Vector3(parcel_half_size, 0, -parcel_half_size),
				wall_height,
				Vector3(1, 0, 0)
			)  # East wall
		EmptyParcelType.SOUTHWEST:
			_create_parcel_type_wall(
				surface_tool,
				Vector3(parcel_half_size, 0, -parcel_half_size),
				Vector3(-parcel_half_size, 0, -parcel_half_size),
				wall_height,
				Vector3(0, 0, -1)
			)  # South wall
			_create_parcel_type_wall(
				surface_tool,
				Vector3(-parcel_half_size, 0, -parcel_half_size),
				Vector3(-parcel_half_size, 0, parcel_half_size),
				wall_height,
				Vector3(-1, 0, 0)
			)  # West wall


func _create_parcel_type_wall(
	surface_tool: SurfaceTool, start_pos: Vector3, end_pos: Vector3, height: float, normal: Vector3
):
	# Create a vertical wall from start_pos to end_pos going downward from floor level
	var top_start = start_pos  # Floor level
	var top_end = end_pos  # Floor level
	var bottom_start = start_pos - Vector3(0, height, 0)  # Below floor
	var bottom_end = end_pos - Vector3(0, height, 0)  # Below floor

	# Calculate UV coordinates based on wall length
	var wall_length = start_pos.distance_to(end_pos)
	var uv_scale = wall_length / 4.0  # Scale UV to avoid stretching

	# Triangle 1 (top_start, top_end, bottom_start) - counter-clockwise for outward normal
	surface_tool.set_normal(normal)
	surface_tool.set_uv(Vector2(0, 1))
	surface_tool.add_vertex(top_start)

	surface_tool.set_normal(normal)
	surface_tool.set_uv(Vector2(uv_scale, 1))
	surface_tool.add_vertex(top_end)

	surface_tool.set_normal(normal)
	surface_tool.set_uv(Vector2(0, 0))
	surface_tool.add_vertex(bottom_start)

	# Triangle 2 (top_end, bottom_end, bottom_start) - counter-clockwise for outward normal
	surface_tool.set_normal(normal)
	surface_tool.set_uv(Vector2(uv_scale, 1))
	surface_tool.add_vertex(top_end)

	surface_tool.set_normal(normal)
	surface_tool.set_uv(Vector2(uv_scale, 0))
	surface_tool.add_vertex(bottom_end)

	surface_tool.set_normal(normal)
	surface_tool.set_uv(Vector2(0, 0))
	surface_tool.add_vertex(bottom_start)
