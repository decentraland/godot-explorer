@tool
extends BoxContainer

@export var invert: bool = false

var _original_vertical: bool = false


func _ready() -> void:
	if Engine.is_editor_hint():
		_original_vertical = vertical
		set_process(true)
		_update_orientation_editor()
		return
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)


func _process(_delta: float) -> void:
	if Engine.is_editor_hint():
		_update_orientation_editor()


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		vertical = _original_vertical
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		_update_orientation_editor()


func _update_orientation_editor() -> void:
	var preview_active: bool = ProjectSettings.get_setting("_mobile_preview/active", false)
	var is_portrait: bool
	if preview_active:
		is_portrait = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	else:
		var vp_w: int = ProjectSettings.get_setting("display/window/size/viewport_width", 720)
		var vp_h: int = ProjectSettings.get_setting("display/window/size/viewport_height", 720)
		is_portrait = vp_w < vp_h
	self.vertical = is_portrait
	if invert:
		self.vertical = not self.vertical


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	self.vertical = window_size.x < window_size.y
	if invert:
		self.vertical = !self.vertical
