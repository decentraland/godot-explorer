class_name Player
extends CharacterBody3D

const DEFAULT_CAMERA_FOV = 60.0
const SPRINTING_CAMERA_FOV = 75.0
const THIRD_PERSON_CAMERA = Vector3(0.75, 0, 3)  # X offset for over-shoulder view

# Double-jump + glide tuning (values mirror Unity CharacterControllerSettings.asset).
const MAX_AIR_JUMPS := 1
const JUMP_BUFFER_WINDOW := 0.15
const JUMP_COOLDOWN := 0.3
const AIR_JUMP_HEIGHT := 2.0
const AIR_JUMP_DELAY := 0.2
const AIR_JUMP_DIRECTION_IMPULSE := 8.0
const GLIDE_MAX_FALL_SPEED := 1.0
const GLIDE_HORIZONTAL_SPEED := 6.0
const GLIDE_MIN_GROUND_DISTANCE := 1.0
const JUMP_TO_GLIDE_INTERVAL := 0.5
const GLIDE_COOLDOWN := 0.6
const GLIDE_OPENING_TIME := 0.5
const GLIDE_CLOSING_TIME := 0.15

# Glide FSM values — mirror DclAvatar.glide_state and rfc4.Movement.GlideState.
const GLIDE_CLOSED := 0
const GLIDE_OPENING := 1
const GLIDE_GLIDING := 2
const GLIDE_CLOSING := 3

# What the jump button would do if pressed right now. Used by the UI to pick
# the matching icon. Mirrors the decision tree in _physics_process.
const JUMP_ACTION_NONE := 0
const JUMP_ACTION_JUMP := 1  # ground jump or air (double) jump
const JUMP_ACTION_GLIDE_TOGGLE := 2  # open or close the glider

# #b9: matches the CharacterBody3D.collision_mask in player.tscn (layer 2 =
# world/terrain). Keeps the ground raycast from pinging avatar wearables,
# triggers, or other non-ground CollisionObject3Ds.
const GROUND_RAYCAST_MASK := 2

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

var jump_count: int = 0
var glide_state: int = GLIDE_CLOSED

var camera_mode_change_blocked: bool = false
var stored_camera_mode_before_block: Global.CameraMode

var current_direction: Vector3 = Vector3()

var time_falling := 0.0
var current_profile_version: int = -1

