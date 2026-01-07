extends Control

@export var clean_on_landscape: bool = false
@export var clean_on_portrait: bool = false


func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	var is_landscape: bool = window_size.x > window_size.y
	var is_portrait: bool = window_size.y > window_size.x

	if clean_on_landscape and is_landscape:
		if get_window().size_changed.is_connected(self._on_size_changed):
			get_window().size_changed.disconnect(self._on_size_changed)
		queue_free()
		return

	if clean_on_portrait and is_portrait:
		if get_window().size_changed.is_connected(self._on_size_changed):
			get_window().size_changed.disconnect(self._on_size_changed)
		queue_free()
		return
