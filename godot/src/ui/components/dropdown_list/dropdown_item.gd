@tool
class_name DropdownItem
extends Button

const CHECK_ICON = preload("res://src/ui/components/dropdown_list/icons/check.svg")
var index: int = -1


func setup(item_index: int, item_text: String, is_selected: bool) -> void:
	index = item_index
	text = item_text

	if is_selected:
		icon = CHECK_ICON
		set_pressed_no_signal(true)
	else:
		icon = null
		set_pressed_no_signal(false)
