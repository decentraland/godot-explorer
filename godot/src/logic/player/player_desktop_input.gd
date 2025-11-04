class_name PlayerDesktopInput
extends Node

const VERTICAL_SENS: float = 0.5
const HORIZONTAL_SENS: float = 0.5

# macOS trackpad specific sensitivity
const MACOS_VERTICAL_SENS: float = 0.3
const MACOS_HORIZONTAL_SENS: float = 0.3

var _player: Player = null
var _mouse_position = Vector2(0.0, 0.0)
var _is_macos: bool = false


func _init(player: Player):
	_player = player
	_is_macos = OS.get_name() == "macOS"


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

	# Toggle first or third person camera
	if event is InputEventMouseButton:
		if !_player.camera_mode_change_blocked and Global.explorer_has_focus():
			if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				if _player.camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
					_player.set_camera_mode(Global.CameraMode.THIRD_PERSON)

			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				if _player.camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
					_player.set_camera_mode(Global.CameraMode.FIRST_PERSON)

	# Handle trackpad gestures on macOS (two-finger scroll/zoom)
	if _is_macos and event is InputEventPanGesture:
		if !_player.camera_mode_change_blocked and Global.explorer_has_focus():
			# Zoom out (third person) when scrolling down/away
			if event.delta.y > 0.1:
				if _player.camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
					_player.set_camera_mode(Global.CameraMode.THIRD_PERSON)

			# Zoom in (first person) when scrolling up/toward
			elif event.delta.y < -0.1:
				if _player.camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
					_player.set_camera_mode(Global.CameraMode.FIRST_PERSON)
