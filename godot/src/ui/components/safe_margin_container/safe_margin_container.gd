class_name SafeMarginContainer
extends MarginContainer


func _ready() -> void:
	var safe_area: Rect2i = DisplayServer.get_display_safe_area()
	var window_size: Vector2i = DisplayServer.window_get_size()

	# BASE MARGINS
	var top: int = 8 if Global.is_mobile else 0
	var left: int = 8 if Global.is_mobile else 0
	var bottom: int = 8 if Global.is_mobile else 0
	var right: int = 8 if Global.is_mobile else 0

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

	var real_safe_area: Rect2i = Rect2i(top, left, size.x - right, size.y - bottom)
	Global.scene_runner.set_interactable_area(real_safe_area)
