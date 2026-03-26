@tool
extends BoxContainer

@export var invert: bool = false

var _original_vertical: bool = false
var _watcher: OrientationWatcher


func _ready() -> void:
	if Engine.is_editor_hint():
		_original_vertical = vertical
	_watcher = OrientationWatcher.new()
	_watcher.orientation_changed.connect(_on_watcher_orientation_changed)
	add_child(_watcher)


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		vertical = _original_vertical
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		if _watcher:
			_on_watcher_orientation_changed(_watcher.get_is_portrait())


func _on_watcher_orientation_changed(is_portrait: bool) -> void:
	var new_vertical: bool = is_portrait if not invert else not is_portrait
	if new_vertical == self.vertical:
		return
	self.vertical = new_vertical
