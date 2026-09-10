@tool
class_name OrientationSpacer
extends Control

## A spacer whose `custom_minimum_size` follows the screen orientation: `portrait_minimum_size` in
## portrait, `landscape_minimum_size` in landscape, re-evaluated on rotation.
##
## At runtime it follows `Global.is_orientation_portrait()` / `Global.orientation_changed` — the
## canonical project orientation, the same source SafeMarginContainer and SettingsSectionItem use, so
## every orientation-aware control agrees (a window-size heuristic can disagree with it on mobile).
## In the editor it follows the mobile-preview toggle via `_process`, and clears the size on save so
## the previewed dimension isn't baked into the scene.

@export var portrait_minimum_size: Vector2 = Vector2.ZERO:
	set(value):
		portrait_minimum_size = value
		if Engine.is_editor_hint() and is_inside_tree():
			_refresh()

@export var landscape_minimum_size: Vector2 = Vector2.ZERO:
	set(value):
		landscape_minimum_size = value
		if Engine.is_editor_hint() and is_inside_tree():
			_refresh()


func _ready() -> void:
	if Engine.is_editor_hint():
		set_process(true)
		_refresh()
		return
	if not Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.connect(_on_orientation_changed)
	_set_for_orientation(Global.is_orientation_portrait())


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)
		return
	if Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.disconnect(_on_orientation_changed)


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


func _on_orientation_changed(_portrait: bool) -> void:
	_set_for_orientation(Global.is_orientation_portrait())


## In the editor there's no window rotation, so mirror hide_orientation and read the mobile preview;
## an inactive preview is treated as portrait.
func _refresh() -> void:
	if not Engine.is_editor_hint():
		_set_for_orientation(Global.is_orientation_portrait())
		return
	var is_portrait: bool = true
	if ProjectSettings.get_setting("_mobile_preview/active", false):
		is_portrait = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	_set_for_orientation(is_portrait)


func _set_for_orientation(is_portrait: bool) -> void:
	custom_minimum_size = portrait_minimum_size if is_portrait else landscape_minimum_size
