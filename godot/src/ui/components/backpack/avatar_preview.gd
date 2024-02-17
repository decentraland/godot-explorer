class_name AvatarPreview
extends SubViewportContainer

const MIN_CAMERA_Z = -3.5
const MAX_CAMERA_Z = -1.25

const MIN_CAMERA_Y = 0.8
const MAX_CAMERA_Y = 2.3

const DEFAULT_ROTATION = Vector3(-20, 180, 0)
const BODY_CAMERA_POSITION = Vector3(0, 2.3, -3.5)
const HEAD_CAMERA_POSITION = Vector3(0, 2, -1.25)

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

func _ready():
	if custom_environment != null:
		world_environment.environment = custom_environment
	
	directional_light_3d.visible = with_light
		
	avatar.hide_name = hide_name
	platform.set_visible(show_platform)

	camera_3d.set_position(BODY_CAMERA_POSITION)
	camera_3d.set_rotation_degrees(DEFAULT_ROTATION)

	if can_move:
		gui_input.connect(self._on_gui_input)

	if Global.standalone:
		Global.player_identity.set_default_profile()
		var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
		avatar.async_update_avatar_from_profile(profile)


func focus_camera_on(type):
	var tween := create_tween()
	if tween == null:
		return

	tween = tween.set_parallel()
	match type:
		Wearables.Categories.HAIR, Wearables.Categories.FACIAL_HAIR, Wearables.Categories.EYEWEAR, Wearables.Categories.TIARA, Wearables.Categories.FACIAL, Wearables.Categories.EYEBROWS, Wearables.Categories.MOUTH, Wearables.Categories.HAT, Wearables.Categories.EARRING, Wearables.Categories.MASK, Wearables.Categories.HELMET, Wearables.Categories.TOP_HEAD, Wearables.Categories.EYES:
			tween.tween_property(camera_3d, "position", HEAD_CAMERA_POSITION, 0.5)
		_:
			tween.tween_property(camera_3d, "position", BODY_CAMERA_POSITION, 0.5)
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


func async_get_viewport_image(face: bool, dest_size: Vector2i, fov: Variant = null) -> Image:
	avatar.emote_controller.freeze_on_idle()
	const PROFILE_BODY_CAMERA_POSITION = Vector3(0, 2.3, -3.5)
	const PROFILE_HEAD_CAMERA_POSITION = Vector3(0, 1.6, -1.25)
	camera_3d.position = PROFILE_HEAD_CAMERA_POSITION if face else PROFILE_BODY_CAMERA_POSITION
	camera_3d.rotation_degrees = DEFAULT_ROTATION if not face else Vector3(0.0, 180.0, 0.0)
	if fov is float:
		camera_3d.fov = fov

	self.size = dest_size
	subviewport.size = dest_size

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := subviewport.get_texture().get_image()
	return img
