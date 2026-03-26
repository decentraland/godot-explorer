@tool
extends Control

@export var hide_on_portrait: bool = false

var _original_visible: bool = true
var _watcher: OrientationWatcher


func _ready() -> void:
	if Engine.is_editor_hint():
		_original_visible = visible
	_watcher = OrientationWatcher.new()
	_watcher.orientation_changed.connect(_on_watcher_orientation_changed)
	add_child(_watcher)


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		visible = _original_visible
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		if _watcher:
			_on_watcher_orientation_changed(_watcher.get_is_portrait())


func _on_watcher_orientation_changed(is_portrait: bool) -> void:
	if Engine.is_editor_hint() and not OrientationWatcher.is_editor_preview_active():
		visible = true
		return
	visible = not is_portrait if hide_on_portrait else is_portrait
