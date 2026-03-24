class_name PlayerGamepadInput
extends Node

const JOYSTICK_CAMERA_DEADZONE: float = 0.15

var _player: Player = null
var _lb_held: bool = false
var _camera_sensitivity_multiplier: float = 0.0


func _init(player: Player):
	_player = player
	_refresh_camera_sensitivity_multiplier()
	Global.get_config().param_changed.connect(_on_config_param_changed)

	# Erase gamepad button events so face buttons are handled manually with LB as modifier
	for action in ["ia_jump", "ia_primary", "ia_secondary", "ia_action_3", "ia_action_4"]:
		for event in InputMap.action_get_events(action):
			if event is InputEventJoypadButton:
				InputMap.action_erase_event(action, event)


func _input(event):
	if not event:
		return

	if event is InputEventJoypadButton:
		_handle_gamepad_button(event)


func _physics_process(_dt: float) -> void:
	if not Global.explorer_has_focus():
		return

	# Right stick camera control
	var right_x := Input.get_joy_axis(0, JOY_AXIS_RIGHT_X)
	var right_y := Input.get_joy_axis(0, JOY_AXIS_RIGHT_Y)

	if absf(right_x) < JOYSTICK_CAMERA_DEADZONE:
		right_x = 0.0
	if absf(right_y) < JOYSTICK_CAMERA_DEADZONE:
		right_y = 0.0

	if right_x != 0.0 or right_y != 0.0:
		_player.rotate_y(deg_to_rad(-right_x) * _camera_sensitivity_multiplier)
		_player.avatar.rotate_y(deg_to_rad(right_x) * _camera_sensitivity_multiplier)
		_player.mount_camera.rotate_x(deg_to_rad(-right_y) * _camera_sensitivity_multiplier)
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
			JOY_BUTTON_A:
				action = "ia_action_3"
			JOY_BUTTON_B:
				action = "ia_action_4"
			JOY_BUTTON_X:
				action = "ia_action_5"
			JOY_BUTTON_Y:
				action = "ia_action_6"
	else:
		match event.button_index:
			JOY_BUTTON_A:
				action = "ia_jump"
			JOY_BUTTON_B:
				action = "ia_primary"
			JOY_BUTTON_X:
				action = "ia_pointer"
			JOY_BUTTON_Y:
				action = "ia_secondary"

	if action.is_empty():
		return

	if event.pressed:
		Input.action_press(action)
	else:
		Input.action_release(action)


func _on_config_param_changed(param: int) -> void:
	if param == ConfigData.ConfigParams.GAMEPAD_CAMERA_SENSITIVITY:
		_refresh_camera_sensitivity_multiplier()


func _refresh_camera_sensitivity_multiplier() -> void:
	_camera_sensitivity_multiplier = Global.get_config().gamepad_camera_sensitivity * 0.06
