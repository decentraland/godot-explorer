class_name Player
extends CharacterBody3D

const DEFAULT_CAMERA_FOV = 75.0
const SPRINTING_CAMERA_FOV = 100.0
const THIRD_PERSON_CAMERA = Vector3(0.5, 0, 3)

var walk_speed = 1.5
var jog_speed = 8.0
var run_speed = 11.0
var gravity := 10.0
var jump_height := 1.0
var jump_velocity_0 := sqrt(2 * jump_height * gravity)

var jump_time := 0.0

var camera_mode_change_blocked: bool = false
var stored_camera_mode_before_block: Global.CameraMode

var current_direction: Vector3 = Vector3()

var time_falling := 0.0
var current_profile_version: int = -1

@onready var mount_camera := $Mount
@onready var camera: DclCamera3D = $Mount/Camera3D
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar


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
		(
			tween_out
			. tween_property(mount_camera, "spring_length", THIRD_PERSON_CAMERA.length(), 0.25)
			. set_ease(Tween.EASE_IN_OUT)
		)
		avatar.set_hidden(false)
		avatar.set_rotation(Vector3(0, 0, 0))
		if play_sound:
			UiSounds.play_sound("ui_fade_out")
	elif mode == Global.CameraMode.FIRST_PERSON:
		var tween_in = create_tween()
		tween_in.tween_property(mount_camera, "spring_length", -.2, 0.25).set_ease(
			Tween.EASE_IN_OUT
		)
		avatar.set_hidden(true)
		if play_sound:
			UiSounds.play_sound("ui_fade_in")


func _ready():
	if Global.is_mobile():
		add_child(PlayerMobileInput.new(self))
	else:
		add_child(PlayerDesktopInput.new(self))

	camera.current = true

	set_camera_mode(Global.CameraMode.THIRD_PERSON)
	avatar.activate_attach_points()

	floor_snap_length = 0.2

	Global.player_identity.profile_changed.connect(self._on_player_profile_changed)


func _on_player_profile_changed(new_profile: DclUserProfile):
	var new_version = new_profile.get_profile_version()
	# Only update avatar if the profile version has changed
	if new_version != current_profile_version:
		current_profile_version = new_version
		avatar.async_update_avatar_from_profile(new_profile)


func _on_param_changed(_param):
	# Disabled for now
	# TODO: make the panel to change these values
	# walk_speed = Global.get_config().walk_velocity
	# run_speed = Global.get_config().run_velocity
	# gravity = Global.get_config().gravity
	# jump_velocity_0 = Global.get_config().jump_velocity
	pass


func clamp_camera_rotation():
	# Maybe mobile wants a requires values
	if camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90))
	elif camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(35))


func _physics_process(dt: float) -> void:
	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")

	if not Global.explorer_has_focus():  # ignore input
		input_dir = Vector2(0, 0)

	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	current_direction = current_direction.move_toward(direction, 8 * dt)

	var on_floor = is_on_floor() or position.y <= 0.0
	jump_time -= dt

	if !on_floor:
		time_falling += dt
	else:
		time_falling = 0.0

	if not on_floor:
		var in_grace_time = (
			time_falling < .2 and !Input.is_action_pressed("ia_jump") and jump_time < 0
		)
		avatar.land = in_grace_time
		avatar.rise = velocity.y > .3
		avatar.fall = velocity.y < -.3 && !in_grace_time
		velocity.y -= gravity * dt
	elif Input.is_action_pressed("ia_jump") and jump_time < 0:
		velocity.y = jump_velocity_0
		avatar.land = false
		avatar.rise = true
		avatar.fall = false
		jump_time = 1.5
	else:
		if not avatar.land:
			avatar.land = true

		velocity.y = 0
		avatar.rise = false
		avatar.fall = false

	camera.set_target_fov(DEFAULT_CAMERA_FOV)
	if current_direction:
		if Input.is_action_pressed("ia_walk"):
			avatar.walk = true
			avatar.run = false
			avatar.jog = false
			velocity.x = current_direction.x * walk_speed
			velocity.z = current_direction.z * walk_speed
		elif Input.is_action_pressed("ia_sprint"):
			camera.set_target_fov(SPRINTING_CAMERA_FOV)
			avatar.walk = false
			avatar.run = true
			avatar.jog = false
			velocity.x = current_direction.x * run_speed
			velocity.z = current_direction.z * run_speed
		else:
			avatar.walk = false
			avatar.run = false
			avatar.jog = true
			velocity.x = current_direction.x * jog_speed
			velocity.z = current_direction.z * jog_speed

		avatar.look_at(current_direction.normalized() + position)
		avatar.rotation.x = 0.0
		avatar.rotation.z = 0.0
	else:
		avatar.walk = false
		avatar.run = false
		avatar.jog = false

		velocity.x = move_toward(velocity.x, 0, walk_speed)
		velocity.z = move_toward(velocity.z, 0, walk_speed)

	move_and_slide()
	position.y = max(position.y, 0)


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

	clamp_camera_rotation()


func _on_avatar_visibility_changed():
	pass  # Replace with function body.


func get_broadcast_position() -> Vector3:
	return avatar.get_global_transform().origin


func get_broadcast_rotation_y() -> float:
	var rotation_y := 0.0

	if camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
		rotation_y = rotation.y + avatar.rotation.y
	else:
		rotation_y = rotation.y

	# 1. Wrap into [-PI, PI) so we never go past the discontinuity
	rotation_y = wrapf(rotation_y, -PI, PI)

	# 2. Snap to 1-degree steps (≈0.01745 rad)
	const SNAP_STEP := 0.0174533  # PI / 180
	rotation_y = snapped(rotation_y, SNAP_STEP)
	return rotation_y
