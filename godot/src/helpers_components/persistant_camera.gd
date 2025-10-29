extends Camera3D


func _on_tree_exiting() -> void:
	reparent(get_viewport())
