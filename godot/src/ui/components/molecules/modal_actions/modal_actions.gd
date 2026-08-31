@tool
class_name ModalActions
extends HBoxContainer

## The secondary/primary button pair that closes out a modal.
##
## `modal.tscn`, `input_modal.tscn` and the bug report form each re-declared this
## same HBox of two equal-width buttons plus a spinner overlay; this is the
## extraction. Styling comes entirely from `dcl_theme.tres` type variations
## (`SecondaryButton` for the left button, the default `Button` for the primary),
## so the pair picks up theme changes instead of pinning its own colours.

signal secondary_pressed
signal primary_pressed

const LOADING_SPINNER = preload(
	"res://src/ui/components/atoms/controls/loading_spinner/loading_spinner.tscn"
)

@export var secondary_text: String = "CANCEL":
	set(value):
		secondary_text = value
		if is_node_ready():
			button_secondary.text = value

@export var primary_text: String = "SUBMIT":
	set(value):
		primary_text = value
		_primary_label = value
		if is_node_ready() and not _busy:
			button_primary.text = value

var _busy: bool = false
var _primary_label: String = "SUBMIT"
var _spinner: Control = null

@onready var button_secondary: Button = %Button_Secondary
@onready var button_primary: Button = %Button_Primary


func _ready() -> void:
	button_secondary.text = secondary_text
	_primary_label = primary_text
	button_primary.text = primary_text

	# Overlaid on the primary button so the row keeps its height while in flight.
	_spinner = LOADING_SPINNER.instantiate()
	_spinner.mouse_filter = Control.MOUSE_FILTER_IGNORE
	button_primary.add_child(_spinner)
	_spinner.anchor_left = 0.5
	_spinner.anchor_top = 0.5
	_spinner.anchor_right = 0.5
	_spinner.anchor_bottom = 0.5
	_spinner.offset_left = -16.0
	_spinner.offset_top = -16.0
	_spinner.offset_right = 16.0
	_spinner.offset_bottom = 16.0
	_spinner.hide()


func set_primary_enabled(enabled: bool) -> void:
	if is_node_ready():
		button_primary.disabled = not enabled or _busy


## Swaps the primary label for a spinner and blocks both buttons. The caller is
## responsible for clearing it — including on the failure path.
func set_busy(busy: bool) -> void:
	_busy = busy
	if not is_node_ready():
		return
	if _spinner != null:
		_spinner.visible = busy
	button_primary.text = "" if busy else _primary_label
	button_primary.disabled = busy
	button_secondary.disabled = busy


func is_busy() -> bool:
	return _busy


func _on_button_secondary_pressed() -> void:
	secondary_pressed.emit()


func _on_button_primary_pressed() -> void:
	primary_pressed.emit()
