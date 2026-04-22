class_name Player
extends CharacterBody3D

const DebugLog = preload("res://src/logic/player/debug_log.gd")

const DEFAULT_CAMERA_FOV = 60.0
const SPRINTING_CAMERA_FOV = 75.0
const THIRD_PERSON_CAMERA = Vector3(0.75, 0, 3)  # X offset for over-shoulder view

# Double-jump + glide tuning. Values mirror Unity's CharacterControllerSettings.asset
# from PR #7312 / #6612. PBAvatarLocomotionSettings overrides will land here once
# the proto fields arrive (double_jump_height, gliding_speed, gliding_falling_speed).

# Jumps: ground counts as 1, each air jump increments jump_count. Gliding is
# gated on jump_count > MAX_AIR_JUMPS (i.e., used up all air jumps first).
const MAX_AIR_JUMPS := 1
const JUMP_BUFFER_WINDOW := 0.15  # input buffer + coyote time (Unity: JumpGraceTime)
const JUMP_COOLDOWN := 0.3  # min time between successive jumps (Unity: CooldownBetweenJumps)

# Double jump has its own height and a brief "hover" before the impulse fires.
# During the hover, gravity is overridden to zero (Unity: AirJumpGravityDuringDelay).
# After the hover, horizontal velocity is replaced by AIR_JUMP_DIRECTION_IMPULSE
# in the current input direction, giving the double-jump its characteristic dash.
const AIR_JUMP_HEIGHT := 2.0
const AIR_JUMP_DELAY := 0.2
const AIR_JUMP_DIRECTION_IMPULSE := 8.0

# Gliding: held-jump to stay aloft, released or altitude-low to close.
const GLIDE_MAX_FALL_SPEED := 1.0  # Unity: GlideMaxGravity
const GLIDE_HORIZONTAL_SPEED := 6.0  # Unity: GlideSpeed
# Unity ships 0.2m. We raise it to 1.0m so the close animation + Jump_Fall
# transition + Jump_End land clip all have time to play before the feet
# touch ground. With glide fall-speed clamped to 1 m/s, 1m of altitude gives
# ~1s of close-sequence headroom — enough for the avatar to look like it
# lands naturally instead of snapping from Glider_Idle to standing.
const GLIDE_MIN_GROUND_DISTANCE := 1.0  # altitude threshold for open/close gating
const JUMP_TO_GLIDE_INTERVAL := 0.5  # time-since-last-jump before glide can open
# Unity asset says 0.2s, code default 0.5s. We set 0.6s so the cooldown
# comfortably covers our full visual close sequence (CLOSING 0.15s + hide
# timer 0.25s = 0.4s) plus some margin, preventing a premature re-open that
# would make Glider_Start visually snap back over the closing Glider_End.
const GLIDE_COOLDOWN := 0.6
const GLIDE_OPENING_TIME := 0.5  # animation blend-in duration
# Shorter than Glider_End clip (0.267s) on purpose — the FSM just needs to
# hold CLOSING long enough to prevent an immediate re-open, while the prop's
# Glider_End clip and the hide timer handle the visual close in parallel.
const GLIDE_CLOSING_TIME := 0.15  # FSM-state holdoff after close press

