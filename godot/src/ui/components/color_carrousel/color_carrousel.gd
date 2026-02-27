extends MarginContainer

signal color_picked(color: Color)
signal toggle_color_picker

enum ColorTargetType { SKIN, OTHER }

const COLOR_BUTTON = preload("res://src/ui/components/color_carrousel/color_button.tscn")

@export var color_type: ColorTargetType = ColorTargetType.SKIN:
	set(value):
		_dirty = true
		color_type = value

@onready var color_carrousel := %ColorCarrousel
@onready var color_slider := %ColorSlider
@onready var saturation_slider := %SaturationSlider
@onready var brightness_slider := %BrightnessSlider

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
var _dirty := false


func _ready() -> void:
	refresh_buttons()
	close_picker()


func _process(_delta: float) -> void:
	if not _dirty:
		return
	_dirty = false
	refresh_buttons()


func refresh_buttons() -> void:
	if color_type == ColorTargetType.OTHER:
		for child in color_carrousel.get_children():
			color_carrousel.remove_child(child)
			child.queue_free()
		
		#Add color swatch button
		var color_swatch := COLOR_BUTTON.instantiate()
		color_swatch.color = Color.BLACK
		color_swatch.button_group = color_button_group
		color_swatch.toggled.connect(self._on_color_carrousel_toggle_color_picker)
		color_swatch.is_color_palette = true
		color_carrousel.add_child(color_swatch)
			
		for color in example_colors:
			var color_square := COLOR_BUTTON.instantiate()
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


func _on_color_toggled(toggled: bool, color: Color):
	if toggled:
		color_slider.refresh_from_color(360.0 - (color.h * 360.0))
		saturation_slider.refresh_from_color(color.s * 100.0)
		brightness_slider.refresh_from_color(color.v * 100.0)
		saturation_slider.color = Color.from_hsv(color.h, 1.0, 1.0, 1.0)
		color_picked.emit(color)
	

func set_color(_color: Color) -> void:
	pass


func _on_color_slider_value_change() -> void:
	var h: float = 360.0 - (color_slider.value / 360.0)
	var s: float = saturation_slider.value / 100.0
	var v: float = brightness_slider.value / 100.0
	saturation_slider.color = Color.from_hsv(h, 1.0, 1.0, 1.0)
	color_picked.emit(Color.from_hsv(h, s, v, 1.0))


func _on_color_carrousel_toggle_color_picker(toggled: bool) -> void:
	if toggled:
		toggle_color_picker.emit(true)
		%ColorPickerTitle.show()
		%ColorPicker.show()


func close_picker() -> void:
	%ColorPicker.hide()
	%ColorPickerTitle.hide()
