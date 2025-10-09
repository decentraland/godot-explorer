class_name Player
extends CharacterBody3D

const DEFAULT_CAMERA_FOV = 75.0
const SPRINTING_CAMERA_FOV = 100.0
const THIRD_PERSON_CAMERA = Vector3(0.75, 0, 3)  # X offset for over-shoulder view

var last_position: Vector3
var actual_velocity_xz: float

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
var forced_position: Vector3
var has_forced_position: bool = false

@onready var mount_camera := $Mount
@onready var camera: DclCamera3D = $Mount/Camera3D
@onready var avatar_raycast: RayCast3D = $Mount/Camera3D/AvatarRaycast
@onready var outline_system: OutlineSystem = $Mount/Camera3D/OutlineSystem
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar


func to_xz(pos: Vector3) -> Vector2:
	return Vector2(pos.x, pos.z)


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
		tween_out.set_parallel(true)
		(
			tween_out
			. tween_property(mount_camera, "spring_length", THIRD_PERSON_CAMERA.length(), 0.25)
			. set_ease(Tween.EASE_IN_OUT)
		)
		# Apply X offset for over-shoulder view in third person
		tween_out.tween_property(mount_camera, "position:x", THIRD_PERSON_CAMERA.x, 0.25).set_ease(
			Tween.EASE_IN_OUT
		)
		avatar.set_hidden(false)
		avatar.set_rotation(Vector3(0, 0, 0))
		if play_sound:
			UiSounds.play_sound("ui_fade_out")
	elif mode == Global.CameraMode.FIRST_PERSON:
		var tween_in = create_tween()
		tween_in.set_parallel(true)
		tween_in.tween_property(mount_camera, "spring_length", -.2, 0.25).set_ease(
			Tween.EASE_IN_OUT
		)
		# Remove X offset for centered view in first person
		tween_in.tween_property(mount_camera, "position:x", 0.0, 0.25).set_ease(Tween.EASE_IN_OUT)
		avatar.set_hidden(true)
		if play_sound:
			UiSounds.play_sound("ui_fade_in")


func update_avatar_movement_state(vel: float):
	avatar.walk = false
	avatar.jog = false
	avatar.run = false

	var speed_diffs = {
		"idle": abs(vel),
		"walk": abs(vel - walk_speed),
		"jog": abs(vel - jog_speed),
		"run": abs(vel - run_speed)
	}

	var nearest = speed_diffs.keys()[0]
	for key in speed_diffs.keys():
		if speed_diffs[key] < speed_diffs[nearest]:
			nearest = key

	match nearest:
		"walk":
			avatar.walk = true
		"jog":
			avatar.jog = true
		"run":
			avatar.run = true


func _ready():
	if Global.is_mobile():
		add_child(PlayerMobileInput.new(self))
	else:
		add_child(PlayerDesktopInput.new(self))

	# Setup the outline system with the main camera
	if outline_system:
		outline_system.setup(camera)

	camera.current = true

	set_camera_mode(Global.CameraMode.THIRD_PERSON, false)  # Don't play sound on initial setup
	avatar.activate_attach_points()

	floor_snap_length = 0.2

	Global.player_identity.profile_changed.connect(self._on_player_profile_changed)

	# Remove own avatar's click area as to avoid self-targeting
	var own_click_area = avatar.get_node("%ClickArea")
	if own_click_area:
		own_click_area.queue_free()


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
			velocity.x = current_direction.x * walk_speed
			velocity.z = current_direction.z * walk_speed
		elif Input.is_action_pressed("ia_sprint"):
			camera.set_target_fov(SPRINTING_CAMERA_FOV)
			velocity.x = current_direction.x * run_speed
			velocity.z = current_direction.z * run_speed
		else:
			velocity.x = current_direction.x * jog_speed
			velocity.z = current_direction.z * jog_speed

		avatar.look_at(current_direction.normalized() + position)
		avatar.rotation.x = 0.0
		avatar.rotation.z = 0.0
	else:
		velocity.x = move_toward(velocity.x, 0, walk_speed)
		velocity.z = move_toward(velocity.z, 0, walk_speed)

	actual_velocity_xz = (to_xz(global_position) - to_xz(last_position)).length() / dt

	update_avatar_movement_state(actual_velocity_xz)

	last_position = global_position
	move_and_slide()
	position.y = max(position.y, 0)

	if has_forced_position:
		global_position = forced_position
		velocity = Vector3.ZERO


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


func get_avatar_under_crosshair() -> Avatar:
	if not avatar_raycast:
		return null

	# Check if raycast is colliding
	if not avatar_raycast.is_colliding():
		return null

	var collider = avatar_raycast.get_collider()
	if not collider:
		return null

	# Check if this is an avatar collision area
	if collider.has_meta("is_avatar") and collider.get_meta("is_avatar"):
		# Walk up the node tree to find the Avatar node
		var node = collider
		while node:
			if node is Avatar:
				return node
			node = node.get_parent()

	return null


func async_move_to(target: Vector3):
	# Clear any previous forced position state
	has_forced_position = false

	var original_target = target
	global_position = target
	velocity = Vector3.ZERO
	await get_tree().physics_frame

	# If physics engine pushed us out due to collision, lock at original position to stay stuck
	# The player will remain stuck until either:
	# 1. The collider that caused the stuck state is removed/moves away
	# 2. async_move_to is called again with a new position
	if global_position.distance_to(original_target) > 0.01:
		forced_position = original_target
		has_forced_position = true
		global_position = original_target
