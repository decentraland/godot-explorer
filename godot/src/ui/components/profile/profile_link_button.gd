class_name ProfileLinkButton

extends Button

signal change_editing(editing:bool)
signal try_open_link(url:String)

var url: String = ""
var is_editing: bool = false

@onready var button_remove: Button = %Button_Remove

func _ready() -> void:
	add_to_group("profile_link_buttons")
	_on_change_editing(false)
		

func _on_change_editing(editing: bool) -> void:
	is_editing = editing
	
	if is_editing:
		button_remove.show()
	else:
		button_remove.hide()


func _on_button_remove_pressed() -> void:
	queue_free()


func _on_pressed() -> void:
	emit_signal("try_open_link", url)
	print(url)
