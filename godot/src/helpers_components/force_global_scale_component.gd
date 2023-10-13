extends Node3D

@export var target: Node3D


func _process(dt):
	var global_scale = target.get_global_transform().basis.get_scale()
	# If the global scale is not 1,1,1, adjust it
	if global_scale != Vector3.ONE:
		target.scale = target.scale / global_scale
