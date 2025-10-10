@tool
extends EditorScenePostImport


func _post_import(scene):
	print("Converting collision shapes to multiple convex shapes...")
	convert_to_multiple_convex_recursive(scene)
	return scene


func convert_to_multiple_convex_recursive(node: Node):
	if node is CollisionShape3D:
		convert_to_multiple_convex(node)

	for child in node.get_children():
		convert_to_multiple_convex_recursive(child)


func convert_to_multiple_convex(collision_shape: CollisionShape3D):
	var shape = collision_shape.shape

	if shape is ConcavePolygonShape3D:
		print("Converting ConcavePolygonShape3D to ConvexPolygonShape3D...")

		var concave_shape = shape as ConcavePolygonShape3D

		# Get the debug mesh and create a convex hull from it
		var debug_mesh = concave_shape.get_debug_mesh()
		if debug_mesh:
			var convex_shape = debug_mesh.create_convex_shape()

			if convex_shape:
				# Replace the concave shape with the convex hull
				collision_shape.shape = convex_shape
				print("Successfully created ConvexPolygonShape3D from debug mesh")
			else:
				print("Failed to create convex shape from debug mesh")


func find_mesh_instance(parent: Node) -> MeshInstance3D:
	for child in parent.get_children():
		if child is MeshInstance3D:
			return child

	if parent is MeshInstance3D:
		return parent

	return null
