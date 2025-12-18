class_name Player
extends DclPlayer

const DEFAULT_CAMERA_FOV = 75.0
const SPRINTING_CAMERA_FOV = 90.0
const THIRD_PERSON_CAMERA = Vector3(0.75, 0, 3)

var camera_mode_change_blocked: bool = false
var stored_camera_mode_before_block: Global.CameraMode

var current_profile_version: int = -1

@onready var mount_camera := $Mount
@onready var camera: DclCamera3D = $Mount/Camera3D
@onready var avatar_raycast: RayCast3D = $Mount/Camera3D/AvatarRaycast
@onready var outline_system: OutlineSystem = $Mount/Camera3D/OutlineSystem
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


func clamp_camera_rotation():
	# Maybe mobile wants a requires values
	if camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90))
	elif camera.get_camera_mode() == Global.CameraMode.THIRD_PERSON:
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(35))


func _physics_process(dt: float) -> void:
	# Call Rust movement processing with focus state
	var has_focus := Global.explorer_has_focus()
	process_movement(dt, has_focus)


func _process(_dt: float) -> void:
	# Update avatar animation state from Rust movement state
	avatar.walk = is_walking
	avatar.jog = is_jogging
	avatar.run = is_running
	avatar.rise = is_rising
	avatar.fall = is_falling
	avatar.land = is_landing

	# Update avatar facing direction
	var current_dir := get_current_direction()
	if current_dir.length() > 0.01:
		avatar.look_at(current_dir.normalized() + position)
		avatar.rotation.x = 0.0
		avatar.rotation.z = 0.0

	# Update camera FOV based on sprint input
	camera.set_target_fov(DEFAULT_CAMERA_FOV)
	if is_sprint_input_active() and current_dir.length() > 0:
		camera.set_target_fov(SPRINTING_CAMERA_FOV)


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


# gdlint:ignore = async-function-name
func teleport_to(target: Vector3):
	# Use the Rust async_move_to for initial teleport
	async_move_to(target)
	await get_tree().physics_frame
	# Check if we got stuck after the physics frame
	check_stuck_after_teleport(target)
