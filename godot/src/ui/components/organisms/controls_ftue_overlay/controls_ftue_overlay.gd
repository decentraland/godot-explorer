class_name ControlsFtueOverlay
extends Control

## Controls overlay shown on scene entry (issue #3014): dims the scene, labels the requested
## controls among Menu / Chat / Character movement / Emotes, and closes on a tap once the lock
## has elapsed.

signal dismissed

const FADE_SECONDS := 0.2
const STAGGER_SECONDS := 0.15
const LABEL_GAP := 8.0
# Radius of the joystick's resting base, which is drawn by a shader and has no rect of its own.
const JOYSTICK_BASE_RADIUS := 86.0
const MOUSE_INDEX := -1

# Label ids, in entrance order: top to bottom, then left to right.
const MENU := "menu"
const CHAT := "chat"
const MOVEMENT := "movement"
const EMOTES := "emotes"
const ALL_ELEMENTS: Array[String] = [MENU, CHAT, MOVEMENT, EMOTES]

var _menu_button: Control
var _chat_button: Control
var _joystick: VirtualJoystick
var _emotes_button: Control
var _elements: Array[String] = []
var _lock_seconds := 3.0
var _tap_through := false

var _unlocked := false
var _closing := false
# Presses swallowed here, so their drags and releases are swallowed too. A finger that was
# already down when the overlay appeared is not in it: MobileCameraInput must see its release.
var _owned_touches: Dictionary = {}

@onready var dim: ColorRect = %Dim
@onready var label_menu: ControlsFtueLabel = %Label_Menu
@onready var label_chat: ControlsFtueLabel = %Label_Chat
@onready var label_movement: ControlsFtueLabel = %Label_Movement
@onready var label_emotes: ControlsFtueLabel = %Label_Emotes
@onready var label_tap_to_start: Label = %Label_TapToStart


func setup(
	menu_button: Control,
	chat_button: Control,
	joystick: VirtualJoystick,
	emotes_button: Control,
	elements: Array[String],
	lock_seconds: float,
	tap_through: bool
) -> void:
	_menu_button = menu_button
	_chat_button = chat_button
	_joystick = joystick
	_emotes_button = emotes_button
	_elements = elements
	_lock_seconds = lock_seconds
	_tap_through = tap_through


func _ready() -> void:
	var by_element := {
		MENU: label_menu, CHAT: label_chat, MOVEMENT: label_movement, EMOTES: label_emotes
	}
	var labels: Array[ControlsFtueLabel] = []
	for element in ALL_ELEMENTS:
		var label: ControlsFtueLabel = by_element[element]
		label.visible = _elements.has(element)
		if label.visible:
			label.modulate.a = 0.0
			label.resized.connect(_place_labels)
			labels.append(label)
	dim.modulate.a = 0.0
	label_tap_to_start.modulate.a = 0.0
	resized.connect(_place_labels)
	_place_labels.call_deferred()

	var tween := create_tween().set_parallel(true)
	tween.tween_property(dim, "modulate:a", 1.0, FADE_SECONDS)
	for i in labels.size():
		tween.tween_property(labels[i], "modulate:a", 1.0, FADE_SECONDS).set_delay(
			STAGGER_SECONDS * (i + 1)
		)
	tween.chain().tween_interval(_lock_seconds)
	tween.chain().tween_callback(_unlock)


# _input, not _gui_input: MobileCameraInput recognizes pinches in _input, before any Control
# could stop them, and this node runs first because it sits later in the tree.
func _input(event: InputEvent) -> void:
	var index := MOUSE_INDEX
	var pressed := false
	if event is InputEventScreenTouch:
		index = event.index
		pressed = event.pressed
	elif event is InputEventMouseButton:
		pressed = event.pressed
	elif event is InputEventScreenDrag:
		if _owned_touches.has(event.index):
			get_viewport().set_input_as_handled()
		return
	else:
		return

	if not pressed:
		if _owned_touches.erase(index):
			get_viewport().set_input_as_handled()
		return
	if _closing:
		return
	if _unlocked and _tap_through:
		_begin_close()
		return
	_owned_touches[index] = true
	get_viewport().set_input_as_handled()
	if _unlocked:
		# Deferred: the mouse press emulated from this same touch arrives right after it and
		# must still be swallowed, or it clicks the control under the finger.
		_begin_close.call_deferred()


func _unlock() -> void:
	_unlocked = true
	create_tween().tween_property(label_tap_to_start, "modulate:a", 1.0, FADE_SECONDS)


func _begin_close() -> void:
	if _closing:
		return
	_closing = true
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	dismissed.emit()
	var tween := create_tween()
	tween.tween_property(self, "modulate:a", 0.0, FADE_SECONDS)
	tween.tween_callback(queue_free)


func _place_labels() -> void:
	if not is_instance_valid(_joystick):
		return
	var pointer := ControlsFtueLabel.POINTER_LENGTH
	var menu := _menu_button.get_global_rect()
	label_menu.global_position = Vector2(
		menu.get_center().x - label_menu.size.x * 0.5, menu.end.y + LABEL_GAP + pointer
	)
	var chat := _chat_button.get_global_rect()
	label_chat.global_position = Vector2(
		chat.end.x + LABEL_GAP + pointer, chat.get_center().y - label_chat.size.y * 0.5
	)
	var base := _joystick.get_base_global_center()
	label_movement.global_position = Vector2(
		base.x - label_movement.size.x * 0.5,
		base.y - JOYSTICK_BASE_RADIUS - LABEL_GAP - pointer - label_movement.size.y
	)
	var emotes := _emotes_button.get_global_rect()
	label_emotes.global_position = Vector2(
		emotes.end.x + LABEL_GAP + pointer, emotes.get_center().y - label_emotes.size.y * 0.5
	)
