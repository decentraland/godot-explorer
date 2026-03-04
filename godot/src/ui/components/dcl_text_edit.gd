@tool
class_name DclTextEdit
extends Control

signal dcl_text_edit_changed

const LINE_EDIT = preload("res://assets/themes/line_edit.tres")
const LINE_EDIT_FOCUSED = preload("res://assets/themes/line_edit_focused.tres")
const LINE_EDIT_ERROR = preload("res://assets/themes/line_edit_error.tres")
const LONG_PRESS_DURATION := 0.5

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
var _touched: bool = false
var _dragging: bool = false
var _drag_start_y: float = 0.0
var _drag_start_scroll: float = 0.0
var _long_press_timer: Timer
var _long_press_position: Vector2

@onready var label_length: Label = %Label_Length
@onready var label_error: Label = %Label_Error
@onready var text_edit: TextEdit = %TextEdit
@onready var clear_button: TextureButton = %ClearButton


func _ready() -> void:
	text_edit.focus_entered.connect(_on_text_edit_focus_entered)
	text_edit.focus_exited.connect(_on_text_edit_focus_exited)
	text_edit.gui_input.connect(_on_text_edit_gui_input)
	clear_button.button_down.connect(_on_clear_button_pressed)
	text_edit.placeholder_text = place_holder
	if wrap_text:
		text_edit.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	else:
		text_edit.wrap_mode = TextEdit.LINE_WRAPPING_NONE
	text_edit.get_v_scroll_bar().add_theme_constant_override("minimum_grab_thickness", 0)
	text_edit.get_v_scroll_bar().custom_minimum_size = Vector2.ZERO
	text_edit.get_v_scroll_bar().modulate = Color.TRANSPARENT
	text_edit.get_h_scroll_bar().add_theme_constant_override("minimum_grab_thickness", 0)
	text_edit.get_h_scroll_bar().custom_minimum_size = Vector2.ZERO
	text_edit.get_h_scroll_bar().modulate = Color.TRANSPARENT
	if has_max_length:
		label_length.show()
		label_length.text = "0/" + str(max_length)
	else:
		label_length.hide()
	_update_clear_button()
	_check_error()

	_long_press_timer = Timer.new()
	_long_press_timer.wait_time = LONG_PRESS_DURATION
	_long_press_timer.one_shot = true
	_long_press_timer.timeout.connect(_on_long_press)
	add_child(_long_press_timer)


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

	if error and _touched:
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


func _on_text_edit_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_dragging = false
			_drag_start_y = event.position.y
			_drag_start_scroll = text_edit.scroll_vertical
			_long_press_position = text_edit.get_global_transform() * event.position
			_long_press_timer.start()
		else:
			_dragging = false
			_long_press_timer.stop()
	elif event is InputEventScreenDrag:
		if not _dragging and absf(event.position.y - _drag_start_y) > 8.0:
			_dragging = true
			text_edit.deselect()
			_long_press_timer.stop()
		if _dragging:
			var line_height := text_edit.get_line_height()
			if line_height > 0:
				var delta_lines: float = (_drag_start_y - event.position.y) / float(line_height)
				text_edit.scroll_vertical = int(_drag_start_scroll + delta_lines)
			text_edit.get_viewport().set_input_as_handled()


func _on_long_press() -> void:
	text_edit.select_all()
	var menu := text_edit.get_menu()
	var allowed_ids: Array[int] = [TextEdit.MENU_COPY, TextEdit.MENU_PASTE, TextEdit.MENU_CLEAR]
	for i in range(menu.item_count - 1, -1, -1):
		if menu.get_item_id(i) not in allowed_ids:
			menu.remove_item(i)
	menu.reset_size()
	var menu_size := menu.size
	var viewport_size := text_edit.get_viewport().get_visible_rect().size
	var kb_height := DisplayServer.virtual_keyboard_get_height()
	var y_factor := viewport_size.y / float(DisplayServer.window_get_size().y)
	var available_bottom := viewport_size.y - kb_height * y_factor
	var pos := Vector2i(_long_press_position)

	if pos.y + menu_size.y > int(available_bottom):
		pos.y = maxi(0, pos.y - menu_size.y)

	menu.position = pos
	menu.popup()


func _on_text_edit_focus_entered() -> void:
	_touched = true
	_update_clear_button()


func _on_text_edit_focus_exited() -> void:
	call_deferred("_update_clear_button")


func _update_clear_button() -> void:
	clear_button.visible = text_edit.text.length() > 0 and text_edit.has_focus()


func _on_clear_button_pressed() -> void:
	text_edit.text = ""
	_update_length()
	_update_clear_button()
	_check_error()
	emit_signal("dcl_text_edit_changed")


func _on_text_edit_text_changed() -> void:
	if text_edit.text.contains("\n"):
		text_edit.text = text_edit.text.replace("\n", "")
		text_edit.set_caret_column(text_edit.text.length())
		text_edit.release_focus()
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
	_touched = false
	text_edit.text = new_text
	_update_length()
	_update_clear_button()
	_check_error()
