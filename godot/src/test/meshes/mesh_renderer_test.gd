extends Node3D

@onready var mesh_instance_3d: MeshInstance3D = $MeshInstance3D


func _ready():
	_init_primitive_shapes()

	var mesh = ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, get_cube_arrays())
	mesh_instance_3d.mesh = mesh


static var PLANE_ARRAY: Array = []
static var CUBE_ARRAY: Array = []


static func _init_primitive_shapes():
	PLANE_ARRAY = build_plane_arrays()
	CUBE_ARRAY = build_cube_arrays()


static func get_plane_arrays() -> Array:
	var ret = PLANE_ARRAY.duplicate()
	ret[4] = ret[4].duplicate()
	return ret


static func get_cube_arrays() -> Array:
	var ret = CUBE_ARRAY.duplicate()
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
	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(0, -1))
	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(1, 0))
	uvs.append(Vector2(0, 0))
	uvs.append(Vector2(0, -1))
	uvs.append(Vector2(1, -1))
	uvs.append(Vector2(1, 0))

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
