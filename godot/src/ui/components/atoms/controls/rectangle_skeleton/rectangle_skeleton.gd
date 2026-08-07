class_name RectangleSkeleton
extends PanelContainer

@export var radius: int = 16:
	set(value):
		radius = value
		_apply_radius()


func _ready() -> void:
	_apply_radius()


func _apply_radius() -> void:
	var sb = get_theme_stylebox("panel")
	if sb is StyleBoxFlat:
		sb = sb.duplicate()
		sb.corner_radius_top_left = radius
		sb.corner_radius_top_right = radius
		sb.corner_radius_bottom_right = radius
		sb.corner_radius_bottom_left = radius
		add_theme_stylebox_override("panel", sb)
