class_name CliffGenerator
extends Node3D

const EMPTY_PARCEL_MATERIAL = preload("res://assets/empty-scenes/empty_parcel_material.tres")

var parent_parcel: EmptyParcel


func _ready():
	parent_parcel = get_parent()


func generate_cliffs():
	for child in get_children():
		if child.name.begins_with("CliffMesh_"):
			child.queue_free()

	var corner_config = parent_parcel.corner_config
	if not corner_config.has_any_out_of_bounds_neighbor():
		return

	# Generate cliff mesh only for edges that are out of bounds (NOTHING)
	if corner_config.north == CornerConfiguration.ParcelState.NOTHING:
		_generate_cliff_mesh("North", Vector3(0, 0, -8), Vector3(0, 0, -1))

	if corner_config.south == CornerConfiguration.ParcelState.NOTHING:
		_generate_cliff_mesh("South", Vector3(0, 0, 8), Vector3(0, 0, 1))

	if corner_config.east == CornerConfiguration.ParcelState.NOTHING:
		_generate_cliff_mesh("East", Vector3(8, 0, 0), Vector3(1, 0, 0))

	if corner_config.west == CornerConfiguration.ParcelState.NOTHING:
		_generate_cliff_mesh("West", Vector3(-8, 0, 0), Vector3(-1, 0, 0))


func _generate_cliff_mesh(
	cliff_name: String,
	edge_position: Vector3,
	outward_normal: Vector3
) -> void:
	var cliff_mesh_instance = MeshInstance3D.new()
	cliff_mesh_instance.name = "CliffMesh_%s" % cliff_name
	add_child(cliff_mesh_instance)

	var surface_tool = SurfaceTool.new()
	surface_tool.begin(Mesh.PRIMITIVE_TRIANGLES)

	var cliff_height = 100.0
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

			if is_horizontal:
				vertex_pos = edge_position + Vector3(horizontal_pos, y_pos, 0)
				world_x = global_position.x + horizontal_pos
				world_z = global_position.z + edge_position.z
			else:
				vertex_pos = edge_position + Vector3(0, y_pos, horizontal_pos)
				world_x = global_position.x + edge_position.x
				world_z = global_position.z + horizontal_pos

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
	cliff_mesh_instance.material_override = EMPTY_PARCEL_MATERIAL
