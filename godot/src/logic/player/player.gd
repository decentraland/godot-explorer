class_name Player
extends CharacterBody3D

const DEFAULT_CAMERA_FOV = 75.0
const SPRINTING_CAMERA_FOV = 100.0
const THIRD_PERSON_CAMERA = Vector3(0.75, 0, 3)  # X offset for over-shoulder view

var last_position: Vector3
var actual_velocity_xz: float

# Locomotion settings - these are updated from the current scene's DclLocomotionSettings
var walk_speed: float = 1.5
var jog_speed: float = 8.0
var run_speed: float = 11.0
var gravity := 10.0
var jump_height: float = 1.8
var run_jump_height: float = 1.8
var hard_landing_cooldown: float = 0.0
var jump_velocity_0 := sqrt(2 * jump_height * gravity)

var jump_time := 0.0

var camera_mode_change_blocked: bool = false
var stored_camera_mode_before_block: Global.CameraMode

var current_direction: Vector3 = Vector3()

var time_falling := 0.0
var current_profile_version: int = -1

# Private variables (prefixed with _)
var _hard_landing_timer: float = 0.0
var _locomotion_settings: DclLocomotionSettings = null

@onready var mount_camera := $Mount
@onready var camera: DclCamera3D = $Mount/Camera3D
@onready var avatar_raycast: RayCast3D = $Mount/Camera3D/AvatarRaycast
@onready var outline_system: OutlineSystem = $Mount/Camera3D/OutlineSystem
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar
@onready var stuck_detector := $StuckDetector


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
		if camera.current:
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
	avatar.is_local_player = true
	avatar.activate_attach_points()

	floor_snap_length = 0.2

	Global.player_identity.profile_changed.connect(self._on_player_profile_changed)

	# Remove own avatar's click area as to avoid self-targeting
	var own_click_area = avatar.get_node("%ClickArea")
	if own_click_area:
		own_click_area.queue_free()

	# Setup trigger detection for local player's avatar
	# entity_id=1 (SceneEntityId::PLAYER)
	avatar.setup_trigger_detection(1)

	# Locomotion settings - subscribe to scene changes and settings updates
	Global.scene_runner.on_change_scene_id.connect(_on_scene_changed)
	Global.scene_runner.locomotion_settings_changed.connect(_on_locomotion_settings_changed)
	_on_scene_changed(Global.scene_runner.get_current_parcel_scene_id())


func _on_player_profile_changed(new_profile: DclUserProfile):
	var new_version = new_profile.get_profile_version()
	# Only update avatar if the profile version has changed
	if new_version != current_profile_version:
		current_profile_version = new_version
		avatar.async_update_avatar_from_profile(new_profile)


func _on_scene_changed(_scene_id: int) -> void:
	_locomotion_settings = Global.scene_runner.get_current_scene_locomotion_settings()
	_apply_locomotion_settings()


func _on_locomotion_settings_changed(settings: DclLocomotionSettings) -> void:
	_locomotion_settings = settings
	_apply_locomotion_settings()


func _apply_locomotion_settings() -> void:
	if _locomotion_settings == null:
		return

	walk_speed = _locomotion_settings.walk_speed
	jog_speed = _locomotion_settings.jog_speed
	run_speed = _locomotion_settings.run_speed
	jump_height = _locomotion_settings.jump_height
	run_jump_height = _locomotion_settings.run_jump_height
	hard_landing_cooldown = _locomotion_settings.hard_landing_cooldown
	jump_velocity_0 = sqrt(2 * jump_height * gravity)


func clamp_camera_rotation():
	# Maybe mobile wants a requires values
	if camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90))
	elif camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(35))


