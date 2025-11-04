class_name PlayerMobileInput
extends Node

const VERTICAL_SENS: float = 0.5
const HORIZONTAL_SENS: float = 0.5

var _player: Player = null

var _touch_position = Vector2(0.0, 0.0)
var _positions: Array = [Vector2(), Vector2()]
var _start_length: int = 0
var _two_fingers = false


func _init(player: Player):
	_player = player


func _input(event):
	if not event:
		return

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

			if event is InputEventScreenDrag and _two_fingers:
				_positions[event.index] = event.position
				var zoom_length = (_positions[0] - _positions[1]).length()
				var zoom_amount = zoom_length - _start_length
				if (
					zoom_amount >= 50
					and _player.camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON
				):
					_player.set_camera_mode(Global.CameraMode.FIRST_PERSON)
					_start_length = zoom_length
				elif (
					zoom_amount <= -50
					and _player.camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON
				):
					_player.set_camera_mode(Global.CameraMode.THIRD_PERSON)
					_start_length = zoom_length
		else:
			_two_fingers = false

		if event is InputEventScreenDrag and !_two_fingers:
			_touch_position = event.relative
			# Only rotate the player on Y-axis, camera mount Y offset is preserved in local space
			_player.rotate_y(deg_to_rad(-_touch_position.x) * HORIZONTAL_SENS)
			_player.avatar.rotate_y(deg_to_rad(_touch_position.x) * HORIZONTAL_SENS)
			_player.mount_camera.rotate_x(deg_to_rad(-_touch_position.y) * VERTICAL_SENS)
			_player.clamp_camera_rotation()
