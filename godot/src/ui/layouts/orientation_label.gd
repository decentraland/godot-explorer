@tool
class_name OrientationLabel
extends Label

## A Label whose font size follows the screen orientation: portrait_font_size in portrait,
## landscape_font_size in landscape, applied at runtime and re-applied on Global.orientation_changed
## (the editor shows the scene's own default size). Same self-managing pattern as OrientationSpacer /
## hide_orientation. If the label carries a LabelSettings it is duplicated per instance so sibling
## labels sharing the resource aren't clobbered; otherwise a theme font-size override is used.

@export var portrait_font_size: int = 30:
	set(value):
		portrait_font_size = value
		if is_node_ready() and not Engine.is_editor_hint():
			_apply_font_size()

@export var landscape_font_size: int = 28:
	set(value):
		landscape_font_size = value
		if is_node_ready() and not Engine.is_editor_hint():
			_apply_font_size()


func _ready() -> void:
	if Engine.is_editor_hint():
		return
	if not Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.connect(_on_orientation_changed)
	_apply_font_size()


func _exit_tree() -> void:
	if Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.disconnect(_on_orientation_changed)


func _on_orientation_changed(_portrait: bool) -> void:
	_apply_font_size()


func _apply_font_size() -> void:
	var size: int = portrait_font_size if Global.is_orientation_portrait() else landscape_font_size
	if label_settings:
		label_settings = label_settings.duplicate()
		label_settings.font_size = size
	else:
		add_theme_font_size_override("font_size", size)