# Private variables (prefixed with _)
var _hard_landing_timer: float = 0.0
var _locomotion_settings: DclLocomotionSettings = null
var _jump_buffer: float = 0.0
var _glide_timer: float = 0.0
var _time_since_last_jump: float = 1000.0
var _time_since_glide_end: float = 1000.0
var _air_jump_delay_timer: float = 0.0
var _air_jump_direction: Vector3 = Vector3.ZERO
var _ground_distance: float = INF
# #b11: typed Array[RID] avoids per-element dynamic cast when passed to
# PhysicsRayQueryParameters3D.exclude every physics frame.
var _raycast_exclude: Array[RID] = []

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
		avatar.set_rotation(Vector3(0, rotation.y, 0))
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
	add_child(PlayerGamepadInput.new(self))

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

	# Cache RIDs to exclude from ground-distance raycasts (player body itself +
	# avatar subtree colliders, including the TriggerDetector which would
	# otherwise make the ray report ~0m at all times).
	_build_raycast_exclude()

	# Avatar is top-level: initialize its world transform to match the player
	avatar.global_position = global_position
	avatar.rotation = Vector3(0, rotation.y, 0)


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
	# Keep the top-level avatar co-located with the player (picks up teleports,
	# external position changes, and ensures look_at below uses the correct origin)
	avatar.global_position = global_position

	# Handle hard landing cooldown
	if _hard_landing_timer > 0:
		_hard_landing_timer -= dt
		# During cooldown, prevent horizontal movement
		velocity.x = move_toward(velocity.x, 0, 20 * dt)
		velocity.z = move_toward(velocity.z, 0, 20 * dt)

	_jump_buffer = max(_jump_buffer - dt, 0.0)
	if Global.explorer_has_focus() and Input.is_action_just_pressed("ia_jump"):
		_jump_buffer = JUMP_BUFFER_WINDOW

	_time_since_last_jump = minf(_time_since_last_jump + dt, 1000.0)
	_time_since_glide_end = minf(_time_since_glide_end + dt, 1000.0)

	if glide_state == GLIDE_OPENING:
		_glide_timer -= dt
		if _glide_timer <= 0.0:
			glide_state = GLIDE_GLIDING
	elif glide_state == GLIDE_CLOSING:
		_glide_timer -= dt
		if _glide_timer <= 0.0:
			glide_state = GLIDE_CLOSED
			_time_since_glide_end = 0.0

	_ground_distance = _measure_ground_distance()

	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")
	var input_magnitude := clampf(input_dir.length(), 0.0, 1.0)

	if not Global.explorer_has_focus():  # ignore input
		input_dir = Vector2(0, 0)

	# Check input modifiers from current scene
	var all_disabled := Global.is_all_input_disabled()
	var walk_disabled := Global.is_walk_disabled()
	var jog_disabled := Global.is_jog_disabled()
	var run_disabled := Global.is_run_disabled()
	var jump_disabled := Global.is_jump_disabled()
	var double_jump_disabled := Global.is_double_jump_disabled()
	var glide_disabled := Global.is_glide_disabled()

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

	if !on_floor:
		time_falling += dt
	else:
		time_falling = 0.0

	# Air-jump hover phase: freeze gravity, then fire impulse + horizontal dash
	# when the timer expires. Leaves avatar.rise/fall untouched on purpose —
	# flipping them mid-hover would trip Jump_Fall → Jump_End via nfall and
	# strand the state machine away from Double_Jump_Rise when jump_count flips.
	if _air_jump_delay_timer > 0.0:
		_air_jump_delay_timer -= dt
		velocity.y = 0.0
		if _air_jump_delay_timer <= 0.0:
			velocity.y = sqrt(2.0 * AIR_JUMP_HEIGHT * gravity)
			var horiz_dir: Vector3 = Vector3(_air_jump_direction.x, 0.0, _air_jump_direction.z)
			if horiz_dir.length_squared() > 0.0001:
				horiz_dir = horiz_dir.normalized()
				velocity.x = horiz_dir.x * AIR_JUMP_DIRECTION_IMPULSE
				velocity.z = horiz_dir.z * AIR_JUMP_DIRECTION_IMPULSE
			jump_count += 1
			_time_since_last_jump = 0.0
			avatar.rise = true
			avatar.fall = false
	elif not on_floor:
		var in_grace_time = (
			time_falling < .2
			and !Input.is_action_pressed("ia_jump")
			and _time_since_last_jump >= JUMP_COOLDOWN
		)
		avatar.land = in_grace_time
		# rise/fall suppressed while the glider is providing lift (OPENING + GLIDING).
		# During CLOSING normal gravity resumes so Jump_Fall can take over.
		var free_flight: bool = glide_state == GLIDE_CLOSED or glide_state == GLIDE_CLOSING
		avatar.rise = velocity.y > .3 and free_flight
		avatar.fall = velocity.y < -.3 && !in_grace_time and free_flight
		velocity.y -= gravity * dt

		# Air-jump: 0.2s hover then impulse (matches Unity ApplyJump two-step).
		if (
			_jump_buffer > 0.0
			and jump_count >= 1
			and jump_count <= MAX_AIR_JUMPS
			and glide_state == GLIDE_CLOSED
			and not jump_disabled
			and not double_jump_disabled
			and _hard_landing_timer <= 0
			and _time_since_last_jump >= JUMP_COOLDOWN
		):
			_air_jump_delay_timer = AIR_JUMP_DELAY
			_air_jump_direction = current_direction
			_jump_buffer = 0.0

		# Glide toggle-open (mobile-friendly). Diverges from Unity's hold-to-glide
		# and from the Unity-exact `jump_count > MAX_AIR_JUMPS` entry gate — we
		# let a stepped-off-a-ledge player open glide without first double-jumping.
		# Air-jump still takes priority (above) because it consumes the buffer first.
		if _jump_buffer > 0.0 and glide_state == GLIDE_CLOSED:
			var gate_enabled := not jump_disabled and not glide_disabled
			var gate_altitude := _ground_distance > GLIDE_MIN_GROUND_DISTANCE
			var gate_jump_interval := _time_since_last_jump >= JUMP_TO_GLIDE_INTERVAL
			var gate_cooldown := _time_since_glide_end >= GLIDE_COOLDOWN
			if gate_enabled and gate_altitude and gate_jump_interval and gate_cooldown:
				glide_state = GLIDE_OPENING
				_glide_timer = GLIDE_OPENING_TIME
				_jump_buffer = 0.0
				avatar.rise = false
				avatar.fall = false

		# Glide close: re-press (toggle), altitude too low, or input disabled.
		# glide_disabled covers scene→scene transitions where the destination
		# forbids gliding: the force-close fires on the next tick after the
		# InputModifier update lands.
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			var exit_toggle := _jump_buffer > 0.0
			var exit_altitude := _ground_distance <= GLIDE_MIN_GROUND_DISTANCE
			var exit_disabled := jump_disabled or glide_disabled
			if exit_toggle or exit_altitude or exit_disabled:
				glide_state = GLIDE_CLOSING
				_glide_timer = GLIDE_CLOSING_TIME
				if exit_toggle:
					_jump_buffer = 0.0

		# Clamp fall speed from OPENING onward so the 0.5s opening window isn't free-fall.
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			if velocity.y < -GLIDE_MAX_FALL_SPEED:
				velocity.y = -GLIDE_MAX_FALL_SPEED
	elif (
		_jump_buffer > 0.0
		and not jump_disabled
		and _hard_landing_timer <= 0
		and _time_since_last_jump >= JUMP_COOLDOWN
	):
		# Ground jump — consume the buffer instead of reading the key again.
		var effective_jump_height := jump_height
		if Input.is_action_pressed("ia_sprint"):
			effective_jump_height = run_jump_height
		velocity.y = sqrt(2 * effective_jump_height * gravity)
		jump_count = 1
		_jump_buffer = 0.0
		_time_since_last_jump = 0.0
		avatar.land = false
		avatar.rise = true
		avatar.fall = false
	else:
		if not avatar.land:
			avatar.land = true
			if was_falling and hard_landing_cooldown > 0 and time_falling > 1.0:
				_hard_landing_timer = hard_landing_cooldown

		velocity.y = 0
		avatar.rise = false
		avatar.fall = false
		# Landing resets the air-jump budget and force-closes the glider.
		jump_count = 0
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			glide_state = GLIDE_CLOSING
			_glide_timer = GLIDE_CLOSING_TIME

	camera.set_target_fov(DEFAULT_CAMERA_FOV)
	if current_direction:
		var wants_walk := Input.is_action_pressed("ia_walk")
		var wants_sprint := Input.is_action_pressed("ia_sprint")

		# Determine the effective speed based on input modifiers
		var effective_speed := 0.0
		if wants_sprint and not run_disabled:
			camera.set_target_fov(SPRINTING_CAMERA_FOV)
			effective_speed = run_speed
		elif Global.is_mobile():
			# Analog speed: interpolate walk→jog based on stick displacement
			if walk_disabled and not jog_disabled:
				effective_speed = jog_speed
			elif jog_disabled and not walk_disabled:
				effective_speed = walk_speed
			elif not walk_disabled and not jog_disabled:
				effective_speed = lerpf(walk_speed, jog_speed, input_magnitude)
		elif wants_walk and not walk_disabled:
			effective_speed = walk_speed
		elif not jog_disabled:
			effective_speed = jog_speed
		elif not walk_disabled:
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

	# While gliding, cap horizontal speed — overrides walk/jog/run speeds set above.
	if glide_state == GLIDE_GLIDING:
		var horizontal := Vector2(velocity.x, velocity.z)
		if horizontal.length() > GLIDE_HORIZONTAL_SPEED:
			horizontal = horizontal.normalized() * GLIDE_HORIZONTAL_SPEED
			velocity.x = horizontal.x
			velocity.z = horizontal.y

	actual_velocity_xz = (to_xz(global_position) - to_xz(last_position)).length() / dt

	update_avatar_movement_state(actual_velocity_xz)

	# Mirror local physics state into DclAvatar so avatar.gd drives the
	# AnimationTree off the same numbers for both local and remote avatars.
	avatar.jump_count = jump_count
	avatar.glide_state = glide_state
	avatar.is_grounded = on_floor

	last_position = global_position
	move_and_slide()
	position.y = max(position.y, 0)
	avatar.global_position = global_position


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
	avatar.set_rotation(Vector3(0, y_rot + PI, 0))
	mount_camera.rotation.x = x_rot

	clamp_camera_rotation()


