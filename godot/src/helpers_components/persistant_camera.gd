extends Camera3D


func _on_tree_exiting() -> void:
	var viewport = get_viewport()
	if is_instance_valid(viewport) and get_parent() != viewport:
		reparent.call_deferred(viewport)
