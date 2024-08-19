class_name PlayerDesktopInput
extends Node

const VERTICAL_SENS: float = 0.5
const HORIZONTAL_SENS: float = 0.5

var _player: Player = null

var _mouse_position = Vector2(0.0, 0.0)


func _init(player: Player):
	_player = player


func _input(event):
	if not event:
		return

	if not Global.explorer_has_focus():
		return

	# Receives mouse motion
	if event is InputEventMouseMotion && Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		_mouse_position = event.relative
		_player.rotate_y(deg_to_rad(-_mouse_position.x) * HORIZONTAL_SENS)
		_player.avatar.rotate_y(deg_to_rad(_mouse_position.x) * HORIZONTAL_SENS)
		_player.mount_camera.rotate_x(deg_to_rad(-_mouse_position.y) * VERTICAL_SENS)
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
