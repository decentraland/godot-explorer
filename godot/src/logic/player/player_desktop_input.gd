class_name PlayerDesktopInput
extends Node

const VERTICAL_SENS: float = 0.5
const HORIZONTAL_SENS: float = 0.5
const JOYSTICK_CAMERA_SENS: float = 3.0
const JOYSTICK_CAMERA_DEADZONE: float = 0.15

# macOS trackpad specific sensitivity
const MACOS_VERTICAL_SENS: float = 0.3
const MACOS_HORIZONTAL_SENS: float = 0.3

var _player: Player = null
var _mouse_position = Vector2(0.0, 0.0)
var _is_macos: bool = false


func _init(player: Player):
	_player = player
	_is_macos = OS.get_name() == "macOS"
	Global.camera_mode_set.connect(_on_camera_mode_set)


func _input(event):
	if not event:
		return

	if not Global.explorer_has_focus():
		return

	# Receives mouse motion
	if event is InputEventMouseMotion && Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		_mouse_position = event.relative

		# Use different sensitivity for macOS trackpads
		var h_sens = MACOS_HORIZONTAL_SENS if _is_macos else HORIZONTAL_SENS
		var v_sens = MACOS_VERTICAL_SENS if _is_macos else VERTICAL_SENS

		# Apply smoothing for trackpad input on macOS
		if _is_macos:
			_mouse_position = _mouse_position * 0.8

		# Only rotate the player on Y-axis, let avatar handle its own rotation
		# Camera mount Y offset (from teleport) is preserved in local space
		_player.rotate_y(deg_to_rad(-_mouse_position.x) * h_sens)
		_player.avatar.rotate_y(deg_to_rad(_mouse_position.x) * h_sens)
		_player.mount_camera.rotate_x(deg_to_rad(-_mouse_position.y) * v_sens)
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


func _on_camera_mode_set(camera_mode: Global.CameraMode) -> void:
	_player.set_camera_mode(camera_mode)
