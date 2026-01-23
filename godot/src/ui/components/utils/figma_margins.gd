@tool
class_name FigmaMargins
extends MarginContainer

## A container that adapts its size based on screen percentages and orientation.

@export_group("Portrait Figma Margins")
@export var height_portrait: float = 720.0:
	set(v):
		height_portrait = v
		_request_update()
@export var top_portrait: float = 20.0:
	set(v):
		top_portrait = v
		_request_update()
@export var right_portrait: float = 20.0:
	set(v):
		right_portrait = v
		_request_update()
@export var bottom_portrait: float = 20.0:
	set(v):
		bottom_portrait = v
		_request_update()
@export var portrait_left_margin: float = 20.0:
	set(v):
		portrait_left_margin = v
		_request_update()

@export_group("Landscape Figma Margins")
@export var height_landscame: float = 720.0:
	set(v):
		height_landscame = v
		_request_update()
@export var top_landscape: float = 20.0:
	set(v):
		top_landscape = v
		_request_update()
@export var right_landscape: float = 20.0:
	set(v):
		right_landscape = v
		_request_update()
@export var bottom_landscape: float = 20.0:
	set(v):
		bottom_landscape = v
		_request_update()
@export var left_landscape: float = 20.0:
	set(v):
		left_landscape = v
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
		NOTIFICATION_SORT_CHILDREN, \
		NOTIFICATION_RESIZED:
			_update_layout()


func _request_update():
	if is_inside_tree():
		_update_layout()


func _is_portrait() -> bool:
	if Engine.is_editor_hint():
		return simulate_portrait
	else:
		return Global.is_orientation_portrait()


func _get_reference_size() -> Vector2:
	if Engine.is_editor_hint():
		var root = get_tree().edited_scene_root
		if root and root != self:
			return root.size
		else:
			return Vector2(
				ProjectSettings.get_setting("display/window/size/viewport_width"),
				ProjectSettings.get_setting("display/window/size/viewport_height")
			)
	else:
		return get_viewport_rect().size


func _update_layout():
	if not is_inside_tree():
		return
	
	var ref_size = _get_reference_size()
	var is_portrait = _is_portrait()
	
	# Select values based on orientation
	var margin_left: float
	var margin_right: float
	var margin_top: float
	var margin_bottom: float

	var portrait_scale: float = ref_size.y / height_portrait
	var landscape_scale: float = ref_size.y / height_landscame

	if is_portrait:
		margin_left = portrait_left_margin * portrait_scale
		margin_right = right_portrait * portrait_scale
		margin_top = top_portrait * portrait_scale
		margin_bottom = bottom_portrait * portrait_scale
	else:
		margin_left = left_landscape * landscape_scale
		margin_right = right_landscape * landscape_scale
		margin_top = top_landscape * landscape_scale
		margin_bottom = bottom_landscape * landscape_scale
		
	add_theme_constant_override("margin_left", margin_left)
	add_theme_constant_override("margin_right", margin_right)
	add_theme_constant_override("margin_top", margin_top)
	add_theme_constant_override("margin_bottom", margin_bottom)
	
