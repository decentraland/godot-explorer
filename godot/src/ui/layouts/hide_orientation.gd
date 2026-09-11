@tool
extends Control

## Shows/hides this control based on screen orientation. At runtime it follows the canonical
## Global.is_orientation_portrait() / Global.orientation_changed — the same source the other
## orientation-aware controls use, and reliable on mobile (DisplayServer.window_get_size() is not).
## In the editor it follows the mobile-preview toggle via _process and restores visibility on save.

@export var hide_on_portrait: bool = false

var _original_visible: bool = true


func _ready() -> void:
	if Engine.is_editor_hint():
		_original_visible = visible
		set_process(true)
		_update_visibility_editor()
		return
	if not Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.connect(_on_orientation_changed)
	_apply(Global.is_orientation_portrait())


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)
		return
	if Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.disconnect(_on_orientation_changed)


func _process(_delta: float) -> void:
	if Engine.is_editor_hint():
		_update_visibility_editor()


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		visible = _original_visible
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		_update_visibility_editor()


func _on_orientation_changed(_portrait: bool) -> void:
	_apply(Global.is_orientation_portrait())


func _apply(is_portrait: bool) -> void:
	visible = (not is_portrait) if hide_on_portrait else is_portrait


func _update_visibility_editor() -> void:
	var preview_active: bool = ProjectSettings.get_setting("_mobile_preview/active", false)
	if not preview_active:
		visible = true
		return
	var is_portrait: bool = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	_apply(is_portrait)
