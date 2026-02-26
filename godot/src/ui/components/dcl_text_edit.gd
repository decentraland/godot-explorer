@tool
class_name DclTextEdit
extends Control

signal dcl_text_edit_changed

const LINE_EDIT = preload("res://assets/themes/line_edit.tres")
const LINE_EDIT_FOCUSED = preload("res://assets/themes/line_edit_focused.tres")
const LINE_EDIT_ERROR = preload("res://assets/themes/line_edit_error.tres")



@export var place_holder: String = "Type text here..."
@export var has_max_length: bool = true
@export var max_length: int = 15
@export var is_optional: bool = true
@export var wrap_text: bool = true

@export_group("Validation")
@export var validate_url: bool = false
@export var validate_date: bool = false
@export var validate_no_symbols: bool = false
@export var validate_no_edge_spaces: bool = false

var length_error: bool = false
var error: bool = false

@onready var label_length: Label = %Label_Length
@onready var label_error: Label = %Label_Error
@onready var text_edit: TextEdit = %TextEdit
@onready var clear_button: TextureButton = %ClearButton


func _ready() -> void:
	text_edit.placeholder_text = place_holder
	if wrap_text:
		text_edit.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	else:
		text_edit.wrap_mode = TextEdit.LINE_WRAPPING_NONE
	if has_max_length:
		label_length.show()
		label_length.text = "0/" + str(max_length)
	else:
		label_length.hide()
	_update_clear_button()
	_check_error()


func _update_length() -> void:
	length_error = false
	if has_max_length:
		label_length.show()
		label_length.text = str(text_edit.text.length()) + "/" + str(max_length)
		label_length.show()
		if text_edit.text.length() > max_length:
			length_error = true


func _is_valid_url(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"^https?:\/\/[^\s]+\.[^\s]+$")
	return regex.search(value) != null


func _is_valid_date(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"^\d{2}/\d{2}/\d{4}$")
	return regex.search(value) != null


func _has_symbols(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"[^A-Za-z0-9 ]")
	return regex.search(value) != null


func _has_edge_spaces(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"(^\s)|(\s$)")
	return regex.search(value) != null


func _check_error() -> void:
	var errors: Array[String] = []
	var text := text_edit.text

	if !is_optional and text.length() <= 0:
		errors.append("This field is required")

	if length_error:
		errors.append("Character limit exceeded")

	if validate_url and text.length() > 0 and !_is_valid_url(text):
		errors.append("Enter a valid URL")

	if validate_date and text.length() > 0 and !_is_valid_date(text):
		errors.append("Use MM/DD/YYYY format")

	if validate_no_symbols and text.length() > 0 and _has_symbols(text):
		errors.append("Use letters and numbers only")

	if validate_no_edge_spaces and text.length() > 0 and _has_edge_spaces(text):
		errors.append("No leading or trailing spaces")

	error = errors.size() > 0

	if error:
		text_edit.add_theme_stylebox_override("normal", LINE_EDIT_ERROR)
		text_edit.add_theme_stylebox_override("focus", LINE_EDIT_ERROR)
		if errors.size() > 1:
			label_error.text = "Invalid format"
		else:
			label_error.text = errors[0]
		label_error.show()
	else:
		text_edit.add_theme_stylebox_override("normal", LINE_EDIT)
		text_edit.add_theme_stylebox_override("focus", LINE_EDIT_FOCUSED)
		label_error.hide()
	


func _update_clear_button() -> void:
	clear_button.visible = text_edit.text.length() > 0


func _on_clear_button_pressed() -> void:
	text_edit.text = ""
	_update_length()
	_update_clear_button()
	_check_error()
	emit_signal("dcl_text_edit_changed")


func _on_text_edit_text_changed() -> void:
	if has_max_length and text_edit.text.length() > max_length:
		text_edit.text = text_edit.text.left(max_length)
		text_edit.set_caret_column(text_edit.text.length())
	_update_length()
	_update_clear_button()
	_check_error()
	emit_signal("dcl_text_edit_changed")


func get_text_value() -> String:
	return text_edit.text


func set_text_value(new_text: String = "") -> void:
	text_edit.text = new_text
	_update_length()
	_update_clear_button()
	_check_error()
