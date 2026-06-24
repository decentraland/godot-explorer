class_name CodeModal
extends ColorRect

signal confirmed(value: String)
signal cancelled

var _code_inputs: Array[LineEdit] = []
var _hidden_input: LineEdit
var _read_only_style: StyleBox
var _focus_style: StyleBox
var _error_style: StyleBox

# Optional async verifier injected by the caller. Receives the entered code and
# must return "" on success or a friendly error string to show inline. When unset
# the modal behaves as a plain collector and emits `confirmed` for any 6 digits.
var _verify_callable: Callable
# Optional async resend handler injected by the caller. Same contract as the
# InputModal submit handler: returns {"status": SUBMIT_*, "message": String}.
var _resend_callable: Callable
var _resending: bool = false

@onready var hbox_verifying: HBoxContainer = %HBoxContainer_Verifying
@onready var _label_error: Label = %Label_Error
@onready var _label_subtitle: RichTextLabel = %Label_Subtitle
@onready var _label_resend: RichTextLabel = %RichTextLabel_ResendCode
@onready var _modal_panel: ResponsiveContainer = $Blur/PanelContainer2


func _ready() -> void:
	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)

	for i in range(1, 7):
		var line_edit: LineEdit = get_node("%" + "LineEdit_Code" + str(i))
		line_edit.editable = false
		_code_inputs.append(line_edit)

	_read_only_style = _code_inputs[0].get_theme_stylebox("read_only")
	_focus_style = _code_inputs[0].get_theme_stylebox("focus")

	var error_sb = _read_only_style.duplicate() as StyleBoxFlat
	error_sb.border_width_left = 1
	error_sb.border_width_top = 1
	error_sb.border_width_right = 1
	error_sb.border_width_bottom = 1
	error_sb.border_color = Color.RED
	_error_style = error_sb

	_label_error.hide()
	_set_verifying_children_visible(false)

	_hidden_input = LineEdit.new()
	_hidden_input.max_length = 6
	_hidden_input.virtual_keyboard_type = LineEdit.VirtualKeyboardType.KEYBOARD_TYPE_NUMBER
	_hidden_input.modulate = Color.TRANSPARENT
	_hidden_input.custom_minimum_size = Vector2.ZERO
	_hidden_input.size = Vector2.ZERO
	add_child(_hidden_input)
	_hidden_input.text_changed.connect(_on_hidden_input_changed)
	_hidden_input.gui_input.connect(_on_hidden_input_gui_input)

	for line_edit in _code_inputs:
		line_edit.gui_input.connect(_on_display_input_tapped)

	%TextureButton_Close.pressed.connect(_on_close_pressed)
	_label_resend.gui_input.connect(_on_resend_gui_input)


func _on_display_input_tapped(event: InputEvent) -> void:
	var is_tap = (
		(event is InputEventScreenTouch and event.pressed)
		or (
			event is InputEventMouseButton
			and event.pressed
			and event.button_index == MOUSE_BUTTON_LEFT
		)
	)
	if is_tap:
		if _hidden_input.has_focus():
			return
		if _label_error.visible:
			_clear_error()
			_clear_inputs()
			_hidden_input.editable = true
		_hidden_input.grab_focus()


func _on_hidden_input_changed(new_text: String) -> void:
	for i in range(_code_inputs.size()):
		if i < new_text.length():
			_code_inputs[i].text = new_text[i]
		else:
			_code_inputs[i].text = ""

	_update_focus_style(new_text.length())

	if new_text.length() == 6:
		_async_submit_code()


func _on_hidden_input_gui_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_V and event.is_command_or_control_pressed():
			var clipboard = DisplayServer.clipboard_get().strip_edges()
			if clipboard.length() > 0:
				_hidden_input.text = clipboard.left(6)
				_hidden_input.caret_column = _hidden_input.text.length()
				_on_hidden_input_changed(_hidden_input.text)
				_hidden_input.accept_event()


func _update_focus_style(filled_count: int) -> void:
	for i in range(_code_inputs.size()):
		if i == filled_count and i < _code_inputs.size():
			_code_inputs[i].add_theme_stylebox_override("read_only", _focus_style)
		else:
			_code_inputs[i].add_theme_stylebox_override("read_only", _read_only_style)


func set_verifier(verifier: Callable) -> void:
	_verify_callable = verifier


func set_resend_handler(handler: Callable) -> void:
	_resend_callable = handler


func _on_resend_gui_input(event: InputEvent) -> void:
	var is_tap = (
		(event is InputEventScreenTouch and event.pressed)
		or (
			event is InputEventMouseButton
			and event.pressed
			and event.button_index == MOUSE_BUTTON_LEFT
		)
	)
	if not is_tap or not _resend_callable.is_valid() or _resending:
		return
	_async_resend_code()


# gdlint:ignore = async-function-name
func _async_resend_code() -> void:
	_resending = true
	_clear_error()
	_clear_inputs()
	var result: Dictionary = await _resend_callable.call()
	if not is_instance_valid(self):
		return
	_resending = false
	var message: String = str(result.get("message", ""))
	if not message.is_empty():
		_show_error(message)
		return
	_hidden_input.editable = true
	_hidden_input.grab_focus()


# gdlint:ignore = async-function-name
func _async_submit_code() -> void:
	var code = get_code()
	if code.length() != 6:
		return

	_hidden_input.editable = false
	_set_verifying_children_visible(true)

	if not _verify_callable.is_valid():
		_set_verifying_children_visible(false)
		confirmed.emit(code)
		return

	var error_message: String = await _verify_callable.call(code)
	if not is_instance_valid(self):
		return
	if error_message.is_empty():
		_set_verifying_children_visible(false)
		confirmed.emit(code)
	else:
		_show_error(error_message)


func _show_error(message: String = "") -> void:
	_set_verifying_children_visible(false)
	if not message.is_empty():
		_label_error.text = message
	_label_error.show()
	for input in _code_inputs:
		input.add_theme_stylebox_override("read_only", _error_style)
		input.add_theme_color_override("font_uneditable_color", Color.RED)


func _clear_error() -> void:
	_label_error.hide()
	for input in _code_inputs:
		input.remove_theme_color_override("font_uneditable_color")


func get_code() -> String:
	return _hidden_input.text


func _on_gui_input(_event: InputEvent) -> void:
	pass


func _set_verifying_children_visible(value: bool) -> void:
	for child in hbox_verifying.get_children():
		child.visible = value


func open(email: String = "") -> void:
	_clear_inputs()
	_hidden_input.editable = true
	_set_verifying_children_visible(false)
	if email != "" and _label_subtitle:
		_label_subtitle.text = (
			"One time password sent to [b]%s[/b]. Please enter the code below to complete verification."
			% email
		)
	show()
	_hidden_input.grab_focus()


func close() -> void:
	_clear_inputs()
	hide()


func _on_close_pressed() -> void:
	cancelled.emit()
	close()


func _clear_inputs() -> void:
	_hidden_input.text = ""
	for input in _code_inputs:
		input.text = ""
	_update_focus_style(0)


func _on_visibility_changed() -> void:
	if not visible and _modal_panel != null:
		_modal_panel.vertical_offset = 0.0


func _on_virtual_keyboard_changed(keyboard_height: int) -> void:
	if keyboard_height == 0:
		_modal_panel.vertical_offset = 0.0
	else:
		var viewport_size = get_viewport().get_visible_rect().size
		var window_size = Vector2(DisplayServer.window_get_size())
		var y_factor = viewport_size.y / window_size.y
		_modal_panel.vertical_offset = -keyboard_height * y_factor * 0.5
