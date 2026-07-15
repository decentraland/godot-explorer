extends Control

# Owns the joystick capture-area lifecycle: STOP normally, IGNORE while a
# Godot-side menu is open so touches fall through to whatever's underneath.


func _ready() -> void:
	mouse_filter = MOUSE_FILTER_STOP
	Global.on_menu_open.connect(func(): mouse_filter = MOUSE_FILTER_IGNORE)
	Global.on_menu_close.connect(func(): mouse_filter = MOUSE_FILTER_STOP)
