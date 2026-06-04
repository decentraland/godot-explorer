class_name MobileCameraInput
extends Control

const HORIZONTAL_SENS: float = 0.5
const VERTICAL_SENS: float = 0.5

var _player: Player = null
var _chat_panel: Control = null
var _touch_positions: Dictionary = {}
var _drag_index: int = -1
var _two_fingers: bool = false


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
	_player.rotate_y(deg_to_rad(-event.relative.x) * HORIZONTAL_SENS)
	_player.mount_camera.rotate_x(deg_to_rad(-event.relative.y) * VERTICAL_SENS)
	_player.clamp_camera_rotation()
	accept_event()
