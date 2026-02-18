@tool
class_name ResponsiveContainer
extends Container

## A container that adapts its size based on screen percentages and orientation.

@export_group("Portrait Mode")
@export_range(0.0, 1.0, 0.01) var portrait_width: float = 0.90:
	set(v):
		portrait_width = v
		_request_update()

@export_range(0.0, 1.0, 0.01) var portrait_max_height: float = 0.80:
	set(v):
		portrait_max_height = v
		_request_update()

@export_range(0.0, 1.0, 0.01) var portrait_min_height: float = 0.0:
	set(v):
		portrait_min_height = v
		_request_update()

@export_group("Landscape Mode")
@export_range(0.0, 1.0, 0.01) var landscape_width: float = 0.45:
	set(v):
		landscape_width = v
		_request_update()

@export_range(0.0, 1.0, 0.01) var landscape_max_height: float = 0.80:
	set(v):
		landscape_max_height = v
		_request_update()

@export_range(0.0, 1.0, 0.01) var landscape_min_height: float = 0.0:
	set(v):
		landscape_min_height = v
		_request_update()

@export_group("Alignment")
@export var center_horizontal: bool = true:
	set(v):
		center_horizontal = v
		_request_update()

@export var center_vertical: bool = true:
	set(v):
		center_vertical = v
		_request_update()

@export var vertical_offset: float = 0.0:
	set(v):
		vertical_offset = v
		_request_update()

@export_group("Debug (Editor Only)")
@export var simulate_portrait: bool = false:
	set(v):
		simulate_portrait = v
		_request_update()


func _ready():
	_update_layout()


func _notification(what):
	match what:
		NOTIFICATION_SORT_CHILDREN, NOTIFICATION_RESIZED:
			_update_layout()


func _request_update():
	if is_inside_tree():
		_update_layout()


func _is_portrait() -> bool:
	if Engine.is_editor_hint():
		var preview_active = ProjectSettings.get_setting("_mobile_preview/active", false)
		if preview_active:
			return ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
		return simulate_portrait
	return Global.is_orientation_portrait()


func _get_reference_size() -> Vector2:
	if Engine.is_editor_hint():
		var root = get_tree().edited_scene_root
		if root and root != self:
			return root.size

		return Vector2(
			ProjectSettings.get_setting("display/window/size/viewport_width"),
			ProjectSettings.get_setting("display/window/size/viewport_height")
		)
	return get_viewport_rect().size


func _update_layout():
	if not is_inside_tree():
		return

	var ref_size = _get_reference_size()
	var is_portrait = _is_portrait()

	# Select values based on orientation
	var width_percent: float
	var max_height_percent: float
	var min_height_percent: float

	if is_portrait:
		width_percent = portrait_width
		max_height_percent = portrait_max_height
		min_height_percent = portrait_min_height
	else:
		width_percent = landscape_width
		max_height_percent = landscape_max_height
		min_height_percent = landscape_min_height

	# Calculate dimensions
	var target_width = ref_size.x * width_percent
	var max_height = ref_size.y * max_height_percent
	var min_height = ref_size.y * min_height_percent

	var content_height = get_combined_minimum_size().y
	var final_height = clamp(content_height, min_height, max_height)

	set_deferred("size", Vector2(target_width, final_height))

	# Position
	var pos = Vector2.ZERO

	if center_horizontal:
		pos.x = (ref_size.x - size.x) / 2

	if center_vertical:
		pos.y = (ref_size.y - size.y) / 2 + vertical_offset
	else:
		pos.y = vertical_offset

	position = pos
