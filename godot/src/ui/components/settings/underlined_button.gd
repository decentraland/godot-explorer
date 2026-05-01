@tool
class_name UnderlinedButton
extends Button

## Text displayed with an underline.
@export var underlined_text: String = "":
	set(val):
		underlined_text = val
		if is_node_ready():
			_update_text()

## Color applied to both the font and the underline.
@export var color: Color = Color.WHITE:
	set(val):
		color = val
		if is_node_ready():
			_update_style()

## Font used for the label.
@export var font: Font:
	set(val):
		font = val
		if is_node_ready():
			_update_style()

## Font size used for the label.
@export var font_size: int = 16:
	set(val):
		font_size = val
		if is_node_ready():
			_update_style()

@onready var _label: Label = %Label
@onready var _underline: ColorRect = %Underline


func _ready():
	_update_text()
	_update_style()


func _update_text():
	if _label:
		_label.text = underlined_text


func _update_style():
	if _label:
		_label.add_theme_color_override("font_color", color)
		_label.add_theme_font_size_override("font_size", font_size)
		if font:
			_label.add_theme_font_override("font", font)
	if _underline:
		_underline.color = color
