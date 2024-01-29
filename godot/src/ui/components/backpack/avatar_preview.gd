extends SubViewportContainer

@export var hide_name: bool = false
@export var show_platform: bool = false

var start_camera_transform
var start_angle
var start_dragging_position
var dirty_is_dragging

@onready var avatar = %Avatar
@onready var camera_3d = $SubViewport/Camera3D
@onready var platform = $SubViewport/Sprite3D_Platform

func _ready():
	avatar.hide_name = hide_name
	platform.set_visible(show_platform)
	if Global.standalone:
		pass
		# TODO: this config no longer exists
		#avatar.async_update_avatar(Global.config.avatar_profile)


func focus_camera_on(type):
	var tween = create_tween().set_parallel()
	match type:
		Wearables.Categories.HAIR, Wearables.Categories.FACIAL_HAIR, Wearables.Categories.EYEWEAR, Wearables.Categories.TIARA, Wearables.Categories.FACIAL, Wearables.Categories.EYEBROWS, Wearables.Categories.MOUTH, Wearables.Categories.HAT, Wearables.Categories.EARRING, Wearables.Categories.MASK, Wearables.Categories.HELMET, Wearables.Categories.TOP_HEAD, Wearables.Categories.EYES:
			tween.tween_property(camera_3d, "position", Vector3(0, 1.68, -0.523), 0.5)
			tween.tween_property(camera_3d, "size", 1, 0.5)
		_:
			tween.tween_property(camera_3d, "position", Vector3(0, 0.957, -1.623), 0.5)
			tween.tween_property(camera_3d, "size", 3, 0.5)
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
					camera_3d.transform.origin.z + dir, -1.6, -0.4
				)

	if event is InputEventMouseMotion:
		if dirty_is_dragging:
			var diff = 0.005 * (get_global_mouse_position() - start_dragging_position)
			var changed_transform = Transform3D(start_camera_transform)
			changed_transform.origin.y = clampf(start_camera_transform.origin.y + diff.y, 0.2, 2)
			avatar.rotation.y = start_angle + diff.x
			camera_3d.transform = changed_transform
