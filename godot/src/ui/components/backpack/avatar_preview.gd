class_name AvatarPreview
extends SubViewportContainer

const MIN_CAMERA_SIZE = 1.0
const MAX_CAMERA_SIZE = 5.0

const DEFAULT_ROTATION = Vector3(-5, 180, 0)
const BODY_CAMERA_POSITION = Vector3(0, 1.25, -3.0)
const BODY_CAMERA_POSITION_WITH_PLATFORM = Vector3(0, 2.15, -3.3)
const HEAD_CAMERA_POSITION = Vector3(0, 1.8, -1.15)

@export var hide_name: bool = false
@export var show_platform: bool = false
@export var can_move: bool = true
@export var custom_environment: Environment = null
@export var with_light: bool = true

var start_angle
var start_dragging_position
var dirty_is_dragging
var on_head := false

@onready var avatar = %Avatar
@onready var camera_3d: Camera3D = %Camera3D
@onready var platform = %Platform
@onready var subviewport: SubViewport = %SubViewport
@onready var world_environment = $SubViewport/WorldEnvironment
@onready var directional_light_3d = $SubViewport/DirectionalLight3D
@onready var outline_system = %OutlineSystem


func get_body_camera_position() -> Vector3:
	return BODY_CAMERA_POSITION_WITH_PLATFORM if show_platform else BODY_CAMERA_POSITION


func _ready():
	if custom_environment != null:
		world_environment.environment = custom_environment

	directional_light_3d.visible = with_light

	avatar.hide_name = hide_name
	platform.set_visible(show_platform)

	camera_3d.set_position(get_body_camera_position())
	camera_3d.set_rotation_degrees(DEFAULT_ROTATION)

	if outline_system:
		outline_system.setup(camera_3d)

	#if can_move:
	#	gui_input.connect(self._on_gui_input)
	set_process_input(true)

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
			tween.set_parallel().tween_property(camera_3d, "size", 1.3, 0.5)
			on_head = true
		_:
			tween.tween_property(camera_3d, "position", get_body_camera_position(), 0.5)
			tween.set_parallel().tween_property(camera_3d, "size", 4.25, 0.5)
			on_head = false
	tween.play()


func _input(event: InputEvent):
	if not can_move:
		return
	if get_parent_control() and event is InputEventMouseButton:
		if not get_parent_control().get_global_rect().has_point(event.position):
			return

	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dirty_is_dragging = true
				start_dragging_position = get_global_mouse_position()
				start_angle = avatar.rotation.y
			else:
				dirty_is_dragging = false

		if not event.pressed:
			var dir: float = 0.0
			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				dir = -0.2
			elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				dir = 0.2

			if dir != 0.0:
				camera_3d.size = clampf(camera_3d.size + dir, MIN_CAMERA_SIZE, MAX_CAMERA_SIZE)

	if event is InputEventMouseMotion:
		if dirty_is_dragging:
			var diff = 0.005 * (get_global_mouse_position() - start_dragging_position)
			avatar.rotation.y = start_angle + diff.x


func reset_avatar_rotation() -> void:
	avatar.rotation.y = 0.0


func enable_outline():
	if outline_system and avatar:
		outline_system.set_outlined_avatar(avatar)


func disable_outline():
	if outline_system:
		outline_system.set_outlined_avatar(null)


func async_get_viewport_image(face: bool, dest_size: Vector2i, ortho_size: float = 2.5) -> Image:
	avatar.emote_controller.freeze_on_idle()
	avatar.rotation.y = 0.0
	const PROFILE_BODY_CAMERA_POSITION = Vector3(0, 1.25, -3.5)
	const PROFILE_HEAD_CAMERA_POSITION = Vector3(0, 1.70, -1.25)
	camera_3d.position = PROFILE_HEAD_CAMERA_POSITION if face else PROFILE_BODY_CAMERA_POSITION
	camera_3d.rotation_degrees = DEFAULT_ROTATION if not face else Vector3(0.0, 180.0, 0.0)
	camera_3d.size = ortho_size

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
