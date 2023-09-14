extends CharacterBody3D

@onready var mount_camera := $Mount
@onready var camera: Camera3D = $Mount/Camera3D
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar

var first_person: bool = true
var _mouse_position = Vector2(0.0, 0.0)
var _touch_position = Vector2(0.0, 0.0)
var captured: bool = true

var is_on_air: bool

@export var vertical_sens: float = 0.5
@export var horizontal_sens: float = 0.5

var WALK_SPEED = 2.0
var RUN_SPEED = 6.0
var GRAVITY := 55.0
var JUMP_VELOCITY_0 := 12.0

var THIRD_PERSON_CAMERA = Vector3(0.5, 0, 3)


func _ready():
	camera.current = true

	# TODO: auto capture mouse
	# if captured:
	# 	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)

	first_person = false
	var tween_out = create_tween()
	tween_out.tween_property(camera, "position", THIRD_PERSON_CAMERA, 0.25).set_ease(
		Tween.EASE_IN_OUT
	)

	first_person = false
	avatar.show()

	floor_snap_length = 0.2

	Global.comms.update_profile_avatar(Global.config.avatar_profile)
	avatar.update_avatar(Global.config.avatar_profile)

	Global.config.param_changed.connect(self._on_param_changed)
	Global.comms.profile_changed.connect(self._on_player_profile_changed)


func _on_player_profile_changed(new_profile: Dictionary):
	avatar.update_avatar(new_profile)


func _on_param_changed(_param):
	WALK_SPEED = Global.config.walk_velocity
	RUN_SPEED = Global.config.run_velocity
	GRAVITY = Global.config.gravity
	JUMP_VELOCITY_0 = Global.config.jump_velocity


@onready var camera_fade_in_audio = preload("res://assets/sfx/ui_fade_in.wav")
@onready var camera_fade_out_audio = preload("res://assets/sfx/ui_fade_out.wav")
@onready var audio_stream_player_camera = $AudioStreamPlayer_Camera


func _input(event):
	# Receives touchscreen motion
	if Global.is_mobile:
		if event is InputEventScreenDrag:
			_touch_position = event.relative
			rotate_y(deg_to_rad(-_touch_position.x) * horizontal_sens)
			avatar.rotate_y(deg_to_rad(_touch_position.x) * horizontal_sens)
			mount_camera.rotate_x(deg_to_rad(-_touch_position.y) * vertical_sens)
			if first_person:
				mount_camera.rotation.x = clamp(
					mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90)
				)
			else:
				mount_camera.rotation.x = clamp(
					mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(45)
				)

	# Receives mouse motion
	if not Global.is_mobile && event:
		if event is InputEventMouseMotion && Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
			_mouse_position = event.relative
			rotate_y(deg_to_rad(-_mouse_position.x) * horizontal_sens)
			avatar.rotate_y(deg_to_rad(_mouse_position.x) * horizontal_sens)
			mount_camera.rotate_x(deg_to_rad(-_mouse_position.y) * vertical_sens)

		if first_person:
			mount_camera.rotation.x = clamp(
				mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90)
			)
		else:
			mount_camera.rotation.x = clamp(
				mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(45)
			)

		# Release mouse
		if event is InputEventKey:
			if event.keycode == KEY_TAB:
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

		# Toggle first or third person camera
		if event is InputEventMouseButton:
			if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				if first_person == true:
					first_person = false
					var tween_out = create_tween()
					(
						tween_out
						. tween_property(camera, "position", THIRD_PERSON_CAMERA, 0.25)
						. set_ease(Tween.EASE_IN_OUT)
					)
					avatar.show()
					avatar.set_rotation(Vector3(0, 0, 0))
					audio_stream_player_camera.stream = camera_fade_out_audio
					audio_stream_player_camera.play()

			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				if first_person == false:
					first_person = true
					var tween_in = create_tween()
					tween_in.tween_property(camera, "position", Vector3(0, 0, -0.2), 0.25).set_ease(
						Tween.EASE_IN_OUT
					)
					avatar.hide()
					audio_stream_player_camera.stream = camera_fade_in_audio
					audio_stream_player_camera.play()


var current_direction: Vector3 = Vector3()


func _physics_process(delta: float) -> void:
	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")
	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	current_direction = current_direction.move_toward(direction, 10 * delta)

	if not is_on_floor():
		velocity.y -= GRAVITY * delta

	elif Input.is_action_just_pressed("ia_jump"):
		velocity.y = JUMP_VELOCITY_0

	if current_direction:
		if Input.is_action_pressed("ia_walk"):
			avatar.set_walking()
			velocity.x = current_direction.x * WALK_SPEED
			velocity.z = current_direction.z * WALK_SPEED
		else:
			avatar.set_running()
			velocity.x = current_direction.x * RUN_SPEED
			velocity.z = current_direction.z * RUN_SPEED

		avatar.look_at(current_direction + position)
	else:
		avatar.set_idle()
		velocity.x = move_toward(velocity.x, 0, WALK_SPEED)
		velocity.z = move_toward(velocity.z, 0, WALK_SPEED)

	move_and_slide()