func _physics_process(dt: float) -> void:
	# Handle hard landing cooldown
	if _hard_landing_timer > 0:
		_hard_landing_timer -= dt
		# During cooldown, prevent horizontal movement
		velocity.x = move_toward(velocity.x, 0, 20 * dt)
		velocity.z = move_toward(velocity.z, 0, 20 * dt)

	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")

	if not Global.explorer_has_focus():  # ignore input
		input_dir = Vector2(0, 0)

	# Check input modifiers from current scene
	var all_disabled := Global.is_all_input_disabled()
	var walk_disabled := Global.is_walk_disabled()
	var jog_disabled := Global.is_jog_disabled()
	var run_disabled := Global.is_run_disabled()
	var jump_disabled := Global.is_jump_disabled()

	# If all input is disabled or during hard landing cooldown, clear input direction
	if all_disabled or _hard_landing_timer > 0:
		input_dir = Vector2(0, 0)

	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()

	# Determine movement basis: use active camera when virtual camera is active
	var movement_basis: Basis
	var active_camera = get_viewport().get_camera_3d()
	if active_camera != camera and is_instance_valid(active_camera):
		# Virtual camera is active - use its Y rotation (yaw) for movement direction
		movement_basis = Basis(Vector3.UP, active_camera.global_rotation.y)
	else:
		# Player camera is active - use player's transform
		movement_basis = transform.basis

	direction = (movement_basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()

	current_direction = current_direction.move_toward(direction, 8 * dt)

	var on_floor = is_on_floor() or position.y <= 0.0
	var was_falling = avatar.fall
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
	elif (
		Input.is_action_pressed("ia_jump")
		and jump_time < 0
		and not jump_disabled
		and _hard_landing_timer <= 0
	):
		# Use run_jump_height if sprinting
		var effective_jump_height := jump_height
		if Input.is_action_pressed("ia_sprint"):
			effective_jump_height = run_jump_height
		velocity.y = sqrt(2 * effective_jump_height * gravity)
		avatar.land = false
		avatar.rise = true
		avatar.fall = false
		jump_time = 1.5
	else:
		if not avatar.land:
			avatar.land = true
			# Check for hard landing (landing after falling for more than 1 second)
			if was_falling and hard_landing_cooldown > 0 and time_falling > 1.0:
				_hard_landing_timer = hard_landing_cooldown

		velocity.y = 0
		avatar.rise = false
		avatar.fall = false

	camera.set_target_fov(DEFAULT_CAMERA_FOV)
	if current_direction:
		var wants_walk := Input.is_action_pressed("ia_walk")
		var wants_sprint := Input.is_action_pressed("ia_sprint")

		# Determine the effective speed based on input modifiers
		var effective_speed := 0.0
		if wants_walk and not walk_disabled:
			effective_speed = walk_speed
		elif wants_sprint and not run_disabled:
			camera.set_target_fov(SPRINTING_CAMERA_FOV)
			effective_speed = run_speed
		elif not jog_disabled:
			effective_speed = jog_speed
		elif not walk_disabled:
			# Fallback to walk if jog is disabled but walk is allowed
			effective_speed = walk_speed
		# else: effective_speed remains 0, no movement allowed

		velocity.x = current_direction.x * effective_speed
		velocity.z = current_direction.z * effective_speed

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


func avatar_look_at(target_position: Vector3):
	var global_pos := get_global_position()
	var target_direction = target_position - global_pos
	target_direction = target_direction.normalized()

	var y_rot = atan2(target_direction.x, target_direction.z)
	var x_rot = atan2(
		target_direction.y,
		sqrt(target_direction.x * target_direction.x + target_direction.z * target_direction.z)
	)

	# Set player body, avatar, and camera to look at same target (backward compatibility)
	rotation.y = y_rot + PI
	avatar.set_rotation(Vector3(0, 0, 0))
	mount_camera.rotation.x = x_rot

	clamp_camera_rotation()


func set_avatar_rotation_independent(target_position: Vector3):
	# Set avatar to face target independently from camera (used when both avatar and camera targets provided)
	var global_pos := get_global_position()
	var target_direction = target_position - global_pos
	target_direction = target_direction.normalized()

	var y_rot = atan2(target_direction.x, target_direction.z)

	# Set avatar rotation relative to player body
	avatar.rotation.y = (y_rot + PI) - rotation.y


func camera_look_at(target_position: Vector3):
	var global_pos := get_global_position()
	var target_direction = target_position - global_pos
	target_direction = target_direction.normalized()

	var y_rot = atan2(target_direction.x, target_direction.z)
	var x_rot = atan2(
		target_direction.y,
		sqrt(target_direction.x * target_direction.x + target_direction.z * target_direction.z)
	)

	# Set player body Y rotation and camera mount X rotation (matches normal controls)
	rotation.y = y_rot + PI
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


func move_to(target: Vector3):
	global_position = target
	velocity = Vector3.ZERO
	if stuck_detector:
		stuck_detector.check_stuck()
