@tool
class_name OrientationVBox
extends VBoxContainer

## A VBoxContainer whose `separation` follows the screen orientation: portrait_separation in portrait,
## landscape_separation in landscape, re-evaluated on rotation. Same self-managing pattern as
## OrientationSpacer / OrientationLabel: at runtime it follows Global.is_orientation_portrait() /
## Global.orientation_changed; in the editor it follows the mobile-preview toggle via _process and
## clears the override on save so the previewed value isn't baked into the scene.

@export var portrait_separation: int = 0:
	set(value):
		portrait_separation = value
		if Engine.is_editor_hint() and is_inside_tree():
			_refresh()

@export var landscape_separation: int = 0:
	set(value):
		landscape_separation = value
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
		remove_theme_constant_override("separation")
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		_refresh()


func _on_orientation_changed(_portrait: bool) -> void:
	_set_for_orientation(Global.is_orientation_portrait())


## In the editor there's no window rotation, so read the mobile preview; an inactive preview is
## treated as landscape.
func _refresh() -> void:
	if not Engine.is_editor_hint():
		_set_for_orientation(Global.is_orientation_portrait())
		return
	var is_portrait: bool = false
	if ProjectSettings.get_setting("_mobile_preview/active", false):
		is_portrait = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	_set_for_orientation(is_portrait)


func _set_for_orientation(is_portrait: bool) -> void:
	add_theme_constant_override(
		"separation", portrait_separation if is_portrait else landscape_separation
	)
