class_name DclMeshRenderer
extends MeshInstance3D

enum MeshRendererPrimitiveType { NONE, MRPT_BOX, MRPT_CYLINDER, MRPT_SPHERE, MRPT_PLANE }

var current_type: MeshRendererPrimitiveType = MeshRendererPrimitiveType.NONE

####################
# Static methods
####################

static var initialized: bool = false
static var plane_array: Array = []
static var cube_array: Array = []


func _ready():
	DclMeshRenderer.init_primitive_shapes()


func set_box(uvs):
	if current_type != MeshRendererPrimitiveType.MRPT_BOX:
		current_type = MeshRendererPrimitiveType.MRPT_BOX
		self.mesh = ArrayMesh.new()
	else:
		self.mesh.clear_surfaces()

	var data_array = DclMeshRenderer.get_cube_arrays()
	var n = min(floor(uvs.size() / 2), 8)
	for i in range(n):
		data_array[4][i] = Vector2(uvs[i * 2], -uvs[i * 2 + 1])

	self.mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, data_array)


func set_plane(uvs: Array):
	if current_type != MeshRendererPrimitiveType.MRPT_PLANE:
		current_type = MeshRendererPrimitiveType.MRPT_PLANE
		self.mesh = ArrayMesh.new()
	else:
		self.mesh.clear_surfaces()

	var data_array = DclMeshRenderer.get_plane_arrays()
	var n = min(floor(float(uvs.size()) / 2.0), 8)
	for i in range(n):
		data_array[4][i] = Vector2(uvs[i * 2], -uvs[i * 2 + 1])

	self.mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, data_array)


func set_cylinder(top_radius: float, bottom_radius: float):
	if current_type != MeshRendererPrimitiveType.MRPT_CYLINDER:
		current_type = MeshRendererPrimitiveType.MRPT_CYLINDER
		self.mesh = ArrayMesh.new()
	else:
		self.mesh.clear_surfaces()

	var data_array = DclMeshRenderer.build_cylinder_arrays(top_radius, bottom_radius)
	self.mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, data_array)


func set_sphere():
	if current_type != MeshRendererPrimitiveType.MRPT_SPHERE:
		current_type = MeshRendererPrimitiveType.MRPT_SPHERE
		self.mesh = SphereMesh.new()


static func init_primitive_shapes():
	if initialized:
		return

	plane_array = build_plane_arrays()
	cube_array = build_cube_arrays()
	initialized = true


static func get_plane_arrays() -> Array:
	init_primitive_shapes()

	var ret = plane_array.duplicate()
	ret[4] = ret[4].duplicate()
	return ret


static func get_cube_arrays() -> Array:
	init_primitive_shapes()

	var ret = cube_array.duplicate()
	ret[4] = ret[4].duplicate()
	return ret


static func build_plane_arrays() -> Array:
	var h = 0.5
	var vertices = PackedVector3Array()
	vertices.append(Vector3(-h, -h, 0.0))
	vertices.append(Vector3(-h, h, 0.0))
	vertices.append(Vector3(h, h, 0.0))
	vertices.append(Vector3(h, -h, 0.0))

	vertices.append(Vector3(h, -h, 0.0))
	vertices.append(Vector3(h, h, 0.0))
	vertices.append(Vector3(-h, h, 0.0))
	vertices.append(Vector3(-h, -h, 0.0))

	var uvs = PackedVector2Array()
	# Match Unity's UV layout but with Y inverted for Godot's coordinate system
	uvs.append(Vector2(0, 1))
	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(1, 1))
	uvs.append(Vector2(1, 1))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(0, 1))

	var triangles = PackedInt32Array()
	triangles.append_array([0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4])

	var normals = PackedVector3Array()
	normals.append(Vector3(0, 0, 1))
	normals.append(Vector3(0, 0, 1))
	normals.append(Vector3(0, 0, 1))
	normals.append(Vector3(0, 0, 1))
	normals.append(Vector3(0, 0, -1))
	normals.append(Vector3(0, 0, -1))
	normals.append(Vector3(0, 0, -1))
	normals.append(Vector3(0, 0, -1))

	return [vertices, normals, null, null, uvs, null, null, null, null, null, null, null, triangles]


