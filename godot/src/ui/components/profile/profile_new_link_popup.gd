extends ColorRect

signal add_link(title: String, url: String)

var title: String
var url: String

@onready var dcl_text_edit_link_url: VBoxContainer = %DclTextEdit_LinkUrl
@onready var dcl_text_edit_link_title: VBoxContainer = %DclTextEdit_LinkTitle
@onready var button_new_link_save: Button = %Button_NewLinkSave
@onready var keyboard_separator: HSeparator = %HSeparator_Keyboard


func _ready() -> void:
	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	dcl_text_edit_link_title.set_text_value()
	dcl_text_edit_link_url.set_text_value()
	hide()


func open() -> void:
	button_new_link_save.disabled = true
	show()


func _on_dcl_text_edit_link_title_dcl_text_edit_changed() -> void:
	title = dcl_text_edit_link_title.text_edit.text
	_check_error()


func _on_dcl_text_edit_link_url_dcl_text_edit_changed() -> void:
	url = dcl_text_edit_link_url.text_edit.text
	_check_error()


func _check_error() -> void:
	if dcl_text_edit_link_title.error or dcl_text_edit_link_url.error:
		button_new_link_save.disabled = true
	else:
		button_new_link_save.disabled = false


func _on_button_new_link_cancel_pressed() -> void:
	close()


func _on_button_new_link_save_pressed() -> void:
	emit_signal("add_link", title, url)
	close()


func _on_visibility_changed() -> void:
	if not visible and keyboard_separator != null:
		keyboard_separator.visible = false
		keyboard_separator.custom_minimum_size.y = 0


func _on_virtual_keyboard_changed(keyboard_height: int) -> void:
	if keyboard_height == 0:
		keyboard_separator.visible = false
		keyboard_separator.custom_minimum_size.y = 0
	else:
		var viewport_size := get_viewport().get_visible_rect().size
		var window_size := Vector2(DisplayServer.window_get_size())
		var y_factor: float = viewport_size.y / window_size.y
		keyboard_separator.custom_minimum_size.y = keyboard_height * y_factor
		keyboard_separator.visible = true
