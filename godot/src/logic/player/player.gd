class_name Player
extends CharacterBody3D

const DEFAULT_CAMERA_FOV = 60.0
const SPRINTING_CAMERA_FOV = 75.0

# Double-jump + glide tuning (values mirror Unity CharacterControllerSettings.asset).
const MAX_AIR_JUMPS := 1
const JUMP_BUFFER_WINDOW := 0.15
const JUMP_COOLDOWN := 0.3
# Coyote-style debounce on the ANIMATION grounded flag: at mobile physics
# rates a 1-tick is_on_floor() flicker replayed the landing pose (the "bounce").
const GROUNDED_GRACE_WINDOW := 0.15
const AIR_JUMP_HEIGHT := 2.0
const AIR_JUMP_DELAY := 0.2
const AIR_JUMP_DIRECTION_IMPULSE := 8.0
const GLIDE_MAX_FALL_SPEED := 1.0
const GLIDE_HORIZONTAL_SPEED := 6.0
# #2854: Unity settings values — 0.2 min ground distance, 0.2 re-open cooldown.
const GLIDE_MIN_GROUND_DISTANCE := 0.2
const JUMP_TO_GLIDE_INTERVAL := 0.5
const GLIDE_COOLDOWN := 0.2
const GLIDE_OPENING_TIME := 0.5
const GLIDE_CLOSING_TIME := 0.15

# Character mass for scene-driven impulses/forces (matches Unity for tuning parity).
# Δv = impulse / CHARACTER_MASS, a = force / CHARACTER_MASS.
const CHARACTER_MASS := 1.0
# Viscous drag on external_velocity each tick.
# Total damping = ENV (always) + GROUND_FRICTION (when grounded).
const EXT_ENV_DRAG := 1.5
const EXT_GROUND_FRICTION := 4.0
# Hard ceiling on |external_velocity|.
const MAX_EXTERNAL_VELOCITY := 50.0
# Snap external_velocity to zero below this squared magnitude.
const EXT_VELOCITY_EPSILON_SQR := 0.0001

# Glide FSM values — mirror DclAvatar.glide_state and rfc4.Movement.GlideState.
const GLIDE_CLOSED := 0
const GLIDE_OPENING := 1
const GLIDE_GLIDING := 2
const GLIDE_CLOSING := 3

# #2850: Unity parity (ApplyCharacterMovementVelocity.cs / ApplyHorizontalAirDrag.cs).
# Acceleration weight ramps 0→1 over 0.5s while input is held; the accel pair
# lerps with it. Air drag is quadratic: AirDrag 0.05 × JumpVelocityDrag 4.
const ACCELERATION_TIME := 0.5
const GROUND_ACCEL := 20.0
const GROUND_ACCEL_MAX := 25.0
const AIR_ACCEL := 15.0
const AIR_ACCEL_MAX := 20.0
const AIR_DRAG := 0.2

# #1557: Unity parity (ApplyJump.cs / ApplyGravity.cs). Seconds, never ticks.
const COYOTE_WINDOW := 0.15
const GRAVITY_ASCENT_FACTOR := 4.0
const LONG_JUMP_TIME := 0.5
const LONG_JUMP_GRAVITY_SCALE := 0.5

# What the jump button would do if pressed right now. Used by the UI to pick
# the matching icon. Mirrors the decision tree in _physics_process.
const JUMP_ACTION_NONE := 0
const JUMP_ACTION_JUMP := 1  # ground jump or air (double) jump
const JUMP_ACTION_GLIDE_TOGGLE := 2  # open or close the glider

# AvatarRaycast resting target (matches player.tscn): straight ahead, 10m.
const AVATAR_RAYCAST_DEFAULT_TARGET := Vector3(0, 0, -10)
# Duration of the camera mode tween (set_camera_mode).
const CAMERA_MODE_TWEEN_TIME := 0.25
# Volume of the 1p<->3p transition sound (QA: -3dB from the default).
const CAMERA_TRANSITION_SOUND_DB := -3.0
# Crosshair model (issue #2709, device QA): first person is screen center. For
# third person the target goes through three phases — hold center for the first
# half of the camera tween, glide to the PLACEHOLDER anchor until the camera
# settles, then track the live projection: CROSSHAIR_TOP_GAP_PX above the
# avatar's on-screen top edge, at its side edge's x.
const CROSSHAIR_PLACEHOLDER_ANCHOR := Vector2(0.53, 0.44)
const CROSSHAIR_TOP_GAP_PX := 20.0
const CROSSHAIR_SCREEN_MARGIN := 24.0
# Top-of-head height above the player origin and half body width, for the live
# projection of the avatar's on-screen top/side edges.
const AVATAR_TOP_HEIGHT := 1.8
const AVATAR_HALF_WIDTH := 0.35

# #b9: matches the CharacterBody3D.collision_mask in player.tscn (layer 2 =
# world/terrain). Keeps the ground raycast from pinging avatar wearables,
# triggers, or other non-ground CollisionObject3Ds.
const GROUND_RAYCAST_MASK := 2

# #2753: Unity parity (ApplySlopeModifier.cs / CharacterObject.prefab).
# CharacterBody3D has no built-in step offset (M1) — custom logic below.
# Max climbable top above the support contact: Unity's effective stepOffset
# (0.35 + PhysX contact skin) lands here — live QA at kuruk.dcl.eth: 0.43
# climbs, 0.44 blocks.
const STEP_MAX_HEIGHT := 0.4375
# Below this the rise machinery isn't worth engaging — protects against
# teleporting DOWN onto lower surfaces read past the face.
const STEP_MIN_RISE := 0.02
# Rises below this are silent ride-assists: they don't count toward the
# two-unrest-rises disarm chain (terrain ripple / collider lips).
const STEP_PENDING_RISE := 0.05
# A rise above this disarms the step-up until the capsule rests (anti
# ramp-climbing — a 50deg ramp rises ~0.31 per band); legit single steps
# (kuruk's first tread measures 0.20-0.21 real) must stay below it.
const STEP_TALL_RISE := 0.25
# Predictive pass skips while walking a climbable slope (floor normal off
# vertical by more than ~8deg) — stepping there turns inclines into stutter.
const SLOPE_WALK_NORMAL_Y := 0.99
# #2852 M2: Unity SlopeVelocityModifier curve — linear keys (-55deg, 1.35)
# -> (0, 1) -> (55deg, 0.65), clamped beyond. Positive angle = uphill.
const SLOPE_MOD_MAX_DEG := 55.0
const SLOPE_MOD_DOWNHILL := 1.35
const SLOPE_MOD_UPHILL := 0.65
# #2852 M4: ground contact further than this from the capsule axis tilts
# gravity toward it (Unity NoSlipDistance).
const EDGE_NO_SLIP_DIST := 0.1
# #2852 M5: head-on wall contact multiplies speed toward this (Unity
# WallSlideMaxMoveSpeedMultiplier = 0), lerped by |facing · wall normal|.
const WALL_SLIDE_MIN_MULT := 0.0
# #2852 M6: hard landing = fall height above this (Unity JumpHeightStun),
# stun duration (Unity LongFallStunTime).
const HARD_LANDING_FALL_HEIGHT := 8.0
const HARD_LANDING_STUN_TIME := 0.75
# #2854 M12: external-force multiplier while gliding (Unity GlideWindResponse).
const GLIDE_WIND_RESPONSE := 1.5
# cos(46deg): a slide collision flatter than this is walkable ground; steeper
# is a ramp/wall face — sliding on one must not count as support.
const WALKABLE_NORMAL_Y := 0.695
# Realm floor idiom (same as `on_floor` in _physics_process): the y=0 clamp
# holds the capsule with no collider contact at all — floor jitter is ~1mm.
const REALM_FLOOR_EPS := 0.005
# Downslope stick, expressed as floor_snap_length so is_on_floor() survives
# downhill moves (a manual raycast snap would report airborne mid-stick).
const DOWNSLOPE_STICK_JOG := 0.45
const DOWNSLOPE_STICK_RUN := 0.55
# Unity serializes 46deg; engine default 45 is wrong.
const SLOPE_LIMIT_DEG := 46.0
# Capsule dims from player.tscn (CollisionShape3D_Body).
const CAPSULE_CENTER_Y := 0.8
const CAPSULE_RADIUS := 0.3

