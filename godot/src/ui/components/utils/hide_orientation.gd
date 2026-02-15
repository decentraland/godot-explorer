@tool
extends Control

@export var hide_on_portrait: bool = false

var _original_visible: bool = true


func _ready() -> void:
	if Engine.is_editor_hint():
		_original_visible = visible
		set_process(true)
		_update_visibility_editor()
		return
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)


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


func _update_visibility_editor() -> void:
	var preview_active: bool = ProjectSettings.get_setting("_mobile_preview/active", false)
	if not preview_active:
		visible = true
		return
	var is_portrait: bool = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	if hide_on_portrait:
		visible = not is_portrait
	else:
		visible = is_portrait


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	if hide_on_portrait:
		visible = window_size.x > window_size.y
	else:
		visible = window_size.x < window_size.y
