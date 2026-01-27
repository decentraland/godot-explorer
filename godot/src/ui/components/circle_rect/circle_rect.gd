@tool
extends Control
class_name CircleRect

@export var color: Color = Color(1, 1, 1)
@export var radius := 0.0 # 0 = automático
@export var border_width := 0.0
@export var border_color := Color.BLACK
@export_range(8, 128) var segments := 64

func _draw():
	var r = radius
	if r <= 0:
		r = min(size.x, size.y) * 0.5

	var center = size * 0.5

	draw_circle(center, r, color)

	if border_width > 0:
		draw_arc(
			center,
			r,
			0,
			TAU,
			segments,
			border_color,
			border_width
		)

func _notification(what):
	if what == NOTIFICATION_RESIZED:
		queue_redraw()
