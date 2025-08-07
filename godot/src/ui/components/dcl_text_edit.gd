@tool 
extends Control

signal dcl_text_edit_changed

@onready var label_length: Label = %Label_Length
@onready var text_edit: TextEdit = %TextEdit
@onready var panel_container: PanelContainer = $PanelContainer

@export var place_holder:String = "Type text here..."
@export var has_max_length:bool = true
@export var max_length:int = 15

var error:bool = false

func _ready() -> void:
	text_edit.placeholder_text = place_holder
	label_length.hide()
	_update_length()

func _update_length()->void:
	var original_stylebox := panel_container.get_theme_stylebox("panel")
	var stylebox := original_stylebox.duplicate()
	error = false
	if has_max_length:
		label_length.show()
		label_length.text = str(text_edit.text.length()) + "/" + str(max_length)
		label_length.show()
		if text_edit.text.length() > max_length:
			error = true
			if stylebox is StyleBoxFlat:
				stylebox.border_color = Color(0.8, 0, 0, 1.0)
		else:
			if stylebox is StyleBoxFlat:
				stylebox.border_color = Color.WHITE
	else:
		stylebox.border_color = Color.TRANSPARENT
	panel_container.add_theme_stylebox_override("panel", stylebox)
	
	
func _on_text_edit_text_changed() -> void:
	_update_length()
	emit_signal("dcl_text_edit_changed")
	
func get_text_value() -> String:
	return text_edit.text