static func build_cube_arrays() -> Array:
	var uvs = PackedVector2Array()
	var uvs2 = PackedVector2Array()
	var vertices = PackedVector3Array()
	var normals = PackedVector3Array()
	var triangles = PackedInt32Array()
	var start: Vector3
	var d = 1.0

	start = Vector3(-d / 2, d / 2, -d / 2)
	vertices.append(start)
	vertices.append(start + Vector3.RIGHT * d)
	vertices.append(start + Vector3.RIGHT * d + Vector3.BACK * d)
	vertices.append(start + Vector3.BACK * d)

	start = Vector3(-d / 2, -d / 2, -d / 2)
	vertices.append(start)
	vertices.append(start + Vector3.RIGHT * d)
	vertices.append(start + Vector3.RIGHT * d + Vector3.BACK * d)
	vertices.append(start + Vector3.BACK * d)

	start = Vector3(-d / 2, d / 2, -d / 2)
	vertices.append(start)
	vertices.append(start + Vector3.BACK * d)
	vertices.append(start + Vector3.BACK * d + Vector3.DOWN * d)
	vertices.append(start + Vector3.DOWN * d)

	start = Vector3(d / 2, d / 2, -d / 2)
	vertices.append(start)
	vertices.append(start + Vector3.BACK * d)
	vertices.append(start + Vector3.BACK * d + Vector3.DOWN * d)
	vertices.append(start + Vector3.DOWN * d)

	start = Vector3(-d / 2, d / 2, -d / 2)
	vertices.append(start)
	vertices.append(start + Vector3.RIGHT * d)
	vertices.append(start + Vector3.RIGHT * d + Vector3.DOWN * d)
	vertices.append(start + Vector3.DOWN * d)

	start = Vector3(-d / 2, d / 2, d / 2)
	vertices.append(start)
	vertices.append(start + Vector3.RIGHT * d)
	vertices.append(start + Vector3.RIGHT * d + Vector3.DOWN * d)
	vertices.append(start + Vector3.DOWN * d)

	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(0, -1))

	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(0, -1))
	uvs.append(Vector2(0, 0))

	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(0, -1))

	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(0, -1))
	uvs.append(Vector2(0, 0))

	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(0, -1))

	uvs.append(Vector2(0, -1))
	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(0, 0))

	uvs2.append(Vector2(1, -1))
	uvs2.append(Vector2(1, 0))
	uvs2.append(Vector2(0, 0))
	uvs2.append(Vector2(0, -1))

	uvs2.append(Vector2(1, 0))
	uvs2.append(Vector2(1, -1))
	uvs2.append(Vector2(0, -1))
	uvs2.append(Vector2(0, 0))

	uvs2.append(Vector2(1, -1))
	uvs2.append(Vector2(1, 0))
	uvs2.append(Vector2(0, 0))
	uvs2.append(Vector2(0, -1))

	uvs2.append(Vector2(1, 0))
	uvs2.append(Vector2(1, -1))
	uvs2.append(Vector2(0, -1))
	uvs2.append(Vector2(0, 0))

	uvs2.append(Vector2(0, 0))
	uvs2.append(Vector2(1, 0))
	uvs2.append(Vector2(1, -1))
	uvs2.append(Vector2(0, -1))

	uvs2.append(Vector2(0, -1))
	uvs2.append(Vector2(1, -1))
	uvs2.append(Vector2(1, 0))
	uvs2.append(Vector2(0, 0))

	normals.append(Vector3.UP)
	normals.append(Vector3.UP)
	normals.append(Vector3.UP)
	normals.append(Vector3.UP)

	normals.append(Vector3.DOWN)
	normals.append(Vector3.DOWN)
	normals.append(Vector3.DOWN)
	normals.append(Vector3.DOWN)

	normals.append(Vector3.LEFT)
	normals.append(Vector3.LEFT)
	normals.append(Vector3.LEFT)
	normals.append(Vector3.LEFT)

	normals.append(Vector3.RIGHT)
	normals.append(Vector3.RIGHT)
	normals.append(Vector3.RIGHT)
	normals.append(Vector3.RIGHT)

	normals.append(Vector3.FORWARD)
	normals.append(Vector3.FORWARD)
	normals.append(Vector3.FORWARD)
	normals.append(Vector3.FORWARD)

	normals.append(Vector3.BACK)
	normals.append(Vector3.BACK)
	normals.append(Vector3.BACK)
	normals.append(Vector3.BACK)

	triangles.append(0)
	triangles.append(1)
	triangles.append(2)
	triangles.append(0)
	triangles.append(2)
	triangles.append(3)

	triangles.append(4 + 0)
	triangles.append(4 + 2)
	triangles.append(4 + 1)
	triangles.append(4 + 0)
	triangles.append(4 + 3)
	triangles.append(4 + 2)

	triangles.append(8 + 0)
	triangles.append(8 + 1)
	triangles.append(8 + 2)
	triangles.append(8 + 0)
	triangles.append(8 + 2)
	triangles.append(8 + 3)

	triangles.append(12 + 0)
	triangles.append(12 + 2)
	triangles.append(12 + 1)
	triangles.append(12 + 0)
	triangles.append(12 + 3)
	triangles.append(12 + 2)

	triangles.append(16 + 0)
	triangles.append(16 + 2)
	triangles.append(16 + 1)
	triangles.append(16 + 0)
	triangles.append(16 + 3)
	triangles.append(16 + 2)

	triangles.append(20 + 0)
	triangles.append(20 + 1)
	triangles.append(20 + 2)
	triangles.append(20 + 0)
	triangles.append(20 + 2)
	triangles.append(20 + 3)

	return [vertices, normals, null, null, uvs, null, null, null, null, null, null, null, triangles]


