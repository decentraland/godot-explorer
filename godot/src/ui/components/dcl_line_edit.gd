@tool
extends VBoxContainer

signal dcl_line_edit_changed
signal checked_error

@export var character_limit: int = 15
@export var allow_spaces: bool = true
@export var allow_edge_spaces: bool = false
@export var allow_special_characters: bool = false
@export var is_optional: bool = false
@export var advice: String = "Any advice or nothing"
@export var hint: String = "Hint"
@export var error_color: Color = Color.RED
@export var show_tag: bool = false

var error: bool = false
var error_message: String = ""
var text_value: String = ""

@onready var line_edit: LineEdit = %LineEdit
@onready var label_length: Label = %Label_Length
@onready var label_advice: Label = %Label_Advice
@onready var label_error: RichTextLabel = %Label_Error
@onready var panel_container_error_border: PanelContainer = %PanelContainer_ErrorBorder
@onready var label_tag: Label = %Label_Tag


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

	if character_limit != 0 and line_edit.text.length() > character_limit:
		error = true
		_append_error_message("⚠️ Characters limit reached")

	if not allow_spaces and line_edit.text.contains(" "):
		error = true
		_append_error_message("⚠️ Spaces aren't allowed")

	if (
		not allow_special_characters
		and not is_alphanumeric_with_spaces(line_edit.text)
		and line_edit.text.length() > 0
	):
		error = true
		_append_error_message("⚠️ Special characters aren't allowed")

	if (
		not allow_edge_spaces
		and has_leading_or_trailing_spaces(line_edit.text)
		and line_edit.text.length() > 0
	):
		error = true
		_append_error_message("⚠️ Edge spaces aren't allowed")

	if line_edit.text.length() <= 0:
		error = true

	var color: Color = Color.WHITE
	label_length.text = (str(line_edit.text.length()) + "/" + str(character_limit))
	if line_edit.text.length() > character_limit:
		color = error_color
	else:
		color = Color.WHITE
	label_length.label_settings.font_color = color

	if error:
		if error_message.length() > 0:
			label_error.show()
			label_advice.hide()
			label_error.text = error_message
		else:
			label_error.hide()
			label_advice.show()
		panel_container_error_border.self_modulate = error_color
	else:
		label_error.hide()
		label_advice.show()
		panel_container_error_border.self_modulate = Color.TRANSPARENT


func _ready() -> void:
	if !line_edit.text_changed.is_connected(_on_line_edit_text_changed):
		line_edit.text_changed.connect(_on_line_edit_text_changed)
	label_advice.text = advice
	line_edit.placeholder_text = hint
	label_tag.visible = show_tag
	_check_error()


func get_text_value() -> String:
	return line_edit.text


func set_text_value(new_text: String) -> void:
	line_edit.text = new_text
	_check_error()
	dcl_line_edit_changed.emit()


func _on_line_edit_text_changed(_new_text: String) -> void:
	_check_error()
	dcl_line_edit_changed.emit()
