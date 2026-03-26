@tool
extends Control

@export var clean_on_landscape: bool = false
@export var clean_on_portrait: bool = false

var _original_visible: bool = true
var _watcher: OrientationWatcher


# gdlint:ignore = async-function-name
func _ready() -> void:
	if Engine.is_editor_hint():
		_original_visible = visible
		_watcher = OrientationWatcher.new()
		_watcher.orientation_changed.connect(_on_watcher_orientation_changed)
		add_child(_watcher)
		return
	await get_tree().process_frame
	_check_orientation_and_clean.call_deferred()


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		visible = _original_visible
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		if _watcher:
			_on_watcher_orientation_changed(_watcher.get_is_portrait())


func _on_watcher_orientation_changed(is_portrait: bool) -> void:
	if not OrientationWatcher.is_editor_preview_active():
		visible = true
		return
	if (clean_on_landscape and not is_portrait) or (clean_on_portrait and is_portrait):
		visible = false
	else:
		visible = true


func _check_orientation_and_clean() -> void:
	if clean_on_landscape and !Global.is_orientation_portrait():
		queue_free()
		return

	if clean_on_portrait and Global.is_orientation_portrait():
		queue_free()
		return
