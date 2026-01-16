extends Control

@export var clean_on_landscape: bool = false
@export var clean_on_portrait: bool = false


func _ready() -> void:
	_check_orientation_and_clean.call_deferred()


func _check_orientation_and_clean() -> void:
	if clean_on_landscape and !Global.is_orientation_portrait():
		queue_free()
		return

	if clean_on_portrait and Global.is_orientation_portrait():
		queue_free()
		return