var last_position: Vector3
var actual_velocity_xz: float
# #2856: gait kind from input mode (0 idle / 1 walk / 2 jog / 3 run) — drives
# the avatar's walk/jog/run classification for the continuous gait blend.
var movement_kind: int = 2
var has_move_input: bool = false

# Locomotion settings - these are updated from the current scene's DclLocomotionSettings
var walk_speed: float = 1.5
var jog_speed: float = 8.0
var run_speed: float = 10.0
var gravity := 10.0
# #1557: jog/run jump heights lerped by horizontal speed (Unity: 1.0 / 1.5).
var jump_height: float = 1.0
var run_jump_height: float = 1.5
var hard_landing_cooldown: float = 0.0

var jump_count: int = 0
var glide_state: int = GLIDE_CLOSED

var camera_mode_change_blocked: bool = false
var stored_camera_mode_before_block: Global.CameraMode

var current_direction: Vector3 = Vector3()

var time_falling := 0.0
var current_profile_version: int = -1

# Persistent velocity accumulating scene-driven impulses (full XYZ) and forces
# (XZ only — force.y feeds gravity). Decays via drag each tick.
var external_velocity: Vector3 = Vector3.ZERO

# Private variables (prefixed with _)
var _hard_landing_timer: float = 0.0
# #2852 M6: apex of the current airborne stretch; landing stun triggers on
# fall HEIGHT (apex - landing), not on scene-driven cooldowns.
var _fall_apex_y: float = 0.0
# #2852 M11: rotating-platform follow state (translation comes free from the
# engine's platform velocity on kinematic colliders).
var _platform: CollisionObject3D = null
var _platform_last_quat := Quaternion.IDENTITY
var _locomotion_settings: DclLocomotionSettings = null
var _jump_buffer: float = 0.0
var _accel_weight: float = 0.0
var _glide_timer: float = 0.0
var _time_since_last_jump: float = 1000.0
var _time_since_glide_end: float = 1000.0
var _air_jump_delay_timer: float = 0.0
var _air_jump_direction: Vector3 = Vector3.ZERO
var _ground_distance: float = INF
# True while AvatarRaycast is aimed at the crosshair (mobile, non-cinematic) —
# gates the restore-to-default so it doesn't write constants every tick.
var _avatar_raycast_crosshair_active: bool = false
# Smoothed crosshair screen position (see _update_crosshair_screen_position).
var _crosshair_screen_pos := Vector2.ZERO
# Crosshair opacity (1p -> 3p fades in so it never covers the avatar's head).
var _crosshair_alpha: float = 1.0
var _crosshair_pos_initialized := false
# Timed-lerp transition state: position captured on the first frame of a mode
# swap, elapsed time, and whether a transition is playing.
var _crosshair_transition_from := Vector2.ZERO
var _crosshair_transition_clock: float = 0.0
var _crosshair_transition_active := false
var _crosshair_prev_mode: Global.CameraMode = Global.CameraMode.THIRD_PERSON
# #b11: typed Array[RID] avoids per-element dynamic cast when passed to
# PhysicsRayQueryParameters3D.exclude every physics frame.
var _raycast_exclude: Array[RID] = []

# --- Pinch-to-zoom (mobile, issue #2709) -------------------------------------
# Team decision: two fixed camera positions (1p/3p, same as prod) — no continuous
# zoom curve. The pinch accumulates the finger-spread change and swaps mode once
# it passes PinchGestureHelpers.MODE_TOGGLE_SPREAD.
var _pinch_accumulated_delta: float = 0.0
# Camera mode when the active pinch started (analytics reports the net direction).
var _pinch_start_mode: Global.CameraMode = Global.CameraMode.THIRD_PERSON
# The active camera-mode tween, killed before a new one so they never fight.
var _camera_mode_tween: Tween = null
# Step-up arming: after a TALL rise (> STEP_TALL_RISE) or two consecutive
# rises with no rest between (steep ramps chain small rises) the step-up
# disarms until the capsule rests. A ramp face never rests, a staircase
# tread always does, and single terrain bumps rise without arming
# side-effects at all.
var _step_armed := true
var _step_pending := false

@onready var mount_camera := $Mount
@onready var camera: DclCamera3D = $Mount/CameraArm/Camera3D
@onready var camera_collision_clamp: CameraCollisionClamp = $Mount/CameraCollisionClamp
@onready var avatar_raycast: RayCast3D = $Mount/CameraArm/Camera3D/AvatarRaycast
@onready var outline_system: OutlineSystem = $Mount/CameraArm/Camera3D/OutlineSystem
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar
@onready var stuck_detector := $StuckDetector
# Margin-less clone for step-up motion tests (skin width would eat the step band).
@onready var _step_test_shape: CapsuleShape3D = _make_step_test_shape()


func to_xz(pos: Vector3) -> Vector2:
	return Vector2(pos.x, pos.z)


func _on_camera_mode_area_detector_block_camera_mode(forced_mode):
	if !camera_mode_change_blocked:  # if it's already blocked, we don't store the state again...
		stored_camera_mode_before_block = camera.get_camera_mode() as Global.CameraMode
		camera_mode_change_blocked = true

	set_camera_mode(forced_mode, false)
	Global.set_camera_mode_blocked(true)


func _on_camera_mode_area_detector_unblock_camera_mode():
	camera_mode_change_blocked = false
	Global.set_camera_mode_blocked(false)
	set_camera_mode(stored_camera_mode_before_block, false)


func _on_global_camera_mode_set(mode: Global.CameraMode) -> void:
	if mode != Global.CameraMode.CINEMATIC:
		set_camera_mode(mode)


func set_camera_mode(mode: Global.CameraMode, play_sound: bool = true):
	camera.set_camera_mode(mode)

	if _camera_mode_tween and _camera_mode_tween.is_running():
		_camera_mode_tween.kill()

	if mode == Global.CameraMode.THIRD_PERSON:
		var targets := CameraRigHelpers.rig_targets(true)
		var tween_out = create_tween()
		_camera_mode_tween = tween_out
		tween_out.set_parallel(true)
		(
			tween_out
			. tween_property(
				mount_camera, "spring_length", targets.spring_length, CAMERA_MODE_TWEEN_TIME
			)
			. set_ease(Tween.EASE_IN_OUT)
		)
		# Apply X offset for third person (0 = avatar centered, issue #2709). The
		# offset lives on the collision clamp, which positions the camera below the
		# arm so the spring-arm pivot stays centered on the player capsule and
		# sweeps a sphere to the real (offset) camera position every physics frame.
		(
			tween_out
			. tween_property(
				camera_collision_clamp,
				"lateral_offset",
				targets.camera_offset_x,
				CAMERA_MODE_TWEEN_TIME
			)
			. set_ease(Tween.EASE_IN_OUT)
		)
		avatar.set_hidden(false)
		avatar.set_rotation(Vector3(0, rotation.y, 0))
		if play_sound:
			UiSounds.play_sound("ui_fade_out", false, CAMERA_TRANSITION_SOUND_DB)
	elif mode == Global.CameraMode.FIRST_PERSON:
		var targets := CameraRigHelpers.rig_targets(false)
		var tween_in = create_tween()
		_camera_mode_tween = tween_in
		tween_in.set_parallel(true)
		(
			tween_in
			. tween_property(
				mount_camera, "spring_length", targets.spring_length, CAMERA_MODE_TWEEN_TIME
			)
			. set_ease(Tween.EASE_IN_OUT)
		)
		# Remove X offset for centered view in first person
		(
			tween_in
			. tween_property(
				camera_collision_clamp,
				"lateral_offset",
				targets.camera_offset_x,
				CAMERA_MODE_TWEEN_TIME
			)
			. set_ease(Tween.EASE_IN_OUT)
		)
		if camera.current:
			avatar.set_hidden(true)
		if play_sound:
			UiSounds.play_sound("ui_fade_in", false, CAMERA_TRANSITION_SOUND_DB)


