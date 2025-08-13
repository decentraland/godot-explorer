class_name ProfileLinkButton

extends Button

signal change_editing(editing: bool)
signal try_open_link(url: String)
signal delete_link

var url: String = ""
var stylebox: StyleBoxFlat

@onready var button_remove: Button = %Button_Remove


func _ready() -> void:
	stylebox = self.get_theme_stylebox("normal")
	add_to_group("profile_link_buttons")
	_on_change_editing(false)


func _on_change_editing(editing: bool) -> void:
	if editing:
		button_remove.show()
		stylebox.content_margin_right = 12 + 5 + 35
	else:
		button_remove.hide()
		stylebox.content_margin_right = 12


func _on_button_remove_pressed() -> void:
	queue_free()
	emit_signal("delete_link")


func _on_pressed() -> void:
	emit_signal("try_open_link", url)
	print(url)
