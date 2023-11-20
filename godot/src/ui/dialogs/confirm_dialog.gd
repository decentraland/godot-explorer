extends DclConfirmDialog


func _on_visibility_changed():
	if is_visible():
		Global.release_mouse()
