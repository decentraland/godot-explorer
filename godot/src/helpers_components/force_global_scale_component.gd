extends Node3D

@export var target: Node3D


func _process(dt):
	# Obtain the global scale
	var current_global_scale = target.get_global_transform().basis.get_scale()

	# If the global scale is not 1,1,1, adjust the local scale
	if current_global_scale != Vector3(1, 1, 1):
		# If the node has a parent, adjust the local scale based on
		# the parent's global scale so that the resulting global scale
		# is 1,1,1.
		var parent = target.get_parent_node_3d()
		if parent:
			var parent_global_scale = parent.get_global_transform().basis.get_scale()
			# Here we use the inverse of the parent's global scale
			target.scale = target.scale / parent_global_scale
		else:
			target.scale = Vector3(1, 1, 1)
