extends Control
enum Slider_type { COLOR, BRIGHTNESS, SATURATION }
@export var slider_type := Slider_type.COLOR
@onready var min_value: int = 0
@onready var max_value: int
@onready var step: int = 1
@export var value: int = 10
@export var lenght: int = 200
@onready var heigth: int = 35

@onready var control_grabber = $TextureRect_Background/Control_Grabber
@onready var texture_rect_background = $TextureRect_Background

@onready var label_title = $Label_Title

var selected: bool = false
var grabber_size: Vector2
var slider_scale: float

var color_background_texture = preload("res://assets/ui/BackgroundHUE.png")
var saturation_background_texture = preload("res://assets/ui/BackgroundSaturation.png")
var brightness_background_texture = preload("res://assets/ui/BackgroundValue.png")

signal value_change


func _ready():
	match slider_type:
		Slider_type.COLOR:
			texture_rect_background.texture = color_background_texture
			texture_rect_background.flip_h = true
			max_value = 360

		Slider_type.BRIGHTNESS:
			texture_rect_background.texture = brightness_background_texture
			max_value = 100

		Slider_type.SATURATION:
			texture_rect_background.texture = saturation_background_texture
			max_value = 100

	label_title.text = Slider_type.keys()[slider_type]
	texture_rect_background.size = Vector2(lenght, heigth)

	if value > max_value:
		value = max_value
	if value < min_value:
		value = max_value

	slider_scale = float(lenght) / float(max_value)

	refresh_grabber_position(value * slider_scale)


func _process(delta):
	if selected:
		follow_mouse(get_local_mouse_position())


func follow_mouse(position: Vector2i):
	refresh_grabber_position(position.x)
	emit_signal("value_change")


func refresh_grabber_position(new_x: int):
	var new_grabber_position: Vector2i = Vector2i(new_x, control_grabber.position.y)
	if new_grabber_position.x <= lenght and new_grabber_position.x >= 0:
		control_grabber.set_position(new_grabber_position)
		value = new_x / slider_scale
	elif new_grabber_position.x > lenght:
		control_grabber.set_position(Vector2i(lenght, control_grabber.position.y))
		value = max_value
	else:
		control_grabber.set_position(Vector2i(0, control_grabber.position.y))
		value = min_value


func _on_texture_rect_background_gui_input(event):
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			selected = true
		else:
			selected = false


func refresh_from_color(color_value: int):
	var new_grabber_position: Vector2i = Vector2i(
		color_value * slider_scale, control_grabber.position.y
	)
	if new_grabber_position.x <= lenght and new_grabber_position.x >= 0:
		control_grabber.set_position(new_grabber_position)
		value = color_value
	elif new_grabber_position.x > lenght:
		control_grabber.set_position(Vector2i(lenght, control_grabber.position.y))
		value = max_value
	else:
		control_grabber.set_position(Vector2i(0, control_grabber.position.y))
		value = min_value
