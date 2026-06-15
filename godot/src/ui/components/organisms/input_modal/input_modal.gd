class_name InputModal
extends ColorRect

signal confirmed(value: String)
signal cancelled

var _validation_callable: Callable

@onready var label_title: Label = %Label_Title
@onready var label_subtitle: Label = %Label_Subtitle
@onready var dcl_text_edit: DclTextEdit = %DclTextEdit
@onready var button_confirm: Button = %Button_Confirm
@onready var button_cancel: Button = %Button_Cancel
@onready var keyboard_separator: HSeparator = %HSeparator_Keyboard


func _ready() -> void:
	button_confirm.disabled = true
	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func setup(
	title: String,
	subtitle: String,
	placeholder: String,
	confirm_text: String,
	cancel_text: String,
	validation: Callable,
) -> void:
	_validation_callable = validation
	label_title.text = title
	label_subtitle.text = subtitle
	dcl_text_edit.place_holder = placeholder
	button_confirm.text = confirm_text
	button_cancel.text = cancel_text


func open() -> void:
	dcl_text_edit.set_text_value("")
	button_confirm.disabled = true
	show()


func close() -> void:
	dcl_text_edit.set_text_value("")
	hide()


func _on_dcl_text_edit_changed() -> void:
	var text = dcl_text_edit.get_text_value()
	if dcl_text_edit.error:
		button_confirm.disabled = true
	elif _validation_callable.is_valid():
		button_confirm.disabled = not _validation_callable.call(text)
	else:
		button_confirm.disabled = text.is_empty()


func _on_button_confirm_pressed() -> void:
	confirmed.emit(dcl_text_edit.get_text_value())
	close()


func _on_button_cancel_pressed() -> void:
	cancelled.emit()
	close()


func _on_visibility_changed() -> void:
	if not visible and keyboard_separator != null:
		keyboard_separator.visible = false
		keyboard_separator.custom_minimum_size.y = 0


func _on_virtual_keyboard_changed(keyboard_height: int) -> void:
	if keyboard_height == 0:
		keyboard_separator.visible = false
		keyboard_separator.custom_minimum_size.y = 0
	else:
		var viewport_size = get_viewport().get_visible_rect().size
		var window_size = Vector2(DisplayServer.window_get_size())
		var y_factor = viewport_size.y / window_size.y
		keyboard_separator.custom_minimum_size.y = keyboard_height * y_factor
		keyboard_separator.visible = true