func update_avatar_movement_state(vel: float):
	# #2856: gait bools mirror the input-mode kind (idle when no move input);
	# movement_speed feeds the continuous blend in avatar.gd.
	avatar.walk = movement_kind == 1 and has_move_input
	avatar.jog = movement_kind == 2 and has_move_input
	avatar.run = movement_kind == 3 and has_move_input
	avatar.movement_speed = vel


func _ready():
	if not Global.is_mobile():
		add_child(PlayerDesktopInput.new(self))

	Global.camera_mode_set.connect(_on_global_camera_mode_set)

	# Setup the outline system with the main camera
	if outline_system:
		outline_system.setup(camera)

	camera.current = true

	set_camera_mode(Global.CameraMode.THIRD_PERSON, false)  # Don't play sound on initial setup
	avatar.is_local_player = true

	# Unity's slopeLimit is exclusive (a 46° ramp does not climb); Godot's
	# floor_max_angle is inclusive, so apply a hair under.
	floor_max_angle = deg_to_rad(SLOPE_LIMIT_DEG - 0.01)

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
	# A masked (upper-body) emote only plays while its owning scene is the one the
	# player is standing in: crossing out parks it, crossing back in resumes it.
	# Local player only — a remote avatar's emote lifetime is driven by its own client.
	Global.scene_runner.on_change_scene_id.connect(avatar.on_current_scene_changed)
	Global.scene_runner.locomotion_settings_changed.connect(_on_locomotion_settings_changed)
	_on_scene_changed(Global.scene_runner.get_current_parcel_scene_id())

	# Reset the pinch zoom on a deliberate transition only (see _on_loading_finished).
	Global.loading_finished.connect(_on_loading_finished)

	# Cache RIDs to exclude from ground-distance raycasts (player body itself +
	# avatar subtree colliders, including the TriggerDetector which would
	# otherwise make the ray report ~0m at all times).
	_build_raycast_exclude()
	# The scene pointer raycast excludes the same set: with the third-person avatar
	# centered on screen (issue #2709), the ray from the camera would hit its back.
	Global.scene_runner.set_pointer_raycast_exclude(_raycast_exclude)

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


# on_change_scene_id fires every time the current-parcel scene id changes, which
# on Genesis City includes simply walking across a parcel boundary — not a signal
# for "the user deliberately went somewhere". loading_finished only fires behind a
# loading screen (Discover jump, teleport, realm change), so it's the right place
# to reset the pinch zoom back to the default third-person view. Mobile-only: the
# pinch input is mobile-only, and desktop users can sit in first person by choice.
func _on_loading_finished() -> void:
	if Global.is_mobile():
		_reset_zoom_to_default()


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


func clamp_camera_rotation():
	# Maybe mobile wants a requires values
	if camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90))
	elif camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(35))


## Apply a relative look delta (screen pixels) to the camera. Shared by the mobile
## touch handler and by scene-UI swipe handoff, so both rotate the camera identically.
func apply_look_delta(relative: Vector2) -> void:
	rotate_y(deg_to_rad(-relative.x) * MobileCameraInput.HORIZONTAL_SENS)
	mount_camera.rotate_x(deg_to_rad(-relative.y) * MobileCameraInput.VERTICAL_SENS)
	clamp_camera_rotation()


# Pure grounded-flag resolution, extracted for regression testing (#2732).
# Grace is suppressed while jump is held AND during the jump cooldown — the
# cooldown check is what keeps tap-jumps (released before the next tick reads
# jump_held == false) from lingering grounded.
static func resolve_is_grounded(
	on_floor: bool, fall_elapsed: float, jump_held: bool, since_last_jump: float
) -> bool:
	return (
		on_floor
		or (
			fall_elapsed < GROUNDED_GRACE_WINDOW
			and not jump_held
			and since_last_jump >= JUMP_COOLDOWN
		)
	)


## Pinch-to-zoom (mobile). MobileCameraInput calls begin → apply(*) → end around a
## two-finger pinch. Two fixed positions (team decision, issue #2709): the gesture
## accumulates the finger-spread change and swaps 1p↔3p once it passes the toggle
## threshold; end() reports the analytics event.
func begin_pinch_zoom() -> void:
	_pinch_accumulated_delta = 0.0
	_pinch_start_mode = camera.get_camera_mode() as Global.CameraMode


## `pixel_delta` is the change in distance between the two fingers this frame.
## Roblox-style mapping: fingers opening zoom IN (toward first person), fingers
## closing zoom OUT (toward third person). After a toggle the accumulator
## resets, so swapping back needs a fresh threshold's worth of spread in the
## opposite direction.
func apply_pinch_zoom(pixel_delta: float) -> void:
	if camera_mode_change_blocked:
		return
	_pinch_accumulated_delta += pixel_delta
	var mode: Global.CameraMode = camera.get_camera_mode() as Global.CameraMode
	match PinchGestureHelpers.toggle_direction(_pinch_accumulated_delta):
		1:
			if mode == Global.CameraMode.THIRD_PERSON:
				Global.set_camera_mode(Global.CameraMode.FIRST_PERSON)
			_pinch_accumulated_delta = 0.0
		-1:
			if mode == Global.CameraMode.FIRST_PERSON:
				Global.set_camera_mode(Global.CameraMode.THIRD_PERSON)
			_pinch_accumulated_delta = 0.0


func end_pinch_zoom() -> void:
	if camera_mode_change_blocked:
		return
	var mode: Global.CameraMode = camera.get_camera_mode() as Global.CameraMode
	if mode == _pinch_start_mode:
		return  # no net change — nothing to report
	var zoom_direction: String = "zoom_out" if mode == Global.CameraMode.THIRD_PERSON else "zoom_in"
	if Global.metrics:
		Global.metrics.track_click_button(
			"ZOOM", "IN_WORLD", JSON.stringify({"zoom_direction": zoom_direction})
		)


# Reset the camera back to the default third-person view. Called on every
# deliberate transition on mobile (see _on_loading_finished) — so a pinch into
# first person returns to third person after the next loading screen. Skipped
# while a scene forces the camera mode.
func _reset_zoom_to_default() -> void:
	if camera_mode_change_blocked:
		return
	if camera.get_camera_mode() != Global.CameraMode.THIRD_PERSON:
		Global.set_camera_mode(Global.CameraMode.THIRD_PERSON)