# Glide FSM values — mirror DclAvatar.glide_state int and rfc4.Movement.glide_state enum.
const GLIDE_CLOSED := 0
const GLIDE_OPENING := 1
const GLIDE_GLIDING := 2
const GLIDE_CLOSING := 3

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
# Seconds since last jump impulse fired — gates JUMP_COOLDOWN and JUMP_TO_GLIDE_INTERVAL.
# Initialized high so the player can jump on frame 1.
var _time_since_last_jump: float = 1000.0
# Seconds since last glide CLOSING→CLOSED transition — gates GLIDE_COOLDOWN.
var _time_since_glide_end: float = 1000.0
# When > 0, the air-jump is in its "hover" phase: gravity is pinned to 0 and the
# real impulse fires once the timer elapses.
var _air_jump_delay_timer: float = 0.0
# Input direction captured at the moment the air-jump delay begins. Used to
# replace horizontal velocity with AIR_JUMP_DIRECTION_IMPULSE × this vector
# once the hover phase ends.
var _air_jump_direction: Vector3 = Vector3.ZERO
# Distance from the player's feet to the ground below, measured via a
# per-frame ray cast. Infinity when no ground is found within range.
var _ground_distance: float = INF
# Cached RID exclude list built in _ready. Lives as a member so we don't pay
# O(n) descendant collection on every physics frame.
var _raycast_exclude: Array = []
# Debug-only: remaining seconds for the per-frame "air-jump window" log. Set
# to ~1.0 when the air-jump delay starts so we capture hover + impulse + early
# descent as a continuous trace. Counts down in _physics_process.
var _dbg_airjump_window: float = 0.0
# Debug-only: remaining seconds for a per-frame "glide window" log. Set each
# time glide_state flips to OPENING/GLIDING/CLOSING so we capture the full
# open-glide-close trajectory with altitudes and velocities.
var _dbg_glide_window: float = 0.0
# Debug-only: dedup key for "glide entry BLOCKED" log — only emit when the
# combination of failed gates actually changes, so a held-jump-for-seconds
# produces one log line per distinct failure mode, not N per frame.
var _dbg_last_blocked_key: String = ""

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

	# Jump input buffering: a just-pressed jump stays "armed" for JUMP_BUFFER_WINDOW
	# seconds, covering cases like "press jump 0.1s before hitting the ground".
	_jump_buffer = max(_jump_buffer - dt, 0.0)
	if Global.explorer_has_focus() and Input.is_action_just_pressed("ia_jump"):
		_jump_buffer = JUMP_BUFFER_WINDOW
		DebugLog.log(
			"PLAYER",
			(
				"ia_jump JUST_PRESSED  count=%d glide=%d on_floor=%s gd=%.2f vy=%.2f"
				% [jump_count, glide_state, str(is_on_floor()), _ground_distance, velocity.y]
			)
		)
	if Global.explorer_has_focus() and Input.is_action_just_released("ia_jump"):
		DebugLog.log(
			"PLAYER",
			(
				"ia_jump JUST_RELEASED count=%d glide=%d on_floor=%s gd=%.2f vy=%.2f"
				% [jump_count, glide_state, str(is_on_floor()), _ground_distance, velocity.y]
			)
		)

	# Global timers used by cooldowns (JUMP_COOLDOWN, JUMP_TO_GLIDE_INTERVAL,
	# GLIDE_COOLDOWN). Capped at a sentinel to avoid float drift on long sessions.
	_time_since_last_jump = minf(_time_since_last_jump + dt, 1000.0)
	_time_since_glide_end = minf(_time_since_glide_end + dt, 1000.0)

	# Tick glide FSM timers (actual transitions live in the grounded/airborne blocks).
	if glide_state == GLIDE_OPENING:
		_glide_timer -= dt
		if _glide_timer <= 0.0:
			_set_glide_state(GLIDE_GLIDING, "opening-timer-elapsed")
	elif glide_state == GLIDE_CLOSING:
		_glide_timer -= dt
		if _glide_timer <= 0.0:
			_set_glide_state(GLIDE_CLOSED, "closing-timer-elapsed")
			_time_since_glide_end = 0.0

	# Tick debug trace windows.
	if _dbg_airjump_window > 0.0:
		_dbg_airjump_window -= dt
	if _dbg_glide_window > 0.0:
		_dbg_glide_window -= dt

	# Distance from feet to ground via downward ray. Used to gate glide entry
	# (GLIDE_MIN_GROUND_DISTANCE) and anticipate the close animation before
	# the character actually touches the floor.
	var prev_gd := _ground_distance
	_ground_distance = _measure_ground_distance()
	# Log only meaningful transitions (infinite ↔ finite, crossing the 0.2m
	# gate, or big jumps) so the log stays readable.
	var prev_inf := prev_gd == INF
	var curr_inf := _ground_distance == INF
	var prev_near := not prev_inf and prev_gd <= GLIDE_MIN_GROUND_DISTANCE
	var curr_near := not curr_inf and _ground_distance <= GLIDE_MIN_GROUND_DISTANCE
	if prev_inf != curr_inf or prev_near != curr_near:
		DebugLog.log(
			"PLAYER",
			(
				"ground_distance  %s  →  %s  (on_floor=%s vy=%.2f)"
				% [
					"INF" if prev_inf else "%.2f" % prev_gd,
					"INF" if curr_inf else "%.2f" % _ground_distance,
					str(is_on_floor()),
					velocity.y
				]
			)
		)

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

	# Air-jump hover phase: during AIR_JUMP_DELAY seconds after the double-jump
	# input, gravity is pinned to 0 (character floats) and at timer end we fire
	# the real impulse + replace horizontal velocity with the direction-change
	# kick. This reproduces Unity's "pause-then-dash" double-jump feel.
	#
	# We deliberately leave avatar.rise / avatar.fall untouched during the hover
	# — if we clear fall here the state machine takes `Jump_Fall → Jump_End` via
	# the `nfall` condition in mid-air, and when jump_count flips to 2 at impulse
	# time there's no transition out of Jump_End (you see the avatar "land in
	# the air" then re-fall). Keeping the pre-hover flags lets the double_jump
	# edge transition cleanly from Jump_Fall / Jump_Rise to Double_Jump_Rise.
	if _air_jump_delay_timer > 0.0:
		_air_jump_delay_timer -= dt
		velocity.y = 0.0  # override gravity
		if _air_jump_delay_timer <= 0.0:
			velocity.y = sqrt(2.0 * AIR_JUMP_HEIGHT * gravity)
			# Flatten the direction to the XZ plane — current_direction is
			# nominally horizontal but can pick up a tiny Y component from
			# basis multiplication / move_toward interpolation, which would
			# leak into velocity.y otherwise.
			var horiz_dir: Vector3 = Vector3(_air_jump_direction.x, 0.0, _air_jump_direction.z)
			if horiz_dir.length_squared() > 0.0001:
				horiz_dir = horiz_dir.normalized()
				velocity.x = horiz_dir.x * AIR_JUMP_DIRECTION_IMPULSE
				velocity.z = horiz_dir.z * AIR_JUMP_DIRECTION_IMPULSE
			jump_count += 1
			_time_since_last_jump = 0.0
			avatar.rise = true
			avatar.fall = false
			DebugLog.log(
				"PLAYER",
				(
					"air_jump IMPULSE count=%d vy=%.2f horiz=(%.2f,%.2f)"
					% [jump_count, velocity.y, velocity.x, velocity.z]
				)
			)
	elif not on_floor:
		var in_grace_time = (
			time_falling < .2
			and !Input.is_action_pressed("ia_jump")
			and _time_since_last_jump >= JUMP_COOLDOWN
		)
		avatar.land = in_grace_time
		# rise/fall are suppressed while the glider is actively providing lift
		# (OPENING + GLIDING). During CLOSING the glider is retracting and the
		# player is once again subject to normal gravity, so the Jump_Fall
		# animation should resume immediately — this is what drives the new
		# `Gliding_End → Jump_Fall` transition.
		var free_flight: bool = glide_state == GLIDE_CLOSED or glide_state == GLIDE_CLOSING
		avatar.rise = velocity.y > .3 and free_flight
		avatar.fall = velocity.y < -.3 && !in_grace_time and free_flight
		velocity.y -= gravity * dt

		# Air-jump trigger: enter the 0.2s hover phase. The actual impulse
		# fires when _air_jump_delay_timer expires (handled above on a later
		# frame). Unity's `ApplyJump.cs` does the same two-step pattern.
		if (
			_jump_buffer > 0.0
			and jump_count >= 1
			and jump_count <= MAX_AIR_JUMPS
			and glide_state == GLIDE_CLOSED
			and not jump_disabled
			and _hard_landing_timer <= 0
			and _time_since_last_jump >= JUMP_COOLDOWN
		):
			_air_jump_delay_timer = AIR_JUMP_DELAY
			_air_jump_direction = current_direction
			_jump_buffer = 0.0
			DebugLog.log(
				"PLAYER",
				(
					"air_jump DELAY_START count=%d vy=%.2f dir=%s dt_since_jump=%.2f"
					% [jump_count, velocity.y, _air_jump_direction, _time_since_last_jump]
				)
			)
			# Open the per-frame trace window for the next ~1s so the log
			# contains the full double-jump sequence (hover + impulse + early
			# descent) without spamming the rest of the session.
			_dbg_airjump_window = 1.0

		# Glide TOGGLE via press (mobile-friendly): pressing ia_jump while
		# CLOSED and airborne above GLIDE_MIN_GROUND_DISTANCE opens the
		# glider; pressing again while OPENING/GLIDING closes it. Unlike
		# Unity's hold-to-glide, the user does NOT have to keep the button
		# held — better for touchscreens with action buttons.
		#
		# We intentionally diverge from Unity here by NOT gating entry on
		# `jump_count > MAX_AIR_JUMPS`. That gate forced a ground+double jump
		# sequence before glide could open, which felt wrong for the common
		# case of stepping off a ledge and wanting to open the glider while
		# falling. Air jumps still take priority when the user does have
		# jump budget remaining (jump_count <= MAX_AIR_JUMPS, checked above
		# and consumes the buffer first), so the sequence "ground jump →
		# air jump → press for glide" still works as before.
		if _jump_buffer > 0.0 and glide_state == GLIDE_CLOSED:
			var gate_enabled := not jump_disabled
			var gate_altitude := _ground_distance > GLIDE_MIN_GROUND_DISTANCE
			var gate_jump_interval := _time_since_last_jump >= JUMP_TO_GLIDE_INTERVAL
			var gate_cooldown := _time_since_glide_end >= GLIDE_COOLDOWN
			if gate_enabled and gate_altitude and gate_jump_interval and gate_cooldown:
				_set_glide_state(GLIDE_OPENING, "toggle-open")
				_glide_timer = GLIDE_OPENING_TIME
				_jump_buffer = 0.0
				avatar.rise = false
				avatar.fall = false
			else:
				# Dedup log so a mashed-jump sequence doesn't flood the log.
				var blocked_key := (
					"%d%d%d%d"
					% [
						int(gate_enabled),
						int(gate_altitude),
						int(gate_jump_interval),
						int(gate_cooldown)
					]
				)
				if blocked_key != _dbg_last_blocked_key:
					_dbg_last_blocked_key = blocked_key
					(
						DebugLog
						. log(
							"PLAYER",
							(
								"glide entry BLOCKED enabled=%s alt=%s(gd=%s) jump_int=%s(dt=%.2f) cooldown=%s(dt=%.2f)"
								% [
									str(gate_enabled),
									str(gate_altitude),
									"INF" if _ground_distance == INF else "%.2f" % _ground_distance,
									str(gate_jump_interval),
									_time_since_last_jump,
									str(gate_cooldown),
									_time_since_glide_end,
								]
							)
						)
					)
		elif glide_state == GLIDE_CLOSED:
			_dbg_last_blocked_key = ""

		# Glide close via re-press (toggle), altitude too low, or input
		# disabled by a scene. CLOSING plays the Glider_End clip then
		# transitions to CLOSED via the FSM timer.
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			var exit_toggle := _jump_buffer > 0.0
			var exit_altitude := _ground_distance <= GLIDE_MIN_GROUND_DISTANCE
			var exit_disabled := jump_disabled
			if exit_toggle or exit_altitude or exit_disabled:
				var reason := (
					"toggle" if exit_toggle else ("altitude" if exit_altitude else "disabled")
				)
				_set_glide_state(
					GLIDE_CLOSING,
					"exit-%s (gd=%.2f vy=%.2f)" % [reason, _ground_distance, velocity.y]
				)
				_glide_timer = GLIDE_CLOSING_TIME
				if exit_toggle:
					_jump_buffer = 0.0

		# Glide flight modifiers — clamp fall speed from OPENING onward
		# (not just GLIDING). If the user opens the glider while falling
		# fast, the 0.5s opening window would otherwise be free-fall and
		# cost ~5-10m of altitude before the clamp engages.
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
		DebugLog.log(
			"PLAYER",
			"ground_jump count=1 vy=%.2f height=%.2f" % [velocity.y, effective_jump_height]
		)
	else:
		if not avatar.land:
			avatar.land = true
			# Check for hard landing (landing after falling for more than 1 second)
			if was_falling and hard_landing_cooldown > 0 and time_falling > 1.0:
				_hard_landing_timer = hard_landing_cooldown

		velocity.y = 0
		avatar.rise = false
		avatar.fall = false
		# Landing resets the air-jump budget and force-closes the glider.
		if jump_count != 0:
			DebugLog.log("PLAYER", "landed — jump_count reset from %d" % jump_count)
		jump_count = 0
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			_set_glide_state(GLIDE_CLOSING, "landed")
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

	# Per-frame detailed trace during active debug windows. Emits a compact
	# snapshot of the physics+animation-input state so the double-jump and
	# glide sequences can be reconstructed frame-by-frame from the log.
	if _dbg_airjump_window > 0.0 or _dbg_glide_window > 0.0:
		(
			DebugLog
			. log(
				"TRACE",
				(
					(
						"on_floor=%s  vy=%.2f  vxz=(%.2f,%.2f)  jc=%d  glide=%d  "
						+ "gd=%s  dj_timer=%.3f  buf=%.3f  dt_j=%.2f  dt_ge=%.2f  "
						+ "rise=%s fall=%s land=%s  jump_held=%s"
					)
					% [
						str(on_floor),
						velocity.y,
						velocity.x,
						velocity.z,
						jump_count,
						glide_state,
						"INF" if _ground_distance == INF else "%.2f" % _ground_distance,
						_air_jump_delay_timer,
						_jump_buffer,
						_time_since_last_jump,
						_time_since_glide_end,
						str(avatar.rise),
						str(avatar.fall),
						str(avatar.land),
						str(Input.is_action_pressed("ia_jump")),
					]
				)
			)
		)

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


