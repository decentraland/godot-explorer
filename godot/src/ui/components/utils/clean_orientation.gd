@tool
extends Control

@export var clean_on_landscape: bool = false
@export var clean_on_portrait: bool = false

var _original_visible: bool = true


# gdlint:ignore = async-function-name
func _ready() -> void:
	if Engine.is_editor_hint():
		_original_visible = visible
		set_process(true)
		_update_visibility_editor()
		return
	await get_tree().process_frame
	_check_orientation_and_clean.call_deferred()


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
	if clean_on_landscape and not is_portrait:
		visible = false
	elif clean_on_portrait and is_portrait:
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
