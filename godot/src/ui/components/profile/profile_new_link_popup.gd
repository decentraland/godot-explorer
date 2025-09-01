extends ColorRect

signal add_link(title: String, url: String)

var title: String
var url: String

@onready var dcl_text_edit_link_url: VBoxContainer = %DclTextEdit_LinkUrl
@onready var dcl_text_edit_link_title: VBoxContainer = %DclTextEdit_LinkTitle
@onready var button_new_link_save: Button = %Button_NewLinkSave


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	dcl_text_edit_link_title.set_text()
	dcl_text_edit_link_url.set_text()
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
