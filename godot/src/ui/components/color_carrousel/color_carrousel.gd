extends VBoxContainer

signal color_picked(color: Color)
signal color_set
signal toggle_color_picker(toggle: bool)

enum ColorTargetType { SKIN, HAIR, EYES }

const COLOR_BUTTON = preload("res://src/ui/components/color_carrousel/color_button.tscn")

@export var color_type: ColorTargetType = ColorTargetType.SKIN:
	set(value):
		_dirty = true
		color_type = value

var skin_colors: Array[Color] = [
	Color("ffe4c6"),
	Color("ffddbc"),
	Color("f2c2a5"),
	Color("ddb18f"),
	Color("cc9b77"),
	Color("9a765b"),
	Color("7d5d47"),
	Color("704b38"),
	Color("532a1c"),
	Color("3d2216")
]

var example_colors: Array[Color] = [
	Color.from_hsv(0.0 / 360.0, 0.0, 0.84),
	Color.from_hsv(0.0 / 360.0, 0.21, 0.11),
	Color.from_hsv(24.0 / 360.0, 0.45, 0.33),
	Color.from_hsv(16.0 / 360.0, 0.77, 0.57),
	Color.from_hsv(31.0 / 360.0, 0.94, 0.84),
	Color.from_hsv(42.0 / 360.0, 0.55, 0.93),
	Color.from_hsv(32.0 / 360.0, 0.100, 0.94),
	Color.from_hsv(47.0 / 360.0, 0.79, 0.99),
	Color.from_hsv(90.0 / 360.0, 0.80, 0.73),
	Color.from_hsv(213.0 / 360.0, 0.67, 0.97),
	Color.from_hsv(257.0 / 360.0, 0.52, 0.93),
	Color.from_hsv(333.0 / 360.0, 0.52, 0.91)
]

var color_button_group: ButtonGroup = ButtonGroup.new()
var selected_color: Color
var color_picker_button: Button

var _dirty := false

@onready var color_carrousel := %ColorCarrousel
@onready var color_slider := %ColorSlider
@onready var saturation_slider := %SaturationSlider
@onready var brightness_slider := %BrightnessSlider
@onready var scroll_swatch_container := %ScrollSwatchContainer
@onready var colo_swatch_title := %ColorSwatchTitle
@onready var color_picker := %ColorPicker


func _ready() -> void:
	colo_swatch_title.hide()
	refresh_buttons()
	close_picker()


func _process(_delta: float) -> void:
	if not _dirty:
		return
	_dirty = false
	refresh_buttons()


func refresh_buttons() -> void:
	if color_type != ColorTargetType.SKIN:
		for child in color_carrousel.get_children():
			color_carrousel.remove_child(child)
			child.queue_free()

		#Add color picker button
		color_picker_button = COLOR_BUTTON.instantiate()
		color_picker_button.toggle_mode = true
		color_picker_button.color = Color.BLACK
		color_picker_button.button_group = color_button_group
		color_picker_button.toggled.connect(self._on_color_carrousel_toggle_color_picker)
		color_picker_button.is_color_palette = true
		color_carrousel.add_child(color_picker_button)

		for color in example_colors:
			var color_square := COLOR_BUTTON.instantiate()
			color_square.toggle_mode = true
			color_square.color = color
			color_square.button_group = color_button_group
			color_square.toggled.connect(self._on_color_toggled.bind(color))
			color_carrousel.add_child(color_square)
	if color_type == ColorTargetType.SKIN:
		for child in color_carrousel.get_children():
			color_carrousel.remove_child(child)
			child.queue_free()
		for color in skin_colors:
			var color_square = COLOR_BUTTON.instantiate()
			color_square.color = color
			color_square.button_group = color_button_group
			color_square.toggled.connect(self._on_color_toggled.bind(color))
			color_carrousel.add_child(color_square)
	scroll_swatch_container.scroll_horizontal = 0.0


func _on_color_toggled(toggled: bool, color: Color):
	if toggled:
		color_slider.refresh_from_color(360.0 - (color.h * 360.0))
		saturation_slider.refresh_from_color(color.s * 100.0)
		brightness_slider.refresh_from_color(color.v * 100.0)
		saturation_slider.color = Color.from_hsv(color.h, 1.0, 1.0, 1.0)
		color_picked.emit(color)
		color_set.emit()


func set_color(color: Color) -> void:
	if _dirty:
		_dirty = false
		refresh_buttons()
	for color_button in color_carrousel.get_children():
		if color.is_equal_approx(color_button.color):
			scroll_swatch_container.ensure_control_visible.call_deferred(color_button)
			color_button.button_pressed = true
			return


func set_title(color_name: String) -> void:
	colo_swatch_title.text = color_name


func _on_color_slider_value_change() -> void:
	var h: float = 1.0 - (color_slider.value / 360.0)
	var s: float = saturation_slider.value / 100.0
	var v: float = brightness_slider.value / 100.0
	saturation_slider.color = Color.from_hsv(h, 1.0, 1.0, 1.0)
	color_picked.emit(Color.from_hsv(h, s, v, 1.0))


func _on_slider_released() -> void:
	color_picker_button.button_pressed = true
	color_set.emit()


func _on_color_carrousel_toggle_color_picker(toggled: bool) -> void:
	if toggled:
		toggle_color_picker.emit(true)
		colo_swatch_title.show()
		color_picker.show()


func close_picker() -> void:
	color_picker.hide()
	colo_swatch_title.hide()
	if color_picker_button:
		color_picker_button.button_pressed = false


func _on_color_picker_title_pressed() -> void:
	close_picker()
	toggle_color_picker.emit(false)
