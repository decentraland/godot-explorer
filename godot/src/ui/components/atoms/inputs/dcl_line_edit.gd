@tool
class_name DclLineEdit
extends VBoxContainer

signal dcl_line_edit_changed
signal checked_error

const LINEEDIT_FOCUS = preload("uid://bv7k1bt4j7pgb")
const LINEEDIT_FOCUS_ERROR = preload("uid://bcoprda85lwd5")
const LINEEDIT_NORMAL = preload("uid://o0x3mbwnvobx")
const LINEEDIT_NORMAL_ERROR = preload("uid://bmwt0rbi3myn3")

## The warning sign stays out of the catalogue: it is presentation, identical in every
## locale, and gluing it into the key would produce a string matching no entry.
const _ERROR_PREFIX := "\u26a0\ufe0f "

@export var character_limit: int = 15
@export var allow_spaces: bool = true
@export var allow_edge_spaces: bool = false
@export var allow_special_characters: bool = false
@export var is_optional: bool = false
@export var hint: String = "Hint"
@export var error_color: Color = Color.RED
@export var show_tag: bool = false

var error: bool = false
var error_message: String = ""
var text_value: String = ""

@onready var line_edit: LineEdit = %LineEdit
@onready var label_length: Label = %Label_Length
@onready var label_error: RichTextLabel = %Label_Error
@onready var label_tag: Label = %Label_Tag


func is_alphanumeric_with_spaces(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"^[A-Za-z0-9 ]+$")
	return regex.search(value) != null


func has_leading_or_trailing_spaces(value: String) -> bool:
	var regex := RegEx.new()
	regex.compile(r"(^\s)|(\s$)")
	return regex.search(value) != null


func _append_error_message(key: TranslationKey) -> void:
	if error_message.length() > 0:
		error_message += "\n"
	# Label_Error is auto_translate_mode = 2, because the messages are joined here into one
	# multi-line string that matches no key. Resolving each piece with text() is therefore
	# this component's job, and _notification() re-runs it on a language change.
	error_message += _ERROR_PREFIX + key.text()


func _check_error():
	error_message = ""
	error = false

	if character_limit != 0 and line_edit.text.length() > character_limit:
		error = true
		_append_error_message(TranslationKey.new("INPUT_ERROR_CHARACTER_LIMIT"))

	if not allow_spaces and line_edit.text.contains(" "):
		error = true
		_append_error_message(TranslationKey.new("INPUT_ERROR_NO_SPACES"))

	if (
		not allow_special_characters
		and not is_alphanumeric_with_spaces(line_edit.text)
		and line_edit.text.length() > 0
	):
		error = true
		_append_error_message(TranslationKey.new("INPUT_ERROR_NO_SPECIAL_CHARACTERS"))

	if (
		not allow_edge_spaces
		and has_leading_or_trailing_spaces(line_edit.text)
		and line_edit.text.length() > 0
	):
		error = true
		_append_error_message(TranslationKey.new("INPUT_ERROR_NO_EDGE_SPACES"))

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
			label_error.text = error_message
		else:
			label_error.hide()

		line_edit.set("theme_override_styles/focus", LINEEDIT_FOCUS_ERROR)
		line_edit.set("theme_override_styles/normal", LINEEDIT_NORMAL_ERROR)
	else:
		line_edit.set("theme_override_styles/focus", LINEEDIT_FOCUS)
		line_edit.set("theme_override_styles/normal", LINEEDIT_NORMAL)
		label_error.hide()


func _notification(what: int) -> void:
	# Composed in GDScript, so unlike a scene `text` property it does not re-translate
	# itself. Rebuilding the message is enough; _check_error() reassigns the label.
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		_check_error()


func _ready() -> void:
	if !line_edit.text_changed.is_connected(_on_line_edit_text_changed):
		line_edit.text_changed.connect(_on_line_edit_text_changed)
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