func set_avatar_rotation_independent(target_position: Vector3):
	# Set avatar to face target independently from camera (used when both avatar and camera targets provided)
	var global_pos := get_global_position()
	var target_direction = target_position - global_pos
	target_direction = target_direction.normalized()

	var y_rot = atan2(target_direction.x, target_direction.z)

	# Avatar is top-level, so set world-space Y rotation directly
	avatar.rotation.y = y_rot + PI


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
		rotation_y = avatar.rotation.y
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


func get_jump_action() -> int:
	if Global.is_jump_disabled() or Global.is_all_input_disabled():
		return JUMP_ACTION_NONE
	if _hard_landing_timer > 0.0:
		return JUMP_ACTION_NONE
	if is_on_floor() or position.y <= 0.0:
		return JUMP_ACTION_JUMP
	# Airborne. Report GLIDE_TOGGLE while the glider is open even if the
	# current scene disables gliding — the force-close in _physics_process
	# will transition to CLOSING on the next tick, and reporting NONE here
	# would flicker the icon in the intervening frame.
	if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
		return JUMP_ACTION_GLIDE_TOGGLE
	if glide_state == GLIDE_CLOSING:
		return JUMP_ACTION_NONE
	# glide_state == GLIDE_CLOSED. Air-jump takes priority over glide-open.
	if (
		jump_count >= 1
		and jump_count <= MAX_AIR_JUMPS
		and not Global.is_double_jump_disabled()
		and _time_since_last_jump >= JUMP_COOLDOWN
	):
		return JUMP_ACTION_JUMP
	if (
		not Global.is_glide_disabled()
		and _ground_distance > GLIDE_MIN_GROUND_DISTANCE
		and _time_since_last_jump >= JUMP_TO_GLIDE_INTERVAL
		and _time_since_glide_end >= GLIDE_COOLDOWN
	):
		return JUMP_ACTION_GLIDE_TOGGLE
	return JUMP_ACTION_NONE


