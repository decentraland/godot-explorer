class_name MobileCameraInput
extends Control

# Gesture adopted from scene UI (finger pressed a UI element then swiped off it).
# Kept separate from the gui_input state below so it never collides with normal
# touches handled by this catcher.
enum AdoptedMode { NONE, CAMERA, JOYSTICK }

const HORIZONTAL_SENS: float = 0.5
const VERTICAL_SENS: float = 0.5

var _player: Player = null
var _chat_panel: Control = null
var _joystick: VirtualJoystick = null
var _touch_positions: Dictionary = {}
var _drag_index: int = -1
var _two_fingers: bool = false
# Adopted scene-UI swipe gestures, keyed by touch index → AdoptedMode. Keyed per
# finger so concurrent breakouts don't clobber each other's state.
var _adopted: Dictionary = {}


func _ready() -> void:
	if not Global.is_mobile():
		hide()
		set_process_input(false)
		return
	mouse_filter = MOUSE_FILTER_STOP
	gui_input.connect(_on_gui_input)
	_resolve_player.call_deferred()


func _resolve_player() -> void:
	var explorer := Global.get_explorer()
	if explorer:
		_player = explorer.player as Player
		_chat_panel = explorer.chat_panel
		_joystick = explorer.virtual_joystick as VirtualJoystick


func _is_chat_visible() -> bool:
	if not is_instance_valid(_chat_panel):
		var explorer := Global.get_explorer()
		if explorer:
			_chat_panel = explorer.chat_panel
	return is_instance_valid(_chat_panel) and _chat_panel.is_chat_visible()


func _on_gui_input(event: InputEvent) -> void:
	# Chat open: a tap outside it reaches this catcher (the chat's own controls are
	# STOP), so close the chat instead of moving the camera.
	if _is_chat_visible() and event is InputEventScreenTouch and event.pressed:
		Global.close_chat.emit()
		accept_event()
		return
	# A tap reaching this catcher means no UI consumed it: reclaim ui_root focus so
	# movement re-enables if some control stole it without handing it back.
	if event is InputEventScreenTouch and event.pressed and not Global.explorer_has_focus():
		Global.explorer_grab_focus()
	if Global.scene_runner.raycast_use_cursor_position:
		_handle_cinematic(event)
		return
	if _player == null:
		return
	if event is InputEventScreenTouch:
		_handle_touch(event)
	elif event is InputEventScreenDrag:
		_handle_drag(event)


func _handle_cinematic(event: InputEvent) -> void:
	if not (event is InputEventScreenTouch):
		return
	var explorer := Global.get_explorer()
	if explorer:
		explorer.set_cursor_position(event.position)
	if event.pressed:
		Input.action_press("ia_pointer")
	else:
		Input.action_release("ia_pointer")


func _handle_touch(event: InputEventScreenTouch) -> void:
	if event.pressed:
		_touch_positions[event.index] = event.position
		if _drag_index == -1:
			_drag_index = event.index
		if _touch_positions.size() >= 2:
			_two_fingers = true
			_drag_index = -1
	else:
		_touch_positions.erase(event.index)
		if event.index == _drag_index:
			_drag_index = -1
		if _touch_positions.size() < 2:
			_two_fingers = false
	accept_event()


func _handle_drag(event: InputEventScreenDrag) -> void:
	_touch_positions[event.index] = event.position
	if _two_fingers or event.index != _drag_index:
		return
	_player.apply_look_delta(event.relative)
	accept_event()


# --- Gesture handoff from scene UI ---------------------------------------------
# DclUiControl owns the touch (Godot drag-capture) once a finger presses an
# interactive scene-UI element. When the finger swipes off the element it forwards
# the gesture here, routed by where the finger first touched down.


## Adopt a gesture that broke out of a scene-UI element. `index` is the touch
## index; `start_position` is the original press point (decides camera vs joystick
## and seeds the joystick base); `current_position`/`relative` describe the drag at
## the breakout moment.
func adopt_touch(
	index: int, start_position: Vector2, current_position: Vector2, relative: Vector2
) -> void:
	if Global.scene_runner.raycast_use_cursor_position:
		return
	if _player == null:
		return
	if _joystick and _joystick.get_active_area_global_rect().has_point(start_position):
		_adopted[index] = AdoptedMode.JOYSTICK
		_joystick.external_begin(start_position)
		_joystick.external_update(current_position)
	else:
		_adopted[index] = AdoptedMode.CAMERA
		_player.apply_look_delta(relative)


func update_adopted_touch(index: int, position: Vector2, relative: Vector2) -> void:
	match _adopted.get(index, AdoptedMode.NONE):
		AdoptedMode.CAMERA:
			if _player:
				_player.apply_look_delta(relative)
		AdoptedMode.JOYSTICK:
			if _joystick:
				_joystick.external_update(position)


func release_adopted_touch(index: int) -> void:
	_end_adopted(index)


# Safety net: an adopted finger is owned (drag-captured) by its scene-UI control,
# which normally forwards the release. But _input sees every touch-up directly
# (before gui processing), so we still end the gesture if that control was freed or
# hidden mid-gesture before it could forward the release — otherwise the joystick
# could stay engaged (avatar walking) with no finger on screen.
func _input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and not event.pressed and _adopted.has(event.index):
		_end_adopted(event.index)


func _end_adopted(index: int) -> void:
	if _adopted.get(index, AdoptedMode.NONE) == AdoptedMode.JOYSTICK and _joystick:
		_joystick.external_end()
	_adopted.erase(index)
