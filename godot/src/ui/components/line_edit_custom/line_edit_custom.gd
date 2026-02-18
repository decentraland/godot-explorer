@tool
class_name LineEditCustom
extends VBoxContainer

## Emitted when the text changes (every keystroke).
signal text_changed(new_text: String)
## Emitted when the user submits the text (Enter / Done on virtual keyboard).
signal text_submitted(text: String)
## Emitted when the LineEdit gains focus.
signal custom_focus_entered()
## Emitted when the LineEdit loses focus.
signal custom_focus_exited()
## Emitted when the optional action button is pressed.
signal button_pressed()

## Title displayed above the field. Hidden when empty.
@export var title: String = "":
	set(value):
		title = value
		if is_node_ready():
			_update_title()

## Button label. When empty, the button is hidden.
@export var text_button: String = "":
	set(value):
		text_button = value
		if is_node_ready():
			_update_button()

## Description displayed below the field. Hidden when empty.
@export var description: String = "":
	set(value):
		description = value
		if is_node_ready():
			_update_description()

## Placeholder text shown when the field is empty.
@export var placeholder_text: String = "":
	set(value):
		placeholder_text = value
		if is_node_ready():
			_line_edit.placeholder_text = placeholder_text

## Current text value. Use this to set or get the content.
var _text_value: String = ""
@export var text: String = "":
	set(value):
		_text_value = value
		if is_node_ready() and _line_edit.text != value:
			_line_edit.text = value
	get:
		return _line_edit.text if is_node_ready() else _text_value

## When true, the field is non-interactive and visually dimmed.
@export var disabled: bool = false:
	set(value):
		disabled = value
		if is_node_ready():
			_apply_disabled_state()

@onready var _title_label: Label = %Label_Title
@onready var _description_label: Label = %Label_Description
@onready var _line_edit: LineEdit = %LineEdit_Input
@onready var _button: Button = %Button


func _ready() -> void:
	_update_title()
	_update_button()
	_update_description()
	_line_edit.placeholder_text = placeholder_text
	_line_edit.text = _text_value
	_apply_disabled_state()

	if Engine.is_editor_hint():
		return

	_line_edit.text_changed.connect(_on_line_edit_text_changed)
	_line_edit.text_submitted.connect(_on_line_edit_text_submitted)
	_line_edit.focus_entered.connect(_on_line_edit_focus_entered)
	_line_edit.focus_exited.connect(_on_line_edit_focus_exited)
	if _button:
		_button.pressed.connect(_on_button_pressed)


func _get_minimum_size() -> Vector2:
	return get_combined_minimum_size()


# -- Public API ----------------------------------------------------------------


func set_text(value: String) -> void:
	text = value


func get_text() -> String:
	return text


func clear() -> void:
	set_text("")


func select_all() -> void:
	if is_node_ready():
		_line_edit.select_all()


func custom_has_focus() -> bool:
	return _line_edit.has_focus() if is_node_ready() else false


func custom_release_focus() -> void:
	if is_node_ready():
		_line_edit.release_focus()


## Updates the description label text and color at runtime (e.g. for status).
func set_description_text_and_color(description_text: String, color: Color) -> void:
	if _description_label:
		_description_label.text = description_text
		_description_label.add_theme_color_override("font_color", color)
		_description_label.visible = not description_text.is_empty()
		update_minimum_size()


# -- Internal ----------------------------------------------------------------


func _update_title() -> void:
	if _title_label:
		_title_label.text = title
		_title_label.visible = not title.is_empty()
		update_minimum_size()


func _update_button() -> void:
	if _button:
		_button.text = text_button
		_button.visible = not text_button.is_empty()
		update_minimum_size()


func _update_description() -> void:
	if _description_label:
		_description_label.text = description
		_description_label.visible = not description.is_empty()
		update_minimum_size()


func _apply_disabled_state() -> void:
	if _line_edit:
		_line_edit.editable = not disabled
		_line_edit.mouse_default_cursor_shape = Control.CURSOR_ARROW if disabled else Control.CURSOR_IBEAM
	if _button:
		_button.disabled = disabled


func _on_line_edit_text_changed(new_text: String) -> void:
	text = new_text
	text_changed.emit(new_text)


func _on_line_edit_text_submitted(submitted_text: String) -> void:
	text_submitted.emit(submitted_text)


func _on_line_edit_focus_entered() -> void:
	custom_focus_entered.emit()


func _on_line_edit_focus_exited() -> void:
	custom_focus_exited.emit()


func _on_button_pressed() -> void:
	button_pressed.emit()
