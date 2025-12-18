@tool
extends LineEdit

signal dcl_line_edit_changed
signal checked_error

@export var character_limit: int = 15
@export var allow_spaces: bool = true
@export var allow_edge_spaces: bool = false
@export var allow_special_characters: bool = false
@export var is_optional: bool = false

var error: bool = false
var error_message: String = ""


func is_alphanumeric_with_spaces(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"^[A-Za-z0-9 ]+$")
	return regex.search(value) != null


func has_leading_or_trailing_spaces(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"(^\s)|(\s$)")
	return regex.search(value) != null


func _append_error_message(msg: String) -> void:
	if error_message.length() > 0:
		error_message += "\n"
	error_message += msg


func _check_error():
	error_message = ""
	error = false

	if character_limit != 0 and text.length() > character_limit:
		error = true
		_append_error_message("⚠️ Characters limit reached")

	if not allow_spaces and text.contains(" "):
		error = true
		_append_error_message("⚠️ Spaces aren't allowed")

	if not allow_special_characters and not is_alphanumeric_with_spaces(text) and text.length() > 0:
		error = true
		_append_error_message("⚠️ Special characters aren't allowed")

	if not allow_edge_spaces and has_leading_or_trailing_spaces(text) and text.length() > 0:
		error = true
		_append_error_message("⚠️ Edge spaces aren't allowed")


func _ready() -> void:
	# Connect the text_changed signal to execute custom actions
	text_changed.connect(_on_text_changed)
	_check_error()


func _on_text_changed(_new_text: String) -> void:
	_check_error()
	dcl_line_edit_changed.emit()


func get_text_value() -> String:
	return text


func set_text_value(new_text: String) -> void:
	text = new_text
	_check_error()
	dcl_line_edit_changed.emit()