func _physics_process(dt: float) -> void:
	# Sample scene-driven physics before gravity — force.y feeds effective_gravity below.
	var scene_external_force: Vector3 = Global.scene_runner.get_active_external_force()
	var scene_pending_impulses: PackedVector3Array = Global.scene_runner.consume_pending_impulses()
	var external_acceleration: Vector3 = scene_external_force / CHARACTER_MASS

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
	# Single grounded signal for the whole jump/gravity chain: on_floor OR
	# walkable support (the margin-cloud rest has no floor contact). Mixing
	# the two signals across branches left landing unreachable mid-cloud
	# (velocity.y accumulated, jump_count never reset, glider never closed).
	var supported := on_floor or _has_walkable_support()

	# #1557: is_on_floor() drops before the capsule visually leaves an edge
	# (rounded bottom + speculative margin), which would burn the coyote
	# window early — Unity's CCT IsGrounded holds through the skin width.
	# Start the fall timer only when walkable support is truly gone.
	if !supported:
		time_falling += dt
	else:
		time_falling = 0.0

	# #1557: coyote window in seconds (B1) — the ground jump stays reachable this
	# long after leaving the floor, and the glide gate must not eat the press.
	# velocity.y guard: the window only opens when walking off a ledge, never
	# on the way up (a jump-pad launch must not become a cancellable "ground").
	var in_coyote := not supported and time_falling <= COYOTE_WINDOW and velocity.y <= 0.0

	# Air-jump hover phase: freeze gravity, then fire impulse + horizontal dash
	# when the timer expires. Leaves avatar.rise/fall untouched on purpose —
	# flipping them mid-hover would trip Jump_Fall → Jump_End via nfall and
	# strand the state machine away from Double_Jump_Rise when jump_count flips.
	if _air_jump_delay_timer > 0.0:
		_air_jump_delay_timer -= dt
		velocity.y = 0.0
		if _air_jump_delay_timer <= 0.0:
			velocity.y = sqrt(2.0 * AIR_JUMP_HEIGHT * gravity * GRAVITY_ASCENT_FACTOR)
			var horiz_dir: Vector3 = Vector3(_air_jump_direction.x, 0.0, _air_jump_direction.z)
			if horiz_dir.length_squared() > 0.0001:
				horiz_dir = horiz_dir.normalized()
				# #1557: max(8, current horizontal speed) (ApplyJump.cs).
				var impulse := maxf(
					AIR_JUMP_DIRECTION_IMPULSE, Vector2(velocity.x, velocity.z).length()
				)
				velocity.x = horiz_dir.x * impulse
				velocity.z = horiz_dir.z * impulse
			jump_count += 1
			_time_since_last_jump = 0.0
			avatar.rise = true
			avatar.fall = false
	elif not supported and not in_coyote:
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
		# Scene force.y reduces effective gravity, so an upward wind cancels
		# fall instead of stacking on velocity.y.
		velocity.y -= (_current_gravity() - external_acceleration.y) * dt

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
		# #1557: fires on the floor and inside the coyote window (B1). Exact port
		# of ApplyJump.GetJumpHeight: run height only while sprinting, lerped by
		# current horizontal speed over run speed; v0 uses the ascent gravity.
		var h_speed := Vector2(velocity.x, velocity.z).length()
		var max_jump_height := (
			run_jump_height if Input.is_action_pressed("ia_sprint") else jump_height
		)
		var effective_jump_height := lerpf(
			jump_height, max_jump_height, clampf(h_speed / run_speed, 0.0, 1.0)
		)
		velocity.y = sqrt(2.0 * effective_jump_height * gravity * GRAVITY_ASCENT_FACTOR)
		jump_count = 1
		_jump_buffer = 0.0
		_time_since_last_jump = 0.0
		avatar.land = false
		avatar.rise = true
		avatar.fall = false
	elif supported:
		if not avatar.land:
			avatar.land = true
			# #2852 M6: fall-height trigger replaces the scene-driven cooldown
			# (Unity StunCharacterSystem: JumpHeightStun 8m, LongFallStunTime 0.75).
			if _fall_apex_y - global_position.y > HARD_LANDING_FALL_HEIGHT:
				_hard_landing_timer = HARD_LANDING_STUN_TIME
		_fall_apex_y = global_position.y

		velocity.y = 0
		avatar.rise = false
		avatar.fall = false
		# Landing resets the air-jump budget and force-closes the glider.
		jump_count = 0
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			glide_state = GLIDE_CLOSING
			_glide_timer = GLIDE_CLOSING_TIME
	else:
		# Coyote fall without a buffered jump: gravity applies, no landing state,
		# glide gate stays closed for the whole window.
		# #2852 M3 — ApplyGravity.cs: on a steep slope (>46deg) gravity tilts
		# along the slope instead of pulling straight down.
		var steep := _steep_slide_dir()
		if steep != Vector3.ZERO:
			velocity += steep * (_current_gravity() - external_acceleration.y) * dt
		else:
			velocity.y -= (_current_gravity() - external_acceleration.y) * dt
	# #2852 M6: track the airborne apex (reset while gliding — a gentle glide
	# down from height is not a hard landing).
	if not supported:
		if glide_state == GLIDE_OPENING or glide_state == GLIDE_GLIDING:
			_fall_apex_y = global_position.y
		else:
			_fall_apex_y = maxf(_fall_apex_y, global_position.y)
	# #2852 M4 — edge slip: off-axis ground contact with no ground straight
	# below tilts gravity toward the edge (capsule slips off).
	if supported:
		var edge_dir := _edge_slip_gravity_dir()
		if edge_dir != Vector3.ZERO:
			velocity += edge_dir * _current_gravity() * dt

	# #2850: Unity port (ApplyCharacterMovementVelocity.cs). Weight ramps over
	# 0.5s while input is held; the accel pair follows the settings curve
	# (keys 0/0.1→0, 0.9/1→1: plateau at min, then ramp). Velocity target uses
	# the RAW input direction — the smoothed current_direction is only for facing.
	has_move_input = direction != Vector3.ZERO
	_accel_weight = move_toward(
		_accel_weight, 1.0 if has_move_input else 0.0, dt / ACCELERATION_TIME
	)
	var curve_t := clampf(inverse_lerp(0.1, 0.9, _accel_weight), 0.0, 1.0)
	var accel := (
		lerpf(GROUND_ACCEL, GROUND_ACCEL_MAX, curve_t)
		if on_floor
		else lerpf(AIR_ACCEL, AIR_ACCEL_MAX, curve_t)
	)

	camera.set_target_fov(DEFAULT_CAMERA_FOV)
	if has_move_input:
		var wants_walk := Input.is_action_pressed("ia_walk")
		var wants_sprint := Input.is_action_pressed("ia_sprint")

		# #2856: gait kind from the input MODE (walk jog run), not measured
		# speed — the anim blend normalizes by the kind's speed cap (Unity).
		if wants_sprint and not run_disabled:
			movement_kind = 3
		elif wants_walk and not walk_disabled:
			movement_kind = 1
		elif Global.is_mobile() and jog_disabled and not walk_disabled:
			movement_kind = 1
		else:
			movement_kind = 2

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
			movement_kind = 1
		# else: effective_speed remains 0, no movement allowed

		# #2852 M2: slope speed modifier — the curve multiplies the target
		# speed by the signed uphill/downhill angle (Unity multiplies its
		# speedLimit the same way).
		if is_on_floor():
			effective_speed *= _slope_speed_modifier(direction)

		# ADAD sign correction: reversing an axis flips sign, keeping momentum.
		var target_x := direction.x * effective_speed
		var target_z := direction.z * effective_speed
		if signf(target_x) != 0.0 and signf(target_x) != signf(velocity.x):
			velocity.x = -velocity.x
		if signf(target_z) != 0.0 and signf(target_z) != signf(velocity.z):
			velocity.z = -velocity.z
		# Ground and air accel pairs; air is MoveTowards instead of the old
		# direct assignment (reduced air control).
		velocity.x = move_toward(velocity.x, target_x, accel * dt)
		velocity.z = move_toward(velocity.z, target_z, accel * dt)

		avatar.look_at(current_direction.normalized() + position)
		avatar.rotation.x = 0.0
		avatar.rotation.z = 0.0
	else:
		if on_floor:
			# StopTimeSec=0: grounded stop is INSTANT in Unity (degenerate
			# SmoothDamp). B4 dies with it — instant is tick-rate independent.
			velocity.x = 0.0
			velocity.z = 0.0
		else:
			# Air with no input drifts toward 0 at the air accel rate.
			velocity.x = move_toward(velocity.x, 0.0, accel * dt)
			velocity.z = move_toward(velocity.z, 0.0, accel * dt)

	# #2850: quadratic horizontal air drag, coefficient 0.2 (live Unity value).
	if not on_floor:
		var h_vel := Vector2(velocity.x, velocity.z)
		var h_mag := h_vel.length()
		if h_mag > 0.0:
			h_vel -= h_vel.normalized() * minf(AIR_DRAG * h_mag * h_mag * dt, h_mag)
			velocity.x = h_vel.x
			velocity.z = h_vel.y

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
	# Debounced ungrounding: see resolve_is_grounded. The jump-pad override
	# below (combined_vy > 0.3) runs after this and still wins.
	avatar.is_grounded = resolve_is_grounded(
		on_floor, time_falling, Input.is_action_pressed("ia_jump"), _time_since_last_jump
	)

	_apply_scene_physics(dt, external_acceleration, scene_pending_impulses, on_floor)

	# Re-derive rise/fall from the combined vertical velocity: the lift from
	# external_velocity isn't visible in velocity.y alone (we undo the Y mix
	# below, so velocity.y reads near-zero mid-bounce).
	if external_velocity.length_squared() > 0.01:
		var combined_vy: float = velocity.y + external_velocity.y
		var free_flight: bool = glide_state == GLIDE_CLOSED or glide_state == GLIDE_CLOSING
		if free_flight:
			if combined_vy > 0.3:
				avatar.rise = true
				avatar.fall = false
				avatar.land = false
				avatar.is_grounded = false
			elif combined_vy < -0.3:
				avatar.fall = true
				avatar.rise = false

	# Snapshot locomotion XZ so we can restore them after the move. Otherwise
	# `move_toward` on the next no-input tick would decel from velocity-with-
	# external stacked, and the external add would compound indefinitely.
	var locomotion_x: float = velocity.x
	var locomotion_z: float = velocity.z
	var external_y_for_move: float = external_velocity.y
	velocity.x += external_velocity.x
	velocity.y += external_y_for_move
	velocity.z += external_velocity.z

	# #2852 M5 — wall slide (ApplyWallSlide.cs): facing into a wall brakes
	# movement toward zero; parallel is free. Unity capsulecasts ahead to find
	# the wall; we already have the contact.
	if is_on_wall() and supported:
		var wall_n := get_wall_normal()
		wall_n.y = 0.0
		var look := Vector3(current_direction.x, 0.0, current_direction.z)
		if wall_n.length_squared() > 0.01 and look.length_squared() > 0.01:
			var wall_dot := absf(look.normalized().dot(wall_n.normalized()))
			var wall_mult := lerpf(1.0, WALL_SLIDE_MIN_MULT, wall_dot)
			velocity.x *= wall_mult
			velocity.z *= wall_mult
	last_position = global_position
	# #2753: downslope stick — ApplySlopeModifier picks by input kind (run when
	# sprinting), not by measured speed. Assigned BEFORE the predictive step-up:
	# a rise zeroes the snap for that frame's move so the lower floor within
	# snap reach cannot re-glue the capsule mid-step. Keeping the snap on every
	# other frame is also what makes is_on_floor() reliable for the re-arm.
	floor_snap_length = (
		DOWNSLOPE_STICK_RUN if Input.is_action_pressed("ia_sprint") else DOWNSLOPE_STICK_JOG
	)
	_try_step_up_predictive(Vector3(locomotion_x, 0.0, locomotion_z), dt)
	var vy_before_move := velocity.y
	# Captured after the predictive pass: a committed rise changes y, and
	# comparing against the pre-rise position would undo legitimate steps.
	var y_before_move := global_position.y
	move_and_slide()
	# PhysX slope limiter, positional: if the move itself gained height with
	# no upward velocity (gravity only), the slope slide carried the capsule —
	# the recovery ratchet climbs steep ramps positionally. Allowed only when
	# a walkable contact exists; jumps/pads set vy > 0 before the move and
	# never match. Without this, a 46deg ramp is climbable (Unity blocks it).
	if global_position.y > y_before_move + 0.0005 and vy_before_move <= 0.0:
		var walkable_contact := false
		var any_contact := false
		for i in get_slide_collision_count():
			any_contact = true
			if get_slide_collision(i).get_normal().y >= WALKABLE_NORMAL_Y:
				walkable_contact = true
		if any_contact and not walkable_contact:
			global_position.y = y_before_move
			velocity.y = minf(velocity.y, 0.0)
	var moved_xz := (to_xz(global_position) - to_xz(last_position)).length()
	_try_step_up(Vector3(locomotion_x, 0.0, locomotion_z), moved_xz)
	# Step-up re-arming: the capsule re-arms when it stands on WALKABLE
	# support — a staircase tread, a step top, flat ground (even resting on
	# Jolt's speculative margin cloud, where is_on_floor() never fires, or
	# pressed against the next riser, which is fine: the tread holds you).
	# On a steep ramp the support under the feet IS the ramp (un-walkable
	# normal), and mid-air there is no support at all, so ramp faces never
	# re-arm: no band-chaining while pressing, sliding, or hopping.
	if (not _step_armed or _step_pending) and _has_walkable_support():
		_step_armed = true
		_step_pending = false
	_update_platform_follow(supported)
	position.y = max(position.y, 0)
	avatar.global_position = global_position

	# Restore locomotion-only XZ; external_velocity carries its own state and is
	# re-added next frame.
	velocity.x = locomotion_x
	velocity.z = locomotion_z

	# Restore velocity.y unless a floor/ceiling collision already zeroed it.
	if not is_on_floor() and not is_on_ceiling():
		velocity.y -= external_y_for_move

	_update_crosshair_screen_position(dt)
	_update_avatar_raycast_to_crosshair()


