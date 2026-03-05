class_name PlayerMobileInput
extends Node

const VERTICAL_SENS: float = 0.5
const HORIZONTAL_SENS: float = 0.5
const JOYSTICK_CAMERA_SENS: float = 3.0
const JOYSTICK_CAMERA_DEADZONE: float = 0.15

var _player: Player = null
var _lb_held: bool = false

var _touch_position = Vector2(0.0, 0.0)
var _positions: Array = [Vector2(), Vector2()]
var _start_length: int = 0
var _two_fingers = false


func _init(player: Player):
	_player = player
	Global.camera_mode_set.connect(_on_camera_mode_set)

	# Erase gamepad button events so face buttons are handled manually with LB as modifier
	for action in ["ia_jump", "ia_primary", "ia_secondary", "ia_action_3", "ia_action_4"]:
		for event in InputMap.action_get_events(action):
			if event is InputEventJoypadButton:
				InputMap.action_erase_event(action, event)


func _input(event):
	if not event:
		return

	# Gamepad: handle all face buttons manually (joypad events erased from InputMap on mobile)
	if event is InputEventJoypadButton:
		_handle_gamepad_button(event)

	if not Global.explorer_has_focus():
		return

	# Receives touchscreen motion
	if Global.is_mobile() and (event is InputEventScreenTouch or event is InputEventScreenDrag):
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
			# Only rotate the player on Y-axis, camera mount Y offset is preserved in local space
			_player.rotate_y(deg_to_rad(-_touch_position.x) * HORIZONTAL_SENS)
			_player.avatar.rotate_y(deg_to_rad(_touch_position.x) * HORIZONTAL_SENS)
			_player.mount_camera.rotate_x(deg_to_rad(-_touch_position.y) * VERTICAL_SENS)
			_player.clamp_camera_rotation()


func _physics_process(dt: float) -> void:
	if not Global.explorer_has_focus():
		return

	# Right stick camera control (gamepad)
	var right_x := Input.get_joy_axis(0, JOY_AXIS_RIGHT_X)
	var right_y := Input.get_joy_axis(0, JOY_AXIS_RIGHT_Y)

	if absf(right_x) < JOYSTICK_CAMERA_DEADZONE:
		right_x = 0.0
	if absf(right_y) < JOYSTICK_CAMERA_DEADZONE:
		right_y = 0.0

	if right_x != 0.0 or right_y != 0.0:
		_player.rotate_y(deg_to_rad(-right_x) * JOYSTICK_CAMERA_SENS)
		_player.avatar.rotate_y(deg_to_rad(right_x) * JOYSTICK_CAMERA_SENS)
		_player.mount_camera.rotate_x(deg_to_rad(-right_y) * JOYSTICK_CAMERA_SENS)
		_player.clamp_camera_rotation()



## Handles gamepad buttons with LB as modifier for combo actions.
## Without LB: A=jump, B=primary(E), X=interact, Y=secondary(F)
## With LB held: A=combo1, B=combo2, X=combo3, Y=combo4
func _handle_gamepad_button(event: InputEventJoypadButton) -> void:
	# Track LB (left bumper) as modifier
	if event.button_index == JOY_BUTTON_LEFT_SHOULDER:
		_lb_held = event.pressed
		return

	# Face buttons: dispatch based on LB state
	var action := ""
	if _lb_held:
		match event.button_index:
			JOY_BUTTON_A: action = "ia_action_3"
			JOY_BUTTON_B: action = "ia_action_4"
			JOY_BUTTON_X: action = "ia_action_5"
			JOY_BUTTON_Y: action = "ia_action_6"
	else:
		match event.button_index:
			JOY_BUTTON_A: action = "ia_jump"
			JOY_BUTTON_B: action = "ia_primary"
			JOY_BUTTON_X: action = "ia_pointer"
			JOY_BUTTON_Y: action = "ia_secondary"

	if action.is_empty():
		return

	if event.pressed:
		Input.action_press(action)
	else:
		Input.action_release(action)


func _on_camera_mode_set(camera_mode: Global.CameraMode) -> void:
	_player.set_camera_mode(camera_mode)