static func build_cylinder_arrays(radius_top: float, radius_bottom: float) -> Array:
	var uvs = PackedVector2Array()
	var vertices = PackedVector3Array()
	var normals = PackedVector3Array()
	var triangles = PackedInt32Array()
	var num_vertices = 50
	var length = 1.0
	var offset_pos = Vector3(0, -0.5, 0)
	var num_vertices_plus_one = num_vertices + 1

	vertices.resize(2 * num_vertices_plus_one + (num_vertices + 1) + (num_vertices + 1))
	normals.resize(2 * num_vertices_plus_one + (num_vertices + 1) + (num_vertices + 1))
	uvs.resize(2 * num_vertices_plus_one + (num_vertices + 1) + (num_vertices + 1))

	var slope: float = atan((radius_bottom - radius_top) / length)
	var slope_sin: float = -sin(slope)
	var slope_cos = cos(slope)

	for i in range(num_vertices):
		var angle: float = 2 * PI * i / num_vertices
		var angle_sin: float = -sin(angle)
		var angle_cos: float = cos(angle)
		var angle_half: float = 2 * PI * (i + 0.5) / num_vertices
		var angle_half_sin: float = -sin(angle_half)
		var angle_half_cos: float = cos(angle_half)

		vertices[i] = Vector3(radius_top * angle_cos, length, radius_top * angle_sin) + offset_pos
		vertices[i + num_vertices_plus_one] = (
			Vector3(radius_bottom * angle_cos, 0, radius_bottom * angle_sin) + offset_pos
		)

		if radius_top == 0:
			normals[i] = Vector3(angle_half_cos * slope_cos, -slope_sin, angle_half_sin * slope_cos)
		else:
			normals[i] = Vector3(angle_cos * slope_cos, -slope_sin, angle_sin * slope_cos)

		if radius_bottom == 0:
			normals[i + num_vertices_plus_one] = Vector3(
				angle_half_cos * slope_cos, -slope_sin, angle_half_sin * slope_cos
			)
		else:
			normals[i + num_vertices_plus_one] = Vector3(
				angle_cos * slope_cos, -slope_sin, angle_sin * slope_cos
			)

		uvs[i] = Vector2(1.0 - 1.0 * i / num_vertices, 1)
		uvs[i + num_vertices_plus_one] = Vector2(1.0 - 1.0 * i / num_vertices, 0)

	vertices[num_vertices] = vertices[0]
	vertices[num_vertices + num_vertices_plus_one] = vertices[0 + num_vertices_plus_one]
	uvs[num_vertices] = Vector2(1.0 - 1.0 * num_vertices / num_vertices, 1)
	uvs[num_vertices + num_vertices_plus_one] = Vector2(1.0 - 1.0 * num_vertices / num_vertices, 0)
	normals[num_vertices] = normals[0]
	normals[num_vertices + num_vertices_plus_one] = normals[0 + num_vertices_plus_one]

	var cover_top_index_start: int = 2 * num_vertices_plus_one
	var covert_top_index_end: int = 2 * num_vertices_plus_one + num_vertices
	for i in range(num_vertices):
		var angle: float = 2 * PI * i / num_vertices
		var angle_sin: float = -sin(angle)
		var angle_cos: float = cos(angle)

		vertices[cover_top_index_start + i] = (
			Vector3(radius_top * angle_cos, length, radius_top * angle_sin) + offset_pos
		)
		normals[cover_top_index_start + i] = Vector3(0, 1, 0)
		uvs[cover_top_index_start + i] = Vector2(angle_cos / 2 + 0.5, angle_sin / 2 + 0.5)

	vertices[cover_top_index_start + num_vertices] = Vector3(0, length, 0) + offset_pos
	normals[cover_top_index_start + num_vertices] = Vector3(0, 1, 0)
	uvs[cover_top_index_start + num_vertices] = Vector2(0.5, 0.5)

	var cover_bottom_index_start: int = cover_top_index_start + num_vertices + 1
	var cover_bottom_index_end: int = cover_bottom_index_start + num_vertices
	for i in range(num_vertices):
		var angle: float = 2 * PI * i / num_vertices
		var angle_sin: float = -sin(angle)
		var angle_cos: float = cos(angle)

		vertices[cover_bottom_index_start + i] = (
			Vector3(radius_bottom * angle_cos, 0, radius_bottom * angle_sin) + offset_pos
		)
		normals[cover_bottom_index_start + i] = Vector3(0, -1, 0)
		uvs[cover_bottom_index_start + i] = Vector2(angle_cos / 2 + 0.5, angle_sin / 2 + 0.5)

	vertices[cover_bottom_index_start + num_vertices] = Vector3(0, 0, 0) + offset_pos
	normals[cover_bottom_index_start + num_vertices] = Vector3(0, -1, 0)
	uvs[cover_bottom_index_start + num_vertices] = Vector2(0.5, 0.5)

	var cnt: int = 0
	if radius_top == 0:
		triangles.resize(num_vertices_plus_one * 3 + num_vertices * 3 + num_vertices * 3)
		for i in range(num_vertices):
			triangles[cnt] = i + num_vertices_plus_one
			cnt += 1
			triangles[cnt] = i
			cnt += 1
			triangles[cnt] = i + 1 + num_vertices_plus_one
			cnt += 1
	elif radius_bottom == 0:
		triangles.resize(num_vertices_plus_one * 3 + num_vertices * 3 + num_vertices * 3)
		for i in range(num_vertices):
			triangles[cnt] = i
			cnt += 1
			triangles[cnt] = i + 1
			cnt += 1
			triangles[cnt] = i + num_vertices_plus_one
			cnt += 1
	else:
		triangles.resize(num_vertices_plus_one * 6 + num_vertices * 3 + num_vertices * 3)
		for i in range(num_vertices):
			var ip1: int = i + 1
			triangles[cnt] = i
			cnt += 1
			triangles[cnt] = ip1
			cnt += 1
			triangles[cnt] = i + num_vertices_plus_one
			cnt += 1

			triangles[cnt] = ip1 + num_vertices_plus_one
			cnt += 1
			triangles[cnt] = i + num_vertices_plus_one
			cnt += 1
			triangles[cnt] = ip1
			cnt += 1

	for i in range(num_vertices):
		var next: int = cover_top_index_start + i + 1

		if next == covert_top_index_end:
			next = cover_top_index_start

		triangles[cnt] = next
		cnt += 1
		triangles[cnt] = cover_top_index_start + i
		cnt += 1
		triangles[cnt] = covert_top_index_end
		cnt += 1

	for i in range(num_vertices):
		var next: int = cover_bottom_index_start + i + 1
		if next == cover_bottom_index_end:
			next = cover_bottom_index_start

		triangles[cnt] = cover_bottom_index_end
		cnt += 1
		triangles[cnt] = cover_bottom_index_start + i
		cnt += 1
		triangles[cnt] = next
		cnt += 1

	return [vertices, normals, null, null, uvs, null, null, null, null, null, null, null, triangles]
