@tool
class_name OrientationSpacer
extends Control

## A spacer whose `custom_minimum_size` follows the screen orientation: `portrait_minimum_size` in
## portrait, `landscape_minimum_size` in landscape, re-evaluated on rotation.
##
## Same triggers as hide_orientation.gd: at runtime it listens to the window's `size_changed` (which
## fires on rotation); in the editor it follows the mobile-preview toggle via `_process`, and clears
## the size on save so the previewed dimension isn't baked into the scene.

@export var portrait_minimum_size: Vector2 = Vector2.ZERO:
	set(value):
		portrait_minimum_size = value
		_refresh()

@export var landscape_minimum_size: Vector2 = Vector2.ZERO:
	set(value):
		landscape_minimum_size = value
		_refresh()


func _ready() -> void:
	if Engine.is_editor_hint():
		set_process(true)
		_refresh()
		return
	get_window().size_changed.connect(_on_size_changed)
	_on_size_changed()


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)


func _process(_delta: float) -> void:
	if Engine.is_editor_hint():
		_refresh()


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		custom_minimum_size = Vector2.ZERO
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		_refresh()


func _on_size_changed() -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	_set_for_orientation(window_size.y >= window_size.x)


## In the editor there's no window rotation, so mirror hide_orientation and read the mobile preview;
## an inactive preview is treated as portrait.
func _refresh() -> void:
	if not Engine.is_editor_hint():
		_on_size_changed()
		return
	var is_portrait: bool = true
	if ProjectSettings.get_setting("_mobile_preview/active", false):
		is_portrait = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	_set_for_orientation(is_portrait)


func _set_for_orientation(is_portrait: bool) -> void:
	custom_minimum_size = portrait_minimum_size if is_portrait else landscape_minimum_size
