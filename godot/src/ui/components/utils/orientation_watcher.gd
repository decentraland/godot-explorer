## Lightweight @tool node that tracks device orientation and emits a signal when it changes.
## Add as a child of any node that needs to react to portrait/landscape switches.
@tool
class_name OrientationWatcher
extends Node

signal orientation_changed(is_portrait: bool)

var _is_portrait: bool = false
var _initialized: bool = false


func _ready() -> void:
	if Engine.is_editor_hint():
		set_process(true)
		_poll_editor()
		return
	get_window().size_changed.connect(_on_size_changed)
	_on_size_changed()


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)


func _process(_delta: float) -> void:
	if Engine.is_editor_hint():
		_poll_editor()


func _poll_editor() -> void:
	var is_portrait: bool
	if is_editor_preview_active():
		is_portrait = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	else:
		var vp_w: int = ProjectSettings.get_setting("display/window/size/viewport_width", 720)
		var vp_h: int = ProjectSettings.get_setting("display/window/size/viewport_height", 720)
		is_portrait = vp_w < vp_h
	_set_is_portrait(is_portrait)


func _on_size_changed() -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	_set_is_portrait(window_size.x < window_size.y)


func _set_is_portrait(value: bool) -> void:
	if _initialized and value == _is_portrait:
		return
	_initialized = true
	_is_portrait = value
	orientation_changed.emit(value)


func get_is_portrait() -> bool:
	return _is_portrait


static func is_editor_preview_active() -> bool:
	return ProjectSettings.get_setting("_mobile_preview/active", false)
