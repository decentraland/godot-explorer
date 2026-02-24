extends HBoxContainer

signal pick_color(color: Color)

enum ColorTargetType { SKIN, OTHER }

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
var _dirty := false


func _ready() -> void:
	refresh_buttons()


func _process(_delta: float) -> void:
	if not _dirty:
		return
	_dirty = false
	refresh_buttons()


func refresh_buttons() -> void:
	if color_type == ColorTargetType.OTHER:
		for child in get_children():
			remove_child(child)
			child.queue_free()
		for color in example_colors:
			var color_square = COLOR_BUTTON.instantiate()
			color_square.color = color
			color_square.button_group = color_button_group
			color_square.toggled.connect(self._on_color_toggled.bind(color))
			add_child(color_square)
	if color_type == ColorTargetType.SKIN:
		for child in get_children():
			remove_child(child)
			child.queue_free()
		for color in skin_colors:
			var color_square = COLOR_BUTTON.instantiate()
			color_square.color = color
			color_square.button_group = color_button_group
			color_square.toggled.connect(self._on_color_toggled.bind(color))
			add_child(color_square)


func _on_color_toggled(toggled: bool, color: Color):
	if toggled:
		#var hsv_color = to_hsv(color)
		#color_slider.refresh_from_color(hsv_color.x * 360.0)
		#saturation_slider.refresh_from_color(hsv_color.y * 100.0)
		#brightness_slider.refresh_from_color(hsv_color.z * 100.0)
		#panel_preview.modulate = color
		pick_color.emit(color)


func set_color(_color: Color) -> void:
	pass
