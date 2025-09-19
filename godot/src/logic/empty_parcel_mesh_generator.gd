class_name EmptyParcelMeshGenerator
extends RefCounted


static func create_grid_mesh() -> ArrayMesh:
	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	# Create a 16x16 grid with 0.5x0.5 cell size
	var grid_size = 32  # 16 units / 0.5 = 32 cells
	var cell_size = 0.5
	var total_size = 16.0  # Total parcel size

	# Center the grid (from -8 to +8)
	var start_pos = -8.0

	# Generate vertices and triangles for the grid
	for z in range(grid_size):
		for x in range(grid_size):
			var x_pos = start_pos + x * cell_size
			var z_pos = start_pos + z * cell_size

			# Create quad vertices (4 corners)
			var v1 = Vector3(x_pos, 0, z_pos)
			var v2 = Vector3(x_pos + cell_size, 0, z_pos)
			var v3 = Vector3(x_pos + cell_size, 0, z_pos + cell_size)
			var v4 = Vector3(x_pos, 0, z_pos + cell_size)

			# UV coordinates for this cell
			var u1 = float(x) / float(grid_size)
			var v1_uv = float(z) / float(grid_size)
			var u2 = float(x + 1) / float(grid_size)
			var v2_uv = float(z + 1) / float(grid_size)

			var uv1 = Vector2(u1, v1_uv)
			var uv2 = Vector2(u2, v1_uv)
			var uv3 = Vector2(u2, v2_uv)
			var uv4 = Vector2(u1, v2_uv)

			# Normal pointing up
			var normal = Vector3(0, 1, 0)

			# First triangle (v1, v2, v3)
			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv1)
			surface_tool.add_vertex(v1)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv2)
			surface_tool.add_vertex(v2)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv3)
			surface_tool.add_vertex(v3)

			# Second triangle (v1, v3, v4)
			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv1)
			surface_tool.add_vertex(v1)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv3)
			surface_tool.add_vertex(v3)

			surface_tool.set_normal(normal)
			surface_tool.set_uv(uv4)
			surface_tool.add_vertex(v4)

	# Generate normals and commit the mesh
	surface_tool.generate_normals()
	return surface_tool.commit()


static func create_empty_parcel_node(material: Material = null) -> MeshInstance3D:
	var mesh_instance = MeshInstance3D.new()
	mesh_instance.mesh = create_grid_mesh()

	if material:
		mesh_instance.material_override = material

	return mesh_instance
