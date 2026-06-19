class_name PresetAvatarCard
extends Button

const TAP_THRESHOLD = 20.0

var preset_data: Dictionary = {}
var _touch_start = Vector2.ZERO
var _touch_active = false
var _scroll_detected = false

@onready var _texture_rect: TextureRect = $TextureRect


func _ready() -> void:
	toggle_mode = true


func setup(image: Texture2D, data: Dictionary) -> void:
	preset_data = data
	_texture_rect.texture = image


func _input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.pressed:
		_scroll_detected = false
		_touch_start = event.position
	elif event is InputEventScreenDrag:
		if not _scroll_detected and event.position.distance_to(_touch_start) >= TAP_THRESHOLD:
			_scroll_detected = true


func _pressed() -> void:
	if _scroll_detected:
		button_pressed = not button_pressed
