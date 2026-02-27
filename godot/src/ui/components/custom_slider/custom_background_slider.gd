extends Control

signal value_change

enum SliderType { COLOR, BRIGHTNESS, SATURATION }

@export var slider_type := SliderType.COLOR
@export var value: int = 10
@export var color: Color = Color.RED:
	set(value):
		color = value
		update_color()

var selected: bool = false
var grabber_size: Vector2
var slider_scale: float
var length: int = 200

var color_background_texture = preload(
	"res://src/ui/components/custom_slider/color_slider_texture.tres"
)
var brightness_background_texture = preload(
	"res://src/ui/components/custom_slider/brightness_slider_texture.tres"
)
var saturation_background_texture = preload(
	"res://src/ui/components/custom_slider/saturation_slider_texture.tres"
)

@onready var min_value: int = 0
@onready var max_value: int
@onready var step: int = 1

@onready var height: int = 35

#@onready var control_grabber = $TextureRect_Background/Control_Grabber
@onready var control_grabber = $Control/Control
@onready var texture_rect_background = %SliderTexturePanel

@onready var label_title = $Label_Title


func _ready():
	# Loads ColorSlider theme variation
	const TYPE_VARIATION := "ColorSlider"
	$Label_Title.add_theme_font_size_override(
		"font_size", get_theme_font_size("font_size", TYPE_VARIATION)
	)
	$Label_Title.add_theme_font_override("font", get_theme_font("font", TYPE_VARIATION))
	$Label_Title.add_theme_color_override(
		"font_color", get_theme_color("font_color", TYPE_VARIATION)
	)
	update_sliders()


func update_sliders():
	match slider_type:
		SliderType.COLOR:
			#texture_rect_background.texture = color_background_texture
			#texture_rect_background.flip_h = true
			max_value = 360

		SliderType.BRIGHTNESS:
			#texture_rect_background.texture = brightness_background_texture
			var new_stylebox := StyleBoxTexture.new()
			new_stylebox.texture = brightness_background_texture
			texture_rect_background.add_theme_stylebox_override("panel", new_stylebox)
			max_value = 100

		SliderType.SATURATION:
			#texture_rect_background.texture = saturation_background_texture
			var new_stylebox := StyleBoxTexture.new()
			new_stylebox.texture = saturation_background_texture
			texture_rect_background.add_theme_stylebox_override("panel", new_stylebox)
			new_stylebox.texture.get_gradient().set_color(1, color)
			max_value = 100

	label_title.text = SliderType.keys()[slider_type]
	length = int(self.size.x) - 64
	#texture_rect_background.size = Vector2(float(length), height)

	if value > max_value:
		value = max_value
	if value < min_value:
		value = max_value

	slider_scale = float(length) / float(max_value)

	refresh_grabber_position(int(float(value) * slider_scale))


func update_color() -> void:
	var style_box: StyleBoxTexture = texture_rect_background.get_theme_stylebox("panel")
	style_box.texture.get_gradient().set_color(1, color)


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
	if is_node_ready():
		update_sliders()
