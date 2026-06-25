class_name PresetAvatarCard
extends Button

const TAP_THRESHOLD = 20.0

var preset_data: Dictionary = {}
var _touch_start = Vector2.ZERO
var _touch_active = false
var _scroll_detected = false

@onready var _texture_rect: TextureRect = $TextureRect
@onready var _skeleton: RectangleSkeleton = $RectangleSkeleton


func _ready() -> void:
	toggle_mode = true
	self_modulate = Color.TRANSPARENT
	disabled = true
	UiSounds.install_audio(self)


func setup(image: Texture2D, data: Dictionary) -> void:
	preset_data = data
	_texture_rect.texture = image
	_skeleton.hide()
	self_modulate = Color.WHITE
	disabled = false


func _input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_scroll_detected = false
			_touch_start = event.position
		elif _scroll_detected:
			_scroll_detected = false
			mouse_filter = Control.MOUSE_FILTER_STOP
	elif event is InputEventScreenDrag:
		if not _scroll_detected and event.position.distance_to(_touch_start) >= TAP_THRESHOLD:
			_scroll_detected = true
			mouse_filter = Control.MOUSE_FILTER_IGNORE


func _pressed() -> void:
	if _scroll_detected:
		button_pressed = not button_pressed
