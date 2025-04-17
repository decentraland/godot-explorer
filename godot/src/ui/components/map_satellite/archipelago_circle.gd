extends Control

var radius := 50.0
var circle_color := Color(0, 1, 0.2, 0.3)

func _ready():
	set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	set_size(Vector2(radius * 2, radius * 2))
	queue_redraw()

func _draw():
	draw_circle(size / 2, radius, circle_color)

func set_circle(pos: Vector2, new_radius: float, color: Color = circle_color):
	print("Draw circle at: ", pos, " radius: ", new_radius)
	radius = new_radius
	circle_color = color
	position = pos - Vector2(radius, radius) # Centra el círculo
	set_size(Vector2(radius * 2, radius * 2))
	queue_redraw()
	
