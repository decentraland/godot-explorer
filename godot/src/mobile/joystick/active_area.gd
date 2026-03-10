extends Control

signal input_received(event: InputEvent)

var _enabled := true


func _ready() -> void:
	Global.on_menu_open.connect(func(): _enabled = false)
	Global.on_menu_close.connect(func(): _enabled = true)


func _input(event: InputEvent) -> void:
	if _enabled:
		input_received.emit(event)
