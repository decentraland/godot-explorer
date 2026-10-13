extends RefCounted

# Regression test for #2704 (Scene Limits counts): the walker must
#  - count only physics-active colliders (parent body collision_layer != 0),
#  - skip `_collider` authoring meshes via the import-time name rule — NOT
#    live visibility, so SDK7 runtime visibility toggles don't move budgets.

var suite_name := "debug_collector"
var method_name := "test_scene_resource_counts"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

	var root := Node3D.new()

	# Ordinary prop mesh: counts as body + geometry + triangles.
	var prop := MeshInstance3D.new()
	prop.name = "wall"
	prop.mesh = _one_triangle_mesh()
	root.add_child(prop)

	# `_collider` authoring mesh: skipped by the name rule.
	var col_mesh := MeshInstance3D.new()
	col_mesh.name = "wall_collider"
	col_mesh.mesh = _one_triangle_mesh()
	root.add_child(col_mesh)

	# Dormant per-mesh trimesh body (layer 0): NOT counted.
	var dormant_body := StaticBody3D.new()
	dormant_body.collision_layer = 0
	dormant_body.add_child(CollisionShape3D.new())
	root.add_child(dormant_body)

	# Active body (SDK default mask 3): counted.
	var active_body := StaticBody3D.new()
	active_body.collision_layer = 3
	active_body.add_child(CollisionShape3D.new())
	root.add_child(active_body)

	# Shape under a non-physics parent: NOT counted.
	root.add_child(CollisionShape3D.new())

	var acc: Dictionary = {"triangles": 0, "bodies": 0, "colliders": 0}
	var geos: Dictionary = {}
	var mats: Dictionary = {}
	var texs: Dictionary = {}
	DebugCollector._walk_scene_resources(root, acc, geos, mats, texs, {})

	ok = _expect(acc["bodies"], 1, "only the non-collider mesh counts as body") and ok
	ok = _expect(acc["triangles"], 1, "collider mesh triangles skipped") and ok
	ok = _expect(geos.size(), 1, "collider mesh geometry skipped") and ok
	ok = _expect(acc["colliders"], 1, "only the layer!=0 shape counts as collider") and ok

	root.free()

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _one_triangle_mesh() -> ArrayMesh:
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = PackedVector3Array([Vector3.ZERO, Vector3.RIGHT, Vector3.UP])
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _expect(actual: Variant, expected: Variant, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true