# True while the next jump press would open or close the glider.
func can_toggle_glide() -> bool:
	if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
		return true
	if glide_state != GLIDE_CLOSED:
		return false
	# jump_count in [1..MAX_AIR_JUMPS] => next press fires air-jump, not glide-open.
	var grounded := is_on_floor() or position.y <= 0.0 or time_falling <= 0.0
	var input_blocked := (
		Global.is_jump_disabled() or Global.is_all_input_disabled() or Global.is_glide_disabled()
	)
	var air_jump_consumes_press := jump_count >= 1 and jump_count <= MAX_AIR_JUMPS
	var too_low := _ground_distance <= GLIDE_MIN_GROUND_DISTANCE
	var on_cooldown := (
		_time_since_last_jump < JUMP_TO_GLIDE_INTERVAL or _time_since_glide_end < GLIDE_COOLDOWN
	)
	if (
		grounded
		or input_blocked
		or _hard_landing_timer > 0.0
		or air_jump_consumes_press
		or too_low
		or on_cooldown
	):
		return false
	return true


func move_to(target: Vector3, check_stuck: bool = true):
	global_position = target
	velocity = Vector3.ZERO
	# #b15: teleports mid-glide (or mid-air-jump hover) must not carry the
	# glider lift / frozen gravity into the destination. Reset everything to a
	# grounded-idle baseline; _physics_process will re-derive on the next tick.
	jump_count = 0
	glide_state = GLIDE_CLOSED
	_glide_timer = 0.0
	_jump_buffer = 0.0
	_air_jump_delay_timer = 0.0
	if check_stuck and stuck_detector:
		stuck_detector.check_stuck()


# Distance from feet to ground for glide entry/close gating. Returns INF beyond
# 20m. Uses GROUND_RAYCAST_MASK (#b9) so it only sees the world/terrain layer;
# the exclude list (#b10) is kept as a belt-and-suspenders for avatar colliders
# that might briefly share the world mask.
func _measure_ground_distance() -> float:
	var space := get_world_3d().direct_space_state
	if space == null:
		return INF
	var from := global_position + Vector3(0.0, 0.1, 0.0)  # above feet to avoid self-hit
	var to := from + Vector3(0.0, -20.0, 0.0)
	var query := PhysicsRayQueryParameters3D.create(from, to)
	# #b9: restrict to terrain layer so wearables / triggers / scene gadgets
	# don't collapse the distance reading.
	query.collision_mask = GROUND_RAYCAST_MASK
	query.exclude = _raycast_exclude
	query.collide_with_bodies = true
	query.collide_with_areas = false
	var hit := space.intersect_ray(query)
	if hit.is_empty():
		return INF
	return from.y - (hit.position as Vector3).y


func _build_raycast_exclude() -> void:
	_raycast_exclude.clear()
	_raycast_exclude.append(get_rid())
	if avatar != null:
		_collect_collider_rids(avatar, _raycast_exclude)


func _collect_collider_rids(node: Node, out: Array) -> void:
	if node is CollisionObject3D:
		out.append((node as CollisionObject3D).get_rid())
	for c in node.get_children():
		_collect_collider_rids(c, out)
