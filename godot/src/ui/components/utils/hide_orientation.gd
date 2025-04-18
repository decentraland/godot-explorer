extends Control

@export var hide_on_portrait: bool = false


# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	if hide_on_portrait:
		visible = window_size.x > window_size.y
	else:
		visible = window_size.x < window_size.y