func move_to(target: Vector3, check_stuck: bool = true):
	global_position = target
	velocity = Vector3.ZERO
	if check_stuck and stuck_detector:
		stuck_detector.check_stuck()


# Write wrapper around `glide_state` that logs the transition + reason so the
# closing glitch and early-close bugs are debuggable from the log alone.
func _set_glide_state(new_state: int, reason: String) -> void:
	if new_state == glide_state:
		return
	var names := ["CLOSED", "OPENING", "GLIDING", "CLOSING"]
	(
		DebugLog
		. log(
			"PLAYER",
			(
				"glide %s → %s  reason=%s  gd=%s vy=%.2f jc=%d dt_j=%.2f dt_ge=%.2f"
				% [
					names[glide_state],
					names[new_state],
					reason,
					"INF" if _ground_distance == INF else "%.2f" % _ground_distance,
					velocity.y,
					jump_count,
					_time_since_last_jump,
					_time_since_glide_end,
				]
			)
		)
	)
	glide_state = new_state
	# Open per-frame trace window after any glide transition — captures the
	# full OPENING → GLIDING → CLOSING → CLOSED trajectory with altitudes +
	# velocities so Bug 3 (early close) is diagnosable from one sample.
	_dbg_glide_window = 2.0


# Distance from the player's feet to the ground below, in meters. Returns INF
# when no ground is found within 20m. 20m lets a player falling at terminal
# velocity (~30-40 m/s) still see the ground coming with ~0.5s of lead time,
# giving the 0.5s glide-opening window a chance to matter before impact.
#
# We accept any collision layer (mask = 0xFFFFFFFF) and explicitly exclude
# this CharacterBody3D plus all CollisionObject3D descendants of the avatar
# (TriggerDetector, ClickArea, avatar_modifier_area_detector). DCL scenes
# don't use a consistent "world" collision layer we can rely on, so masking
# by layer was making the ray miss every terrain piece.
func _measure_ground_distance() -> float:
	var space := get_world_3d().direct_space_state
	if space == null:
		return INF
	var from := global_position + Vector3(0.0, 0.1, 0.0)  # above feet to avoid self-hit
	var to := from + Vector3(0.0, -20.0, 0.0)
	var query := PhysicsRayQueryParameters3D.create(from, to)
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