# Issue #2709: crosshair target in screen pixels — a pure function of the
# spring-arm length (fixed anchors, quadratic ease-in across the mode swap).
# No live projection, no prediction: it can never swing or jump.
# Live third-person crosshair target (settled-camera phase): CROSSHAIR_TOP_GAP_PX
# above the avatar's on-screen top edge, at its side edge's x, clamped inside
# the screen margin. Falls back to the placeholder anchor when the head is
# behind the camera.
func _compute_live_crosshair_target(viewport_size: Vector2) -> Vector2:
	var top_world := global_position + Vector3(0, AVATAR_TOP_HEIGHT, 0)
	if camera.is_position_behind(top_world):
		return CROSSHAIR_PLACEHOLDER_ANCHOR * viewport_size
	var edge_world := top_world + camera.global_transform.basis.x * AVATAR_HALF_WIDTH
	var top_px := camera.unproject_position(top_world)
	var edge_px := camera.unproject_position(edge_world)
	var tracked := Vector2(edge_px.x, top_px.y - CROSSHAIR_TOP_GAP_PX)
	var margin := Vector2(CROSSHAIR_SCREEN_MARGIN, CROSSHAIR_SCREEN_MARGIN)
	return tracked.clamp(margin, viewport_size - margin)


# Smoothed crosshair position (single source for the HUD label, the scene
# interaction raycast and AvatarRaycast, so the three never desync).
func get_crosshair_screen_position() -> Vector2:
	return _crosshair_screen_pos


