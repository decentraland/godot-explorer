class_name SafeMarginContainer
extends MarginContainer

@export var default_margin: int = 0
@export var use_left: bool = true
@export var use_right: bool = true
@export var use_top: bool = true
@export var use_bottom: bool = true

var last_margin_bottom: int = 0


func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	var safe_area: Rect2i = DisplayServer.get_display_safe_area()
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size = get_viewport().get_visible_rect().size

	# BASE MARGINS
	var top: int = default_margin
	var left: int = default_margin
	var bottom: int = default_margin
	var right: int = default_margin

	var x_factor: float = viewport_size.x / window_size.x
	var y_factor: float = viewport_size.y / window_size.y

	if Global.is_mobile() and !Global.is_virtual_mobile():
		top = max(top, safe_area.position.y * y_factor)
		left = max(left, safe_area.position.x * x_factor)
		bottom = max(bottom, abs(safe_area.end.y - window_size.y) * y_factor)
		right = max(right, abs(safe_area.end.x - window_size.x) * x_factor)

	last_margin_bottom = bottom

	if use_top:
		add_theme_constant_override("margin_top", top)
	if use_left:
		add_theme_constant_override("margin_left", left)
	if use_right:
		add_theme_constant_override("margin_right", right)
	if use_bottom:
		add_theme_constant_override("margin_bottom", bottom)
