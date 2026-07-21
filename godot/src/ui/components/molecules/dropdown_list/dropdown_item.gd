@tool
class_name DropdownItem
extends Button

const CHECK_ICON = preload("res://src/ui/components/molecules/dropdown_list/icons/check.svg")

const COLOR_NORMAL = Color(0.9882353, 0.9882353, 0.9882353, 1)
const COLOR_ACTIVE = Color(0.9098039, 0.7254902, 1, 1)

@export var is_name: bool = false

var index: int = -1
var _name_text: String = ""

@onready var h_box_container_name: HBoxContainer = %HBoxContainer_Name
@onready var label_name: Label = %Label_Name


func setup(item_index: int, item_text: String, is_selected: bool) -> void:
	index = item_index
	_name_text = item_text

	if is_name:
		text = ""
	else:
		text = item_text

	if is_selected:
		icon = CHECK_ICON
		set_pressed_no_signal(true)
	else:
		icon = null
		set_pressed_no_signal(false)


func _ready() -> void:
	if Engine.is_editor_hint():
		return
	if is_name:
		_setup_name_label()
	else:
		h_box_container_name.hide()
	toggled.connect(_on_toggled)
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)


func _setup_name_label() -> void:
	label_name.add_theme_font_override("font", get_theme_font("font"))
	label_name.add_theme_font_size_override("font_size", get_theme_font_size("font_size"))
	label_name.text = _name_text
	var initial_color: Color = COLOR_ACTIVE if button_pressed else COLOR_NORMAL
	label_name.add_theme_color_override("font_color", initial_color)
	h_box_container_name.show()


func _on_toggled(pressed: bool) -> void:
	if not is_name:
		return
	label_name.add_theme_color_override("font_color", COLOR_ACTIVE if pressed else COLOR_NORMAL)


func _on_mouse_entered() -> void:
	if not is_name:
		return
	label_name.add_theme_color_override("font_color", COLOR_ACTIVE)


func _on_mouse_exited() -> void:
	if not is_name:
		return
	var color: Color = COLOR_ACTIVE if button_pressed else COLOR_NORMAL
	label_name.add_theme_color_override("font_color", color)
