class_name AvatarPreview
extends SubViewportContainer

const MIN_CAMERA_Z = -3.5
const MAX_CAMERA_Z = -1.25

const MIN_CAMERA_Y = 0.8
const MAX_CAMERA_Y = 2.3

const DEFAULT_ROTATION = Vector3(-20, 180, 0)
const BODY_CAMERA_POSITION = Vector3(0, 2.3, -3.0)
const BODY_CAMERA_POSITION_WITH_PLATFORM = Vector3(0, 2.15, -3.3)
const HEAD_CAMERA_POSITION = Vector3(0, 2.2, -1.15)

@export var hide_name: bool = false
@export var show_platform: bool = false
@export var can_move: bool = true
@export var custom_environment: Environment = null
@export var with_light: bool = true

var start_camera_transform
var start_angle
var start_dragging_position
var dirty_is_dragging

@onready var avatar = %Avatar
@onready var camera_3d: Camera3D = %Camera3D
@onready var platform = %Platform
@onready var subviewport: SubViewport = %SubViewport
@onready var world_environment = $SubViewport/WorldEnvironment
@onready var directional_light_3d = $SubViewport/DirectionalLight3D
@onready var outline_system = %OutlineSystem


func get_body_camera_position() -> Vector3:
	return BODY_CAMERA_POSITION_WITH_PLATFORM if show_platform else BODY_CAMERA_POSITION


func _apply_layout() -> void:
	if not is_inside_tree():
		return

	set_anchors_preset(Control.PRESET_FULL_RECT)
	offset_left = 0
	offset_top = 0
	offset_right = 0
	offset_bottom = 0
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	stretch = true


func _apply_properties() -> void:
	if is_instance_valid(avatar):
		avatar.hide_name = hide_name
	if is_instance_valid(platform):
		platform.set_visible(show_platform)

	if is_instance_valid(camera_3d):
		camera_3d.set_position(get_body_camera_position())

	if can_move:
		if not gui_input.is_connected(self._on_gui_input):
			gui_input.connect(self._on_gui_input)
	else:
		if gui_input.is_connected(self._on_gui_input):
			gui_input.disconnect(self._on_gui_input)
	_apply_layout()


func _ready():
	if custom_environment != null:
		world_environment.environment = custom_environment

	directional_light_3d.visible = with_light

	_apply_layout()

	_apply_properties()

	camera_3d.set_rotation_degrees(DEFAULT_ROTATION)

	if outline_system:
		outline_system.setup(camera_3d)

	if Global.standalone:
		Global.player_identity.set_default_profile()
		var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
		avatar.async_update_avatar_from_profile(profile)


func focus_camera_on(type):
	if not is_inside_tree():
		return

	var tween := create_tween()
	if tween == null:
		return

	tween = tween.set_parallel()
	match type:
		Wearables.Categories.HAIR, Wearables.Categories.FACIAL_HAIR, Wearables.Categories.EYEWEAR, Wearables.Categories.TIARA, Wearables.Categories.FACIAL, Wearables.Categories.EYEBROWS, Wearables.Categories.MOUTH, Wearables.Categories.HAT, Wearables.Categories.EARRING, Wearables.Categories.MASK, Wearables.Categories.HELMET, Wearables.Categories.TOP_HEAD, Wearables.Categories.EYES:
			tween.tween_property(camera_3d, "position", HEAD_CAMERA_POSITION, 0.5)
		_:
			tween.tween_property(camera_3d, "position", get_body_camera_position(), 0.5)
	tween.play()


func _on_gui_input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dirty_is_dragging = true
				start_dragging_position = get_global_mouse_position()
				start_camera_transform = camera_3d.transform
				start_angle = avatar.rotation.y
			else:
				dirty_is_dragging = false

		if not event.pressed:
			var dir: float = 0.0
			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				dir = 0.1
			elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				dir = -0.1

			if dir != 0.0:
				camera_3d.transform.origin.z = clampf(
					camera_3d.transform.origin.z + dir, MIN_CAMERA_Z, MAX_CAMERA_Z
				)

	if event is InputEventMouseMotion:
		if dirty_is_dragging:
			var diff = 0.005 * (get_global_mouse_position() - start_dragging_position)
			var changed_transform = Transform3D(start_camera_transform)
			var min_y = (
				MAX_CAMERA_Y
				- (
					((camera_3d.transform.origin.z - MIN_CAMERA_Z) / (MAX_CAMERA_Z - MIN_CAMERA_Z))
					* (MAX_CAMERA_Y - MIN_CAMERA_Y)
				)
			)
			changed_transform.origin.y = clampf(
				start_camera_transform.origin.y + diff.y, min_y, MAX_CAMERA_Y
			)
			avatar.rotation.y = start_angle + diff.x
			camera_3d.transform = changed_transform


func reset_avatar_rotation() -> void:
	avatar.rotation.y = 0.0


func enable_outline():
	if outline_system and avatar:
		outline_system.set_outlined_avatar(avatar)


func disable_outline():
	if outline_system:
		outline_system.set_outlined_avatar(null)


func async_get_viewport_image(face: bool, dest_size: Vector2i, fov: float = 40) -> Image:
	avatar.emote_controller.freeze_on_idle()
	avatar.rotation.y = 0.0
	const PROFILE_BODY_CAMERA_POSITION = Vector3(0, 2.3, -3.5)
	const PROFILE_HEAD_CAMERA_POSITION = Vector3(0, 1.7, -1.25)
	camera_3d.position = PROFILE_HEAD_CAMERA_POSITION if face else PROFILE_BODY_CAMERA_POSITION
	camera_3d.rotation_degrees = DEFAULT_ROTATION if not face else Vector3(0.0, 180.0, 0.0)
	camera_3d.fov = fov

	# Store original values to restore after capture
	var original_stretch = stretch
	var original_size = size

	# Disable stretch to allow manual SubViewport sizing
	stretch = false
	set_size(dest_size)
	subviewport.set_size(dest_size)

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := subviewport.get_texture().get_image()

	# Restore original stretch and size
	stretch = original_stretch
	set_size(original_size)

	return img


func _notification(what: int) -> void:
	match what:
		NOTIFICATION_UNPARENTED:
			_on_tree_exiting()
		NOTIFICATION_PREDELETE:
			_on_tree_exiting()


func _on_tree_exiting() -> void:
	var parent = get_parent()
	var viewport = get_viewport()

	if (
		(
			not is_instance_valid(parent)
			or (is_instance_valid(parent) and parent.is_queued_for_deletion())
		)
		and is_instance_valid(viewport)
	):
		if get_parent() != viewport:
			reparent(viewport)
