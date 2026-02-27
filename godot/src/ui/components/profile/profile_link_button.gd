class_name ProfileLinkButton

extends Button

signal change_editing(editing: bool)
signal try_open_link(url: String)
signal delete_link

const EDITING_MARGIN_RIGHT = 60


var url: String = ""
var is_editing: bool= false

@onready var texture_rect_remove: TextureRect = %TextureRect_Remove


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
	is_editing = editing
	if editing:
		texture_rect_remove.show()
		_apply_editing_overrides()
	else:
		texture_rect_remove.hide()
		_remove_editing_overrides()



func _on_pressed() -> void:
	if is_editing:
		queue_free()
		emit_signal("delete_link")
	else:
		emit_signal("try_open_link", url)
