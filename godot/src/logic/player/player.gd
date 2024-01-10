extends CharacterBody3D

const THIRD_PERSON_CAMERA = Vector3(0.5, 0, 3)

@export var vertical_sens: float = 0.5
@export var horizontal_sens: float = 0.5

var captured: bool = true

var is_on_air: bool

var walk_speed = 2.0
var run_speed = 6.0
var gravity := 55.0
var jump_velocity_0 := 12.0

var camera_mode_change_blocked: bool = false
var stored_camera_mode_before_block: Global.CameraMode

var current_direction: Vector3 = Vector3()

var _mouse_position = Vector2(0.0, 0.0)
var _touch_position = Vector2(0.0, 0.0)

@onready var mount_camera := $Mount
@onready var camera: DclCamera3D = $Mount/Camera3D
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar

@onready var camera_fade_in_audio = preload("res://assets/sfx/ui_fade_in.wav")
@onready var camera_fade_out_audio = preload("res://assets/sfx/ui_fade_out.wav")
@onready var audio_stream_player_camera = $AudioStreamPlayer_Camera


func _on_camera_mode_area_detector_block_camera_mode(forced_mode):
	if !camera_mode_change_blocked:  # if it's already blocked, we don't store the state again...
		stored_camera_mode_before_block = camera.get_camera_mode() as Global.CameraMode
		camera_mode_change_blocked = true

	set_camera_mode(forced_mode, false)


func _on_camera_mode_area_detector_unblock_camera_mode():
	camera_mode_change_blocked = false
	set_camera_mode(stored_camera_mode_before_block, false)


func set_camera_mode(mode: Global.CameraMode, play_sound: bool = true):
	camera.set_camera_mode(mode)

	if mode == Global.CameraMode.THIRD_PERSON:
		var tween_out = create_tween()
		tween_out.tween_property(camera, "position", THIRD_PERSON_CAMERA, 0.25).set_ease(
			Tween.EASE_IN_OUT
		)
		avatar.show()
		avatar.set_rotation(Vector3(0, 0, 0))
		if play_sound:
			audio_stream_player_camera.stream = camera_fade_out_audio
			audio_stream_player_camera.play()
	elif mode == Global.CameraMode.FIRST_PERSON:
		var tween_in = create_tween()
		tween_in.tween_property(camera, "position", Vector3(0, 0, -0.2), 0.25).set_ease(
			Tween.EASE_IN_OUT
		)
		avatar.hide()
		if play_sound:
			audio_stream_player_camera.stream = camera_fade_in_audio
			audio_stream_player_camera.play()


func _ready():
	camera.current = true

	# TODO: auto capture mouse
	# if captured:
	# 	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	set_camera_mode(Global.CameraMode.THIRD_PERSON)
	avatar.activate_attach_points()

	floor_snap_length = 0.2

	Global.player_identity.profile_changed.connect(self._on_player_profile_changed)


func _on_player_profile_changed(new_profile: Dictionary):
	avatar.async_update_avatar_from_profile(new_profile)


func _on_param_changed(_param):
	walk_speed = Global.config.walk_velocity
	run_speed = Global.config.run_velocity
	gravity = Global.config.gravity
	jump_velocity_0 = Global.config.jump_velocity


func _clamp_camera_rotation():
	# Maybe mobile wants a requires values
	if camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90))
	elif camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(45))


func _input(event):
	# Receives touchscreen motion
	if Global.is_mobile:
		if event is InputEventScreenDrag:
			_touch_position = event.relative
			rotate_y(deg_to_rad(-_touch_position.x) * horizontal_sens)
			avatar.rotate_y(deg_to_rad(_touch_position.x) * horizontal_sens)
			mount_camera.rotate_x(deg_to_rad(-_touch_position.y) * vertical_sens)
			_clamp_camera_rotation()

	# Receives mouse motion
	if not Global.is_mobile && event:
		if event is InputEventMouseMotion && Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
			_mouse_position = event.relative
			rotate_y(deg_to_rad(-_mouse_position.x) * horizontal_sens)
			avatar.rotate_y(deg_to_rad(_mouse_position.x) * horizontal_sens)
			mount_camera.rotate_x(deg_to_rad(-_mouse_position.y) * vertical_sens)
			_clamp_camera_rotation()

		# Toggle first or third person camera
		if event is InputEventMouseButton and Global.explorer_has_focus():
			if !camera_mode_change_blocked:
				if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
					if camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
						set_camera_mode(Global.CameraMode.THIRD_PERSON)

				if event.button_index == MOUSE_BUTTON_WHEEL_UP:
					if camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
						set_camera_mode(Global.CameraMode.FIRST_PERSON)


func _physics_process(delta: float) -> void:
	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")

	if not Global.explorer_has_focus():  # ignore input
		input_dir = Vector2(0, 0)

	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	current_direction = current_direction.move_toward(direction, 10 * delta)

	if not is_on_floor():
		velocity.y -= gravity * delta

	elif Input.is_action_just_pressed("ia_jump"):
		velocity.y = jump_velocity_0

	if current_direction:
		if Input.is_action_pressed("ia_walk"):
			avatar.set_walking()
			velocity.x = current_direction.x * walk_speed
			velocity.z = current_direction.z * walk_speed
		else:
			avatar.set_running()
			velocity.x = current_direction.x * run_speed
			velocity.z = current_direction.z * run_speed

		avatar.look_at(current_direction.normalized() + position)
	else:
		avatar.set_idle()
		velocity.x = move_toward(velocity.x, 0, walk_speed)
		velocity.z = move_toward(velocity.z, 0, walk_speed)

	move_and_slide()


func avatar_look_at(target_position: Vector3):
	var global_pos := get_global_position()
	var target_direction = target_position - global_pos
	target_direction = target_direction.normalized()

	var y_rot = atan2(target_direction.x, target_direction.z)
	var x_rot = atan2(
		target_direction.y,
		sqrt(target_direction.x * target_direction.x + target_direction.z * target_direction.z)
	)

	rotation.y = y_rot + PI
	avatar.set_rotation(Vector3(0, 0, 0))
	mount_camera.rotation.x = x_rot

	_clamp_camera_rotation()
