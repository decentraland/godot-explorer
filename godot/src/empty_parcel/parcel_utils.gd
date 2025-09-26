class_name ParcelUtils
extends RefCounted

static func create_aligned_transform(position: Vector3, normal: Vector3, random_rotation: bool = true, scale: float = 1.0) -> Transform3D:
	var transform = Transform3D()

	if normal != Vector3.UP:
		var rotation_axis = Vector3.UP.cross(normal).normalized()
		if rotation_axis.length() > 0.001:
			var angle = Vector3.UP.angle_to(normal)
			transform = transform.rotated(rotation_axis, angle)

	if random_rotation:
		var rotation_y = randf() * TAU
		transform = transform.rotated(normal, rotation_y)

	if scale != 1.0:
		transform = transform.scaled(Vector3.ONE * scale)

	transform.origin = position
	return transform

static func get_random_indices(array_size: int, count: int, condition_func: Callable = Callable()) -> Array[int]:
	var available_indices = []

	for i in range(array_size):
		if condition_func.is_valid():
			if condition_func.call(i):
				available_indices.append(i)
		else:
			available_indices.append(i)

	var selected_indices: Array[int] = []
	var actual_count = mini(count, available_indices.size())

	for i in range(actual_count):
		if available_indices.is_empty():
			break
		var random_idx = randi() % available_indices.size()
		selected_indices.append(available_indices[random_idx])
		available_indices.remove_at(random_idx)

	return selected_indices