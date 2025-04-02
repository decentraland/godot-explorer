class_name SafeMarginContainer
extends MarginContainer

@export var default_margin: int = 8


func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	var safe_area: Rect2i = DisplayServer.get_display_safe_area()
	var window_size: Vector2i = DisplayServer.window_get_size()

	# BASE MARGINS
	var top: int = default_margin if Global.is_mobile() else 0
	var left: int = default_margin if Global.is_mobile() else 0
	var bottom: int = default_margin if Global.is_mobile() else 0
	var right: int = default_margin if Global.is_mobile() else 0

	if window_size.x >= safe_area.size.x and window_size.y >= safe_area.size.y:
		var x_factor: float = size.x / window_size.x
		var y_factor: float = size.y / window_size.y

		top = max(top, safe_area.position.y * y_factor)
		left = max(left, safe_area.position.x * x_factor)
		bottom = max(bottom, abs(safe_area.end.y - window_size.y) * y_factor)
		right = max(right, abs(safe_area.end.x - window_size.x) * x_factor)

	add_theme_constant_override("margin_top", top)
	add_theme_constant_override("margin_left", left)
	add_theme_constant_override("margin_right", right)
	add_theme_constant_override("margin_bottom", bottom)
