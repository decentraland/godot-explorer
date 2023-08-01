extends SubViewportContainer

@onready var avatar = %Avatar
@onready var camera_3d = $SubViewport/Camera3D


# Called when the node enters the scene tree for the first time.
func _ready():
	if Global.standalone:
		avatar.update_avatar(Global.config.avatar_profile)


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass


var start_camera_transform
var start_angle
var start_dragging_position
var dirty_is_dragging


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
