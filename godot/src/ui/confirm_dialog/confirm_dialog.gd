extends DclConfirmDialog

func _on_visibility_changed():
	if is_visible():
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
