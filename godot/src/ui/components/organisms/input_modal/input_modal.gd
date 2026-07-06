class_name InputModal
extends ColorRect

## Emitted when the value was accepted: either no submit handler is set, or the
## handler returned "ok". The modal has already closed itself.
signal confirmed(value: String)
signal cancelled
## Emitted when the submit handler reported a non-recoverable error. The modal
## has already closed itself; the caller surfaces the message (e.g. a generic
## "try again later" modal).
signal failed(message: String)

const LOADING_SPINNER = preload(
	"res://src/ui/components/atoms/controls/loading_spinner/loading_spinner.tscn"
)

# Status values an async submit handler may return (see set_submit_handler):
#   "ok"      -> accepted: modal closes and emits `confirmed`.
#   "invalid" -> recoverable input error: modal stays open, shows `message`
#                inline in the field (like the local format validation).
#   "error"   -> fatal: modal closes and emits `failed(message)`.
const SUBMIT_OK = "ok"
const SUBMIT_INVALID = "invalid"
const SUBMIT_ERROR = "error"

## When false, tapping outside the modal does nothing. Default true to preserve
## existing behaviour for all other use cases.
var dismissable: bool = true
var _validation_callable: Callable
# Optional async handler injected by the caller. Receives the entered value and
# returns a Dictionary { "status": SUBMIT_*, "message": String }. When unset the
# modal stays "dumb": confirm just emits `confirmed` and closes.
var _submit_callable: Callable
var _confirm_text: String = ""
var _busy: bool = false
var _spinner: Control = null

@onready var label_title: Label = %Label_Title
@onready var label_subtitle: Label = %Label_Subtitle
@onready var dcl_text_edit: DclTextEdit = %DclTextEdit
@onready var button_confirm: Button = %Button_Confirm
@onready var button_cancel: Button = %Button_Cancel
@onready var _modal_panel: ResponsiveContainer = $Blur/VBoxContainer/PanelContainer2


func _ready() -> void:
	button_confirm.disabled = true
	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)

	# Spinner overlaid on the confirm button, shown while a submit handler runs.
	_spinner = LOADING_SPINNER.instantiate()
	_spinner.mouse_filter = Control.MOUSE_FILTER_IGNORE
	button_confirm.add_child(_spinner)
	_spinner.anchor_left = 0.5
	_spinner.anchor_top = 0.5
	_spinner.anchor_right = 0.5
	_spinner.anchor_bottom = 0.5
	_spinner.offset_left = -16.0
	_spinner.offset_top = -16.0
	_spinner.offset_right = 16.0
	_spinner.offset_bottom = 16.0
	_spinner.hide()


func _on_gui_input(event: InputEvent) -> void:
	if not dismissable or _busy:
		return
	if event is InputEventScreenTouch and event.pressed:
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
	_confirm_text = confirm_text
	label_title.text = title
	label_subtitle.text = subtitle
	dcl_text_edit.place_holder = placeholder
	button_confirm.text = confirm_text
	button_cancel.text = cancel_text


## Injects an async handler run when the user confirms. Without it the modal
## emits `confirmed` and closes immediately (legacy behavior). With it, the modal
## shows a spinner while awaiting the handler and acts on its returned status.
func set_submit_handler(handler: Callable) -> void:
	_submit_callable = handler


func open() -> void:
	dcl_text_edit.set_text_value("")
	_set_busy(false)
	button_confirm.disabled = true
	show()


func close() -> void:
	dcl_text_edit.set_text_value("")
	_set_busy(false)
	hide()


func _on_dcl_text_edit_changed() -> void:
	var text = dcl_text_edit.get_text_value()
	if dcl_text_edit.error:
		button_confirm.disabled = true
	elif _validation_callable.is_valid():
		button_confirm.disabled = not _validation_callable.call(text)
	else:
		button_confirm.disabled = text.is_empty()


# gdlint:ignore = async-function-name
func _on_button_confirm_pressed() -> void:
	if _busy:
		return
	var value := dcl_text_edit.get_text_value()

	# No handler: legacy behavior — accept and close immediately.
	if not _submit_callable.is_valid():
		confirmed.emit(value)
		close()
		return

	_set_busy(true)
	var result: Dictionary = await _submit_callable.call(value)
	_set_busy(false)

	match str(result.get("status", SUBMIT_OK)):
		SUBMIT_INVALID:
			# Recoverable: keep the modal open and show the error inline so the
			# user can fix the value and retry.
			dcl_text_edit.show_external_error(str(result.get("message", "")))
		SUBMIT_ERROR:
			# Fatal: close and let the caller surface a generic message.
			close()
			failed.emit(str(result.get("message", "")))
		_:
			confirmed.emit(value)
			close()


func _on_button_cancel_pressed() -> void:
	if _busy:
		return
	cancelled.emit()
	close()


# Toggles the in-flight state: swaps the confirm label for a spinner and blocks
# interaction (confirm/cancel disabled, background tap-to-close ignored).
func _set_busy(busy: bool) -> void:
	_busy = busy
	if _spinner:
		_spinner.visible = busy
	button_confirm.text = "" if busy else _confirm_text
	button_confirm.disabled = busy
	button_cancel.disabled = busy


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
