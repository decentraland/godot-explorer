class_name PlayerMobileInput
extends Node

const VERTICAL_SENS: float = 0.5
const HORIZONTAL_SENS: float = 0.5

var _player: Player = null
var _virtual_joystick: VirtualJoystick = null

var _touch_position = Vector2(0.0, 0.0)
var _positions: Array = [Vector2(), Vector2()]
var _start_length: int = 0
var _two_fingers = false


func _init(player: Player):
	_player = player
	Global.camera_mode_set.connect(_on_camera_mode_set)
	_resolve_joystick.call_deferred()


func _resolve_joystick() -> void:
	var explorer = Global.get_explorer()
	if explorer:
		_virtual_joystick = explorer.virtual_joystick


func _is_joystick_finger(index: int) -> bool:
	return _virtual_joystick and _virtual_joystick.touch_index == index


func _is_touch_over_chat(position: Vector2) -> bool:
	var explorer = Global.get_explorer()
	if not explorer:
		return false
	var chat_panel = explorer.chat_panel
	if is_instance_valid(chat_panel):
		return chat_panel.is_interactive_area_at(position)
	return false


func _input(event):
	if not event:
		return

	if not Global.explorer_has_focus():
		return

	# Receives touchscreen motion
	if Global.is_mobile() and (event is InputEventScreenTouch or event is InputEventScreenDrag):
		if _is_joystick_finger(event.index):
			return
		if _is_touch_over_chat(event.position):
			return

		var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")
		if input_dir == Vector2.ZERO and event.index < 2 and Global.explorer_has_focus():  # Not walking
			if event is InputEventScreenTouch:
				_positions[event.index] = event.position
				if event.index == 1:
					_two_fingers = event.pressed
					if event.pressed:
						_start_length = (_positions[0] - _positions[1]).length()
		else:
			_two_fingers = false

		if event is InputEventScreenDrag and !_two_fingers:
			_touch_position = event.relative
			# Avatar is top-level, so player Y rotation does not propagate to it
			_player.rotate_y(deg_to_rad(-_touch_position.x) * HORIZONTAL_SENS)
			_player.mount_camera.rotate_x(deg_to_rad(-_touch_position.y) * VERTICAL_SENS)
			_player.clamp_camera_rotation()


func _on_camera_mode_set(camera_mode: Global.CameraMode) -> void:
	if camera_mode != Global.CameraMode.CINEMATIC:
		_player.set_camera_mode(camera_mode)
