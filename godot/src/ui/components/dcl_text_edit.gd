@tool 
extends Control

signal dcl_text_edit_changed

@onready var label_length: Label = %Label_Length
@onready var text_edit: TextEdit = %TextEdit
@onready var panel_container: PanelContainer = $PanelContainer

@export var place_holder:String = "Type text here..."
@export var has_max_length:bool = true
@export var max_length:int = 15
@export var validate_url:bool = false
@export var is_optional:bool = true

var length_error:bool = false

var error:bool = false

func _ready() -> void:
	text_edit.placeholder_text = place_holder
	label_length.hide()
	_check_error()

func _update_length()->void:
	length_error = false
	if has_max_length:
		label_length.show()
		label_length.text = str(text_edit.text.length()) + "/" + str(max_length)
		label_length.show()
		if text_edit.text.length() > max_length:
			length_error = true


func is_valid_web_url(url: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"^https:\/\/[^\s]+\.[^\s]+$")
	return regex.search(url) != null
	
func _check_error() -> void:
	var original_stylebox := panel_container.get_theme_stylebox("panel")
	var stylebox := original_stylebox.duplicate()
	if stylebox is StyleBoxFlat:
		if (validate_url and !is_valid_web_url(text_edit.text)) or length_error or (!is_optional and text_edit.text.length() <= 0):
			stylebox.border_color = Color(0.8, 0, 0, 1.0)
			error = true
		else:
			stylebox.border_color = Color.TRANSPARENT
			error = false
		panel_container.add_theme_stylebox_override("panel", stylebox)


func _on_text_edit_text_changed() -> void:
	_update_length()
	_check_error()
	emit_signal("dcl_text_edit_changed")
	
func get_text_value() -> String:
	return text_edit.text

func set_text(new_text: String = "") -> void:
	text_edit.text = new_text
	_update_length()