# Crosshair opacity for the HUD label (see _update_crosshair_screen_position).
func get_crosshair_alpha() -> float:
	return _crosshair_alpha


func _update_crosshair_screen_position(dt: float) -> void:
	var viewport_size := get_viewport().get_visible_rect().size
	var active := Global.is_mobile() and not Global.scene_runner.raycast_use_cursor_position
	if not active:
		# Desktop / cinematic own the crosshair; park it at center so re-entering
		# mobile gameplay starts from center, never from a stale point.
		_crosshair_screen_pos = viewport_size * 0.5
		_crosshair_alpha = 1.0
		_crosshair_pos_initialized = false
		_crosshair_transition_active = false
		_crosshair_prev_mode = camera.get_camera_mode() as Global.CameraMode
		return

	# Timed-lerp model (device QA: zero overshoot). A mode swap captures the
	# current position and plays deterministic smoothstep lerps — no spring
	# chase, no prediction. Timeline is in units of the camera tween (T=0.25s):
	#   3p -> 1p: glide to center over [0, T].
	#   1p -> 3p: hold until 1.25T (the collision clamp extends the real camera
	#   ~0.15s past the arm tween, so the live point is only stable by then),
	#   then glide to the LIVE point over [1.25T, 1.75T].
	_crosshair_transition_clock += dt
	var mode: Global.CameraMode = camera.get_camera_mode() as Global.CameraMode
	if mode != _crosshair_prev_mode:
		_crosshair_prev_mode = mode
		if _crosshair_pos_initialized:
			_crosshair_transition_from = _crosshair_screen_pos
			_crosshair_transition_clock = 0.0
			_crosshair_transition_active = true

	if not _crosshair_pos_initialized:
		_crosshair_pos_initialized = true
		if mode == Global.CameraMode.FIRST_PERSON:
			_crosshair_screen_pos = viewport_size * 0.5
		else:
			_crosshair_screen_pos = _compute_live_crosshair_target(viewport_size)
		return

	if _crosshair_transition_active:
		var t := _crosshair_transition_clock / CAMERA_MODE_TWEEN_TIME
		if mode == Global.CameraMode.FIRST_PERSON:
			var w := smoothstep(0.0, 1.0, minf(t, 1.0))
			_crosshair_screen_pos = _crosshair_transition_from.lerp(viewport_size * 0.5, w)
			if w >= 1.0:
				_crosshair_transition_active = false
		else:
			# 1p -> 3p: no position transition at all (QA) — the crosshair tracks
			# the live point the whole time but stays invisible until the camera
			# settles, then simply FADES IN at the final spot over [1.5T, 2.0T].
			_crosshair_screen_pos = _compute_live_crosshair_target(viewport_size)
			_crosshair_alpha = smoothstep(0.0, 1.0, minf((t - 1.5) / 0.5, 1.0))
			if t >= 2.0:
				_crosshair_transition_active = false
	elif mode == Global.CameraMode.FIRST_PERSON:
		_crosshair_screen_pos = viewport_size * 0.5
		_crosshair_alpha = 1.0
	else:
		_crosshair_screen_pos = _compute_live_crosshair_target(viewport_size)
		_crosshair_alpha = 1.0


# Issue #2709: aim the avatar outline/view-profile raycast at the crosshair.
# The ray keeps its origin at the camera (own avatar is excluded via its removed
# ClickArea) and only the direction changes. Mobile only; desktop/cinematic keep
# the tscn default. Writes happen only on state change, not every physics tick.
func _update_avatar_raycast_to_crosshair() -> void:
	var active := Global.is_mobile() and not Global.scene_runner.raycast_use_cursor_position
	if not active:
		if _avatar_raycast_crosshair_active:
			_avatar_raycast_crosshair_active = false
			avatar_raycast.position = Vector3.ZERO
			avatar_raycast.target_position = AVATAR_RAYCAST_DEFAULT_TARGET
		return
	_avatar_raycast_crosshair_active = true
	var dir := camera.project_ray_normal(_crosshair_screen_pos)
	avatar_raycast.position = Vector3.ZERO
	avatar_raycast.target_position = avatar_raycast.to_local(
		camera.global_position + dir * AVATAR_RAYCAST_DEFAULT_TARGET.length()
	)


# #1557: asymmetric gravity (ApplyGravity.cs) — hold window FIRST (applies on
# ascent AND early descent: 10×0.5=5), then the ascent factor (hold+rise = 20).
func _current_gravity() -> float:
	var g := gravity
	if Input.is_action_pressed("ia_jump") and _time_since_last_jump < LONG_JUMP_TIME:
		g *= LONG_JUMP_GRAVITY_SCALE
	if velocity.y > 0.0:
		g *= GRAVITY_ASCENT_FACTOR
	return g


# #2753: custom step offset — CharacterBody3D has no built-in (M1). The
# step-up is a PhysX CCT relocation test: move the margin-less capsule clone
# up (the lift cap is the climbable band), forward past the face, and down;
# point rays then read the landing, measured from the resting contact.
# `intent` is the pre-slide locomotion velocity (slide zeroes it on the wall).
func _make_step_test_shape() -> CapsuleShape3D:
	var s2: CapsuleShape3D = %CollisionShape3D_Body.shape.duplicate()
	s2.margin = 0.0
	return s2


func _steep_slide_dir() -> Vector3:
	# #2852 M3: downhill tangent of a steeper-than-walkable slide contact
	# (ZERO when none). n.y > 0.05 excludes vertical walls.
	for i in get_slide_collision_count():
		var n := get_slide_collision(i).get_normal()
		if n.y < WALKABLE_NORMAL_Y and n.y > 0.05:
			var g_vec := Vector3(0.0, -1.0, 0.0)
			return (g_vec - n * g_vec.dot(n)).normalized()
	return Vector3.ZERO


func _edge_slip_gravity_dir() -> Vector3:
	# #2852 M4 — ApplyEdgeSlip.cs: probe down from the capsule axis; a ground
	# contact offset past NoSlipDistance (edge!) tilts gravity toward the
	# contact normal's downhill tangent. Skipped when ground sits directly
	# below within EdgeSlipSafeDistance (0.4) — no slipping on gentle ground.
	if not is_on_floor():
		return Vector3.ZERO
	var space := get_world_3d().direct_space_state
	if space == null:
		return Vector3.ZERO
	var rq := PhysicsRayQueryParameters3D.new()
	var center := global_position + Vector3(0.0, CAPSULE_CENTER_Y, 0.0)
	rq.from = center
	rq.to = center + Vector3(0.0, -CAPSULE_CENTER_Y * 1.2, 0.0)
	rq.collision_mask = collision_mask
	rq.exclude = _raycast_exclude
	var hit := space.intersect_ray(rq)
	if hit.is_empty():
		return Vector3.ZERO
	var rel: Vector3 = hit.position - center
	rel.y = 0.0
	if rel.length() <= EDGE_NO_SLIP_DIST:
		return Vector3.ZERO
	var straight := PhysicsRayQueryParameters3D.new()
	straight.from = global_position
	straight.to = global_position + Vector3(0.0, -0.4, 0.0)
	straight.collision_mask = collision_mask
	straight.exclude = _raycast_exclude
	if not space.intersect_ray(straight).is_empty():
		return Vector3.ZERO
	var n: Vector3 = hit.normal
	var g_vec := Vector3(0.0, -1.0, 0.0)
	return (g_vec - n * g_vec.dot(n)).normalized()


