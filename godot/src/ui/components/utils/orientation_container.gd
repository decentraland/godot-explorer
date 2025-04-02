extends BoxContainer

@export var invert: bool = false


# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	self.vertical = window_size.x < window_size.y
	if invert:
		self.vertical = !self.vertical
