extends PopupPanel

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

enum ColorTargetType { SKIN, OTHER }

var color_type: ColorTargetType = ColorTargetType.SKIN

var color_button_group: ButtonGroup = ButtonGroup.new()

@onready var v_box_container_hair = $VBoxContainer/VBoxContainer_Hair
@onready var grid_container_hair = $VBoxContainer/VBoxContainer_Hair/GridContainer_Hair
@onready var grid_container_skin = $VBoxContainer/GridContainer_Skin

@onready var color_slider = $VBoxContainer/VBoxContainer_Hair/ColorSlider
@onready var saturation_slider = $VBoxContainer/VBoxContainer_Hair/SaturationSlider
@onready var brightness_slider = $VBoxContainer/VBoxContainer_Hair/BrightnessSlider
@onready var panel_preview = $VBoxContainer/VBoxContainer_Hair/Panel_Preview

var colorable_square = preload("res://src/ui/color_picker/colorable_square.tscn")

signal pick_color(color: Color)


func custom_popup(rect: Rect2, current_color: Color):
	v_box_container_hair.hide()
	grid_container_skin.hide()

	if color_type == ColorTargetType.OTHER:
		for child in grid_container_hair.get_children():
			grid_container_hair.remove_child(child)

		v_box_container_hair.show()
		for color in example_colors:
			var color_square = colorable_square.instantiate()
			color_square.background_color = color
			color_square.button_group = color_button_group
			color_square.toggled.connect(self._on_color_toggled.bind(color))
			grid_container_hair.add_child(color_square)

	if color_type == ColorTargetType.SKIN:
		for child in grid_container_skin.get_children():
			grid_container_skin.remove_child(child)

		grid_container_skin.show()
		for color in skin_colors:
			var color_square = colorable_square.instantiate()
			color_square.background_color = color
			color_square.button_group = color_button_group
			color_square.toggled.connect(self._on_color_toggled.bind(color))
			grid_container_skin.add_child(color_square)

	self.popup(rect)

	_on_color_toggled(true, current_color)


func _on_color_toggled(toggled: bool, color: Color):
	if toggled:
		var hsv_color = to_hsv(color)
		color_slider.refresh_from_color(hsv_color.x * 360.0)
		saturation_slider.refresh_from_color(hsv_color.y * 100.0)
		brightness_slider.refresh_from_color(hsv_color.z * 100.0)
		panel_preview.modulate = color
		pick_color.emit(color)

func _on_custom_background_slider_value_change():
	modulate_panel_preview()
	pick_color.emit(panel_preview.modulate)


func _on_custom_background_slider_2_value_change():
	modulate_panel_preview()
	pick_color.emit(panel_preview.modulate)


func _on_custom_background_slider_3_value_change():
	modulate_panel_preview()
	pick_color.emit(panel_preview.modulate)


func to_hsv(color: Color) -> Vector3:
	var max_component = max(color.r, color.g, color.b)
	var min_component = min(color.r, color.g, color.b)
	var delta_component = max_component - min_component
	var hue = 0.0
	var sat = 0.0

	if delta_component > 0.0:
		match max_component:
			0.0:
				pass
			color.r:
				hue = 60 * ((color.g - color.b) / delta_component)
			color.g:
				hue = 60 * (((color.b - color.r) / delta_component) + 2)
			color.b:
				hue = 60 * (((color.r - color.g) / delta_component) + 4)

	hue = hue / 360.0

	if max_component > 0.0:
		sat = delta_component / max_component

	return Vector3(hue, sat, max_component)


func modulate_panel_preview():
	var h: float = color_slider.value / 360.0
	var s: float = saturation_slider.value / 100.0
	var v: float = brightness_slider.value / 100.0
	panel_preview.modulate = Color.from_hsv(h, s, v, 1.0)