func _update_platform_follow(supported: bool) -> void:
	# #2852 M11 — CharacterPlatformSystem.cs: stay attached to the platform
	# underfoot; rotation follow (translation is engine platform velocity).
	# The support ray finds the collider even on the margin-cloud rest (slide
	# contacts never register there).
	var current: CollisionObject3D = null
	if supported:
		var space := get_world_3d().direct_space_state
		if space != null:
			var rq := PhysicsRayQueryParameters3D.new()
			rq.from = global_position + Vector3(0.0, 0.05, 0.0)
			rq.to = global_position + Vector3(0.0, -0.2, 0.0)
			rq.collision_mask = collision_mask
			rq.exclude = _raycast_exclude
			var hit := space.intersect_ray(rq)
			if not hit.is_empty() and hit.normal.y >= WALKABLE_NORMAL_Y:
				current = hit.collider as CollisionObject3D
	if current != _platform:
		_platform = current
		_platform_last_quat = (
			current.global_transform.basis.get_rotation_quaternion()
			if current
			else Quaternion.IDENTITY
		)
		return
	if _platform == null:
		return
	var quat_now: Quaternion = _platform.global_transform.basis.get_rotation_quaternion()
	var delta := quat_now * _platform_last_quat.inverse()
	_platform_last_quat = quat_now
	if delta.is_equal_approx(Quaternion.IDENTITY):
		return
	# Rotate the player around the platform origin, and the facing with it.
	var origin: Vector3 = _platform.global_transform.origin
	global_position = origin + delta * (global_position - origin)
	rotation.y += delta.get_euler().y


func _slope_speed_modifier(input_dir: Vector3) -> float:
	# #2852 M2 — ApplyCharacterMovementVelocity.cs:18-21. slopeForward is the
	# input direction projected onto the slope plane; the signed angle from
	# the input direction to it (around input × up) is positive uphill.
	var look := Vector3(input_dir.x, 0.0, input_dir.z)
	if look.length_squared() < 0.01:
		return 1.0
	look = look.normalized()
	var n := get_floor_normal()
	var slope_forward := n.cross(look.cross(n))
	if slope_forward.length_squared() < 0.01:
		return 1.0  # flat ground — no slope direction
	slope_forward = slope_forward.normalized()
	var angle := rad_to_deg(look.signed_angle_to(slope_forward, look.cross(Vector3.UP)))
	var a := clampf(angle, -SLOPE_MOD_MAX_DEG, SLOPE_MOD_MAX_DEG)
	if a < 0.0:
		return lerpf(1.0, SLOPE_MOD_DOWNHILL, -a / SLOPE_MOD_MAX_DEG)
	return lerpf(1.0, SLOPE_MOD_UPHILL, a / SLOPE_MOD_MAX_DEG)


func _has_walkable_support() -> bool:
	# Direct support check for step-up re-arming. is_on_floor() misses rests
	# on Jolt's speculative margin cloud (contacts cancel motion without ever
	# registering floor), so when it reports nothing, confirm with a short
	# ray — rays return normals, cast_motion in this build does not.
	if is_on_floor():
		return true  # floor contacts are within floor_max_angle by definition
	if global_position.y <= REALM_FLOOR_EPS:
		return true  # clamp-held realm floor, see REALM_FLOOR_EPS
	var space := get_world_3d().direct_space_state
	if space == null:
		return false
	var rq := PhysicsRayQueryParameters3D.new()
	rq.from = global_position + Vector3(0.0, 0.05, 0.0)
	rq.to = global_position + Vector3(0.0, -0.1, 0.0)
	rq.collision_mask = collision_mask
	rq.exclude = _raycast_exclude
	var hit := space.intersect_ray(rq)
	return not hit.is_empty() and hit.normal.y >= WALKABLE_NORMAL_Y


# Predictive step (PhysX CCT parity: the step happens inside the move, not
# after a blocked frame). Probe the intended motion before move_and_slide; on
# a step, rise first and let the horizontal motion flow unobstructed.
func _try_step_up_predictive(intent: Vector3, dt: float) -> void:
	var horiz := Vector3(intent.x, 0.0, intent.z)
	if horiz.length_squared() < 0.25:
		return
	# Already walking a climbable slope: the contact ahead is the slope
	# itself, not a riser — stepping here turns a smooth incline into a
	# stuttered staircase. The post-move fallback still covers blocked cases.
	if is_on_floor() and get_floor_normal().y < SLOPE_WALK_NORMAL_Y:
		return
	var space := get_world_3d().direct_space_state
	if space == null:
		return
	var shape_query := PhysicsShapeQueryParameters3D.new()
	shape_query.shape = _step_test_shape
	shape_query.collision_mask = collision_mask
	shape_query.exclude = _raycast_exclude
	shape_query.transform = Transform3D(
		Basis.IDENTITY, global_position + Vector3(0.0, CAPSULE_CENTER_Y, 0.0)
	)
	shape_query.motion = horiz.normalized() * (horiz.length() * dt + 0.05)
	var contact: PackedFloat32Array = space.cast_motion(shape_query)
	if contact[0] >= 1.0:
		return  # nothing in the way this frame
	_step_up(intent)


func _try_step_up(intent: Vector3, moved_xz: float) -> void:
	# Post-move fallback: the predictive pass (before move_and_slide) covers the
	# approach; this catches direction changes mid-contact. Trigger: wall
	# contact OR blocked horizontal motion. The rounded capsule bottom meets a
	# riser with a diagonal normal that classifies as FLOOR for the first
	# frames (< 46°); waiting for is_on_wall() alone adds a visible hitch.
	var expected := Vector3(intent.x, 0, intent.z).length() * get_physics_process_delta_time()
	var blocked := expected > 0.008 and moved_xz < expected * 0.3
	if not is_on_wall() and not blocked:
		return
	_step_up(intent)


