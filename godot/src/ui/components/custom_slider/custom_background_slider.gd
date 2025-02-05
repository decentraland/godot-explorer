extends Control

signal value_change

enum SliderType { COLOR, BRIGHTNESS, SATURATION }

@export var slider_type := SliderType.COLOR
@export var value: int = 10

var selected: bool = false
var grabber_size: Vector2
var slider_scale: float
var length: int = 200

var color_background_texture = preload("res://assets/ui/BackgroundHUE.png")
var saturation_background_texture = preload("res://assets/ui/BackgroundSaturation.png")
var brightness_background_texture = preload("res://assets/ui/BackgroundValue.png")

@onready var min_value: int = 0
@onready var max_value: int
@onready var step: int = 1

@onready var height: int = 35

@onready var control_grabber = $TextureRect_Background/Control_Grabber
@onready var texture_rect_background = $TextureRect_Background

@onready var label_title = $Label_Title


func _ready():
	update_sliders()


func update_sliders():
	match slider_type:
		SliderType.COLOR:
			texture_rect_background.texture = color_background_texture
			texture_rect_background.flip_h = true
			max_value = 360

		SliderType.BRIGHTNESS:
			texture_rect_background.texture = brightness_background_texture
			max_value = 100

		SliderType.SATURATION:
			texture_rect_background.texture = saturation_background_texture
			max_value = 100

	label_title.text = SliderType.keys()[slider_type]
	length = int(self.size.x)
	texture_rect_background.size = Vector2(float(length), height)

	if value > max_value:
		value = max_value
	if value < min_value:
		value = max_value

	slider_scale = float(length) / float(max_value)

	refresh_grabber_position(int(float(value) * slider_scale))


func _process(_delta):
	if selected:
		follow_mouse(get_local_mouse_position())


func follow_mouse(mouse_position: Vector2i):
	refresh_grabber_position(mouse_position.x)
	value_change.emit()


func refresh_grabber_position(new_x: int):
	var new_grabber_position: Vector2i = Vector2i(new_x, control_grabber.position.y)
	if new_grabber_position.x <= length and new_grabber_position.x >= 0:
		control_grabber.set_position(new_grabber_position)
		value = int(float(new_x) / slider_scale)
	elif new_grabber_position.x > length:
		control_grabber.set_position(Vector2i(length, control_grabber.position.y))
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
		int(float(color_value) * slider_scale), int(control_grabber.position.y)
	)
	if new_grabber_position.x <= length and new_grabber_position.x >= 0:
		control_grabber.set_position(new_grabber_position)
		value = color_value
	elif new_grabber_position.x > length:
		control_grabber.set_position(Vector2i(length, control_grabber.position.y))
		value = max_value
	else:
		control_grabber.set_position(Vector2i(0, control_grabber.position.y))
		value = min_value


func _on_resized():
	update_sliders()
