extends RefCounted

# Generated mesh colliders are one-sided, like PhysX (#2853). A scene baked by an
# older build still carries `backface_collision = true` inside the saved
# PackedScene — local cache and published bucket alike — so the load path clears
# the flag instead of trusting the bake.

const GltfContainerScript = preload("res://src/decentraland_components/gltf_container.gd")

var suite_name := "gltf_container_colliders"
var method_name := "test_baked_backface_collision_is_cleared"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true
	var container = GltfContainerScript.new()

	var baked := _body_with_collider(true)
	container._make_collider_one_sided(baked)
	ok = _expect_flag(baked, false, "baked backface_collision is cleared") and ok
	ok = _expect_faces(baked, "the baked geometry survives") and ok

	var current := _body_with_collider(false)
	container._make_collider_one_sided(current)
	ok = _expect_flag(current, false, "an already one-sided collider stays one-sided") and ok

	var bare := StaticBody3D.new()
	container._make_collider_one_sided(bare)
	ok = _expect(bare.get_child_count() == 0, "a body without a collider is left alone") and ok

	baked.free()
	current.free()
	bare.free()
	container.free()

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _body_with_collider(backface: bool) -> StaticBody3D:
	var shape := ConcavePolygonShape3D.new()
	shape.set_faces(PackedVector3Array([Vector3.ZERO, Vector3.RIGHT, Vector3.UP]))
	shape.backface_collision = backface
	var collider := CollisionShape3D.new()
	collider.shape = shape
	var body := StaticBody3D.new()
	body.add_child(collider)
	return body


func _shape_of(body: StaticBody3D) -> ConcavePolygonShape3D:
	for child in body.get_children():
		if child is CollisionShape3D and child.shape is ConcavePolygonShape3D:
			return child.shape
	return null


func _expect_flag(body: StaticBody3D, expected: bool, label: String) -> bool:
	var shape := _shape_of(body)
	if shape == null:
		errors.append("%s: no concave shape in the body" % label)
		return false
	if shape.backface_collision != expected:
		errors.append(
			"%s: expected backface %s, got %s" % [label, expected, shape.backface_collision]
		)
		return false
	return true


func _expect_faces(body: StaticBody3D, label: String) -> bool:
	var shape := _shape_of(body)
	if shape == null or shape.get_faces().size() != 3:
		errors.append("%s: expected 3 face vertices, got %d" % [label, shape.get_faces().size()])
		return false
	return true


func _expect(condition: bool, label: String) -> bool:
	if not condition:
		errors.append(label)
		return false
	return true
