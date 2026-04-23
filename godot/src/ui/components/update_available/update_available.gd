extends Control

const PLAY_STORE_URL := "https://play.google.com/store/apps/details?id=org.decentraland.godotexplorer"
const APP_STORE_URL := "https://apps.apple.com/us/app/decentraland/id6478403840"

var _allow_later: bool = false
var _previous_portrait: bool = false

@onready var later_button: Button = %LaterButton
@onready var update_button: Button = %UpdateButton


func setup(allow_later: bool) -> void:
	_allow_later = allow_later


func _ready() -> void:
	_previous_portrait = Global.is_orientation_portrait()
	Global.set_orientation_portrait()
	later_button.visible = _allow_later
	update_button.pressed.connect(_on_update_pressed)
	later_button.pressed.connect(_on_later_pressed)


func _on_update_pressed() -> void:
	var url := APP_STORE_URL if Global.is_ios() else PLAY_STORE_URL
	Global.open_url(url)


func _on_later_pressed() -> void:
	if not _allow_later:
		return
	if not _previous_portrait:
		Global.set_orientation_landscape()
	var wrapper := get_parent()
	if wrapper is CanvasLayer:
		wrapper.queue_free()
	else:
		queue_free()
