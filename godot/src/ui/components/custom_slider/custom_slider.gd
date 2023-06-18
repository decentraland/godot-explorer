extends Control

@export var min_value: int
@export var max_value: int
@export var step: int
@export var value: int

@export var lenght: int
@export var heigth: int

@onready var panel_grabber = $Panel_Grabber
@onready var panel_grabber_area_highlight = $Panel_GrabberAreaHighlight
@onready var panel_grabber_area = $Panel_GrabberArea
@onready var label_grabber_value = $Panel_Grabber/Label_GrabberValue

var selected: bool = false
var grabber_size: Vector2i
var slider_scale: float


# Called when the node enters the scene tree for the first time.
func _ready():
	if value > max_value:
		value = max_value
	if value < min_value:
		value = max_value

	panel_grabber_area.size = Vector2i(lenght, heigth)

	if max_value == 0:
		slider_scale = 0
	else:
		slider_scale = lenght / (max_value - min_value)

	grabber_size = Vector2i(heigth + 4, heigth + 4)
	label_grabber_value.add_theme_font_size_override("font_size", heigth / 2)
	panel_grabber.offset_left = grabber_size.x / 2
	panel_grabber_area.size = Vector2i(lenght, heigth)
	panel_grabber.size = grabber_size
	panel_grabber.set_position(Vector2i(panel_grabber.position.x, -2))

	refresh_grabber_position(value * slider_scale)
	refresh_highlight_area()


func _process(delta):
	if selected:
		follow_mouse(get_local_mouse_position())


func follow_mouse(position: Vector2i):
	refresh_grabber_position(position.x)
	refresh_highlight_area()


func refresh_highlight_area():
	panel_grabber_area_highlight.size = Vector2i(panel_grabber.position.x + heigth / 2, heigth)


func refresh_value(new_x: int):
	if slider_scale == 0:
		label_grabber_value.text = "Error"
	else:
		value = floor(new_x / slider_scale / step) * step
		label_grabber_value.text = str(value)


func refresh_grabber_position(new_x: int):
	var new_grabber_position: Vector2i = Vector2i(new_x, panel_grabber.position.y)
	if new_grabber_position.x <= lenght and new_grabber_position.x >= 0:
		panel_grabber.set_position(new_grabber_position)
		refresh_value(new_x)
	elif new_grabber_position.x > lenght:
		panel_grabber.set_position(Vector2i(lenght, panel_grabber.position.y))
		refresh_value(lenght)
	else:
		panel_grabber.set_position(Vector2i(0, panel_grabber.position.y))
		refresh_value(0)


func _on_color_rect_grabber_area_gui_input(event):
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			selected = true
		else:
			selected = false
