extends Control

@export var clean_on_landscape: bool = false
@export var clean_on_portrait: bool = false


func _ready() -> void:
	await get_tree().process_frame
	_check_orientation_and_clean.call_deferred()


func _check_orientation_and_clean() -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var is_landscape: bool = window_size.x > window_size.y
	var is_portrait: bool = window_size.y > window_size.x

	if clean_on_landscape and is_landscape:
		queue_free()
		return

	if clean_on_portrait and is_portrait:
		queue_free()
		return
