class_name ProfileLinkButton

extends Button

signal change_editing(editing: bool)
signal try_open_link(url: String)
signal delete_link

const EDITING_MARGIN_RIGHT = 52

var url: String = ""

@onready var button_remove: Button = %Button_Remove


func _ready() -> void:
	add_to_group("profile_link_buttons")
	_on_change_editing(false)


func _apply_editing_overrides() -> void:
	for state in ["normal", "hover", "pressed", "focus"]:
		var base: StyleBox = get_theme_stylebox(state)
		var copy: StyleBox = base.duplicate()
		copy.content_margin_right = EDITING_MARGIN_RIGHT
		add_theme_stylebox_override(state, copy)


func _remove_editing_overrides() -> void:
	for state in ["normal", "hover", "pressed", "focus"]:
		remove_theme_stylebox_override(state)


func _on_change_editing(editing: bool) -> void:
	if editing:
		button_remove.show()
		_apply_editing_overrides()
	else:
		button_remove.hide()
		_remove_editing_overrides()


func _on_button_remove_pressed() -> void:
	queue_free()
	emit_signal("delete_link")


func _on_pressed() -> void:
	emit_signal("try_open_link", url)