func _step_up(intent: Vector3) -> void:
	if not _step_armed:
		return
	var horiz := Vector3(intent.x, 0.0, intent.z)
	var space := get_world_3d().direct_space_state
	if horiz.length_squared() < 0.25 or space == null:
		return
	var dir := horiz.normalized()
	var origin := global_position + Vector3(0.0, CAPSULE_CENTER_Y, 0.0)
	var q := PhysicsShapeQueryParameters3D.new()
	q.shape = _step_test_shape  # margin-less clone: the skin width would eat the band
	q.collision_mask = collision_mask
	q.exclude = _raycast_exclude
	# PhysX CCT relocation test: instead of measuring the obstacle with point
	# probes, move the (margin-less) capsule itself — the test uses the same
	# shape the solver moves, so seam lips, stacked colliders and trimesh
	# edges need no special cases, and the lift cap IS the climbable band
	# (nothing measured, nothing to flake at the boundary).
	# a) face distance (margin-less clone travel + radius): needed to PLACE
	#    the down phase with the pole just past the edge — the capsule's own
	#    down sweep then reads the top exactly, no hemisphere graze.
	q.transform = Transform3D(Basis.IDENTITY, origin)
	q.motion = dir * 0.5
	var low: PackedFloat32Array = space.cast_motion(q)
	if low[1] >= 1.0:
		return  # no face within cast range — nothing to step onto
	var d_face := low[1] * 0.5 + CAPSULE_RADIUS
	# b) up: how far the capsule can be lifted (doubles as the headroom check).
	q.motion = Vector3(0.0, STEP_MAX_HEIGHT, 0.0)
	var up: PackedFloat32Array = space.cast_motion(q)
	var lift := up[0] * STEP_MAX_HEIGHT
	if lift < STEP_MIN_RISE:
		return  # ceiling
	# c) forward at lifted height until the pole sits just past the face: a
	#    riser taller than the lift blocks this — a wall, not a step.
	var over_face := d_face + 0.005
	q.transform = Transform3D(Basis.IDENTITY, origin + Vector3(0.0, lift, 0.0))
	q.motion = dir * over_face
	var fw: PackedFloat32Array = space.cast_motion(q)
	if fw[0] < 1.0:
		return
	# d) down with the pole directly over the edge. Void past the band => a
	#    gap, not a step (the capsule steps UP only).
	var landing := origin + Vector3(0.0, lift, 0.0) + dir * over_face
	q.transform = Transform3D(Basis.IDENTITY, landing)
	q.motion = Vector3(0.0, -(lift + 0.02), 0.0)
	var dn: PackedFloat32Array = space.cast_motion(q)
	if dn[0] >= 1.0 or dn[0] < 0.005:
		# >=1: void past the band — a gap, not a step (the capsule steps UP
		# only). ~0: the down phase starts already touching — squeezed against
		# the face/edge at lifted height, a wall (reading that contact as a
		# landing reports floor_y = lifted height and teleports up).
		return

	var floor_y: float = landing.y - dn[0] * (lift + 0.02) - _step_test_shape.height * 0.5
	if floor_y < global_position.y + STEP_MIN_RISE:
		return  # flat or lower — nothing to step onto
	# Never commit an overlapping relocation (CCT invariant): diagonal corner
	# approaches can thread the lifted forward cast past the block's edge and
	# end with the pole inside the top slab.
	q.motion = Vector3.ZERO
	q.transform = Transform3D(
		Basis.IDENTITY, Vector3(landing.x, floor_y + CAPSULE_CENTER_Y, landing.z)
	)
	if not space.intersect_shape(q, 1).is_empty():
		return
	# The highest surface under the landing footprint decides the rise (the
	# body cast can graze an edge with its hemisphere and misread it). Probe
	# the edge, mid-tread, and a capsule-radius past (diagonal approaches park
	# the pole beside the block); rays read exact tops. A triple miss (a gap
	# between blocks, or the landing slid off the obstacle) bails: keeping the
	# body-cast height there is how walls get climbed — its graze contact
	# always reads just under the band.
	var prq := PhysicsRayQueryParameters3D.new()
	prq.collision_mask = collision_mask
	prq.exclude = _raycast_exclude
	var best_y := -INF
	for dist in [d_face + 0.005, d_face + 0.15, d_face + 0.3]:
		var px: float = global_position.x + dir.x * dist
		var pz: float = global_position.z + dir.z * dist
		prq.from = Vector3(px, global_position.y + lift + 0.1, pz)
		prq.to = Vector3(px, global_position.y - 0.05, pz)
		var phit := space.intersect_ray(prq)
		if not phit.is_empty() and phit.position.y > best_y:
			best_y = phit.position.y
	var measured := best_y > -INF
	if measured:
		floor_y = best_y
	# The band is measured from the real contact the capsule rests on (slide
	# contacts — the CCT measures stepOffset from the contact point). A ray
	# under the axis misses edge rests (the center hangs past the edge) and
	# measuring from the pole smuggles +1cm when the hemisphere rides an edge.
	var support_y := -INF
	var has_contact := false
	for i in get_slide_collision_count():
		var contact := get_slide_collision(i)
		has_contact = true
		if contact.get_normal().y >= WALKABLE_NORMAL_Y:
			support_y = maxf(support_y, contact.get_position().y)
	if support_y == -INF:
		if global_position.y <= REALM_FLOOR_EPS:
			support_y = 0.0  # clamp-held realm floor
		elif has_contact:
			support_y = global_position.y  # wedged/hanging: from the pole
	if (
		not measured  # all probe rays missed — never keep the graze read
		or support_y == -INF  # airborne, no footing at all
		or floor_y > support_y + STEP_MAX_HEIGHT
		or floor_y < global_position.y + STEP_MIN_RISE
	):
		# above the band measured from the resting contact (wall corner/top),
		# or below the feet (the cast contact was a graze and the pole hangs
		# over void/lower ground)
		return
	# The capsule's center rests ~a radius past the face: that surface must be
	# walkable. A beveled curb is past its slope there (flat), a staircase
	# tread is flat, a continuous ramp still reads its slope — that's what
	# stops the climb/slide loop. A ray miss accepts (trimesh tri edges).
	var nrq := PhysicsRayQueryParameters3D.new()
	nrq.collision_mask = collision_mask
	nrq.exclude = _raycast_exclude
	var nx: float = global_position.x + dir.x * (d_face + 0.25)
	var nz: float = global_position.z + dir.z * (d_face + 0.25)
	nrq.from = Vector3(nx, global_position.y + lift + 0.1, nz)
	nrq.to = Vector3(nx, global_position.y - 0.05, nz)
	var nhit := space.intersect_ray(nrq)
	if not nhit.is_empty() and nhit.normal.y < WALKABLE_NORMAL_Y:
		return  # the capsule would rest on an un-walkable slope — a ramp
	# Rise in place — the horizontal motion flows via move_and_slide itself,
	# so there is no blocked frame and no forward teleport pop. Tall rises
	# disarm until the capsule rests: re-triggering on the same ramp face is
	# what stair-climbs steep inclines. Small rises (bumps, treads) don't
	# disarm, but two in a row without a rest in between do — that chain is
	# how a 60-65° ramp climbs in sub-0.2 bands. Snap off for this frame's
	# move: the lower floor is within snap reach and would re-glue the
	# capsule mid-step.
	var rise := floor_y - global_position.y
	if rise > STEP_TALL_RISE or (rise >= STEP_PENDING_RISE and _step_pending):
		_step_armed = false
	elif rise >= STEP_PENDING_RISE:
		_step_pending = true
	floor_snap_length = 0.0
	global_position.y = floor_y + 0.001


# Fold scene-driven force/impulses into external_velocity, then drag and clamp.
# scene_runner only emits these for the current parcel scene.
func _apply_scene_physics(
	dt: float, external_acceleration: Vector3, impulses: PackedVector3Array, on_floor: bool
) -> void:
	# Force XZ accumulates; force Y was already folded into effective_gravity.
	# #2854 M12: an open glider catches airflow — external forces act stronger
	# (Unity ApplyExternalForce.cs: ExternalAcceleration *= GlideWindResponse).
	var wind := GLIDE_WIND_RESPONSE if glide_state == GLIDE_GLIDING else 1.0
	external_velocity.x += external_acceleration.x * wind * dt
	external_velocity.z += external_acceleration.z * wind * dt

	var got_upward_impulse: bool = false
	for impulse in impulses:
		var delta_v: Vector3 = impulse / CHARACTER_MASS
		if delta_v.y > 0.0:
			got_upward_impulse = true
			# Upward impulse on a falling player clears gravity so jump pads launch.
			if velocity.y < 0.0:
				velocity.y = 0.0
		external_velocity += delta_v

	# Treat an upward impulse as ungrounding for the rest of this tick, or the
	# grounded drag + Y-zero below would cancel a jump-pad launch.
	var effective_on_floor: bool = on_floor and not got_upward_impulse

	# Viscous drag: v *= (1 - damping * dt).
	var damping := EXT_ENV_DRAG
	if effective_on_floor:
		damping += EXT_GROUND_FRICTION
	external_velocity *= maxf(0.0, 1.0 - damping * dt)

	# Zero Y when grounded so landings don't micro-bounce.
	if effective_on_floor:
		external_velocity.y = 0.0

	# Snap tiny magnitudes to zero, clamp large ones to MAX_EXTERNAL_VELOCITY.
	var sqr_mag: float = external_velocity.length_squared()
	if sqr_mag < EXT_VELOCITY_EPSILON_SQR:
		external_velocity = Vector3.ZERO
	elif sqr_mag > MAX_EXTERNAL_VELOCITY * MAX_EXTERNAL_VELOCITY:
		external_velocity = external_velocity.normalized() * MAX_EXTERNAL_VELOCITY


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
