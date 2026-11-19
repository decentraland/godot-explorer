extends Node

# Generated mesh colliders are one-sided, like PhysX (#2853). A scene baked by an
# older build still carries `backface_collision = true` inside the saved
# PackedScene — local cache and published bucket alike — so the load path clears
# the flag instead of trusting the bake.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/gltf/test_one_sided_colliders.tscn

const GltfContainerScript = preload("res://src/decentraland_components/gltf_container.gd")

var _failures := 0


func _ready() -> void:
	var container = GltfContainerScript.new()

	var baked := _body_with_collider(true)
	container._make_collider_one_sided(baked)
	_check_flag(baked, false, "a baked backface_collision is cleared")
	_check_faces(baked, "the baked geometry survives")

	var current := _body_with_collider(false)
	container._make_collider_one_sided(current)
	_check_flag(current, false, "an already one-sided collider stays one-sided")

	var bare := StaticBody3D.new()
	container._make_collider_one_sided(bare)
	_expect(bare.get_child_count() == 0, "a body without a collider is left alone")

	baked.free()
	current.free()
	bare.free()
	container.free()

	if _failures > 0:
		print("[test_one_sided_colliders] FAIL (%d)" % _failures)
		get_tree().quit(1)
		return
	print("[test_one_sided_colliders] PASS")
	get_tree().quit(0)


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
	for child in body.get_children(true):
		if child is CollisionShape3D and child.shape is ConcavePolygonShape3D:
			return child.shape
	return null


func _check_flag(body: StaticBody3D, expected: bool, label: String) -> void:
	var shape := _shape_of(body)
	if shape == null:
		_fail(label, "no concave shape in the body")
		return
	_expect(shape.backface_collision == expected, "%s (got %s)" % [label, shape.backface_collision])


func _check_faces(body: StaticBody3D, label: String) -> void:
	var shape := _shape_of(body)
	if shape == null:
		_fail(label, "no concave shape in the body")
		return
	_expect(
		shape.get_faces().size() == 3, "%s (got %d vertices)" % [label, shape.get_faces().size()]
	)


func _expect(condition: bool, label: String) -> void:
	if not condition:
		_fail(label, "assertion failed")


func _fail(label: String, detail: String) -> void:
	_failures += 1
	printerr("FAIL: %s — %s" % [label, detail])
