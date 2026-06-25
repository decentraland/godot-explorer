class_name PresetAvatarCard
extends Button

# Distance in pixels a finger must travel before we treat it as a scroll.
const TAP_THRESHOLD = 20.0

@export var style_normal: StyleBox
@export var style_selected: StyleBox
@export var style_tapping: StyleBox

var preset_data: Dictionary = {}
var _touch_start = Vector2.ZERO
var _touch_active = false  # touch that started inside this card's rect
var _scroll_detected = false

@onready var _texture_rect: TextureRect = $TextureRect
@onready var _skeleton: RectangleSkeleton = $RectangleSkeleton


func _ready() -> void:
	toggle_mode = true
	# All interaction is handled manually in _input so the parent ScrollContainer
	# receives GUI events unobstructed.
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	self_modulate = Color.TRANSPARENT
	disabled = true
	UiSounds.install_audio(self)
	toggled.connect(_on_toggled)


func setup(image: Texture2D, data: Dictionary) -> void:
	preset_data = data
	_texture_rect.texture = image
	_skeleton.hide()
	self_modulate = Color.WHITE
	disabled = false
	_apply_style()


func _input(event: InputEvent) -> void:
	if disabled:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			_scroll_detected = false
			_touch_active = get_global_rect().has_point(event.position)
			if _touch_active:
				_touch_start = event.position
				_set_tap_style(true)
		elif _touch_active:
			_touch_active = false
			_set_tap_style(false)
			if not _scroll_detected and not button_pressed:
				# Confirmed tap: select this card and notify the ButtonGroup.
				button_pressed = true
				if button_group:
					for btn in button_group.get_buttons():
						if btn != self:
							btn.set_pressed_no_signal(false)
					button_group.pressed.emit(self)
			_scroll_detected = false
	elif event is InputEventScreenDrag:
		if _touch_active and not _scroll_detected:
			if event.position.distance_to(_touch_start) >= TAP_THRESHOLD:
				_scroll_detected = true
				_set_tap_style(false)


func _on_toggled(_pressed: bool) -> void:
	_apply_style()


func _apply_style() -> void:
	var style = style_selected if button_pressed else style_normal
	if style:
		add_theme_stylebox_override("normal", style)
		add_theme_stylebox_override("pressed", style)
		add_theme_stylebox_override("hover", style)
		add_theme_stylebox_override("hover_pressed", style)


func _set_tap_style(tapping: bool) -> void:
	if tapping:
		if style_tapping:
			add_theme_stylebox_override("normal", style_tapping)
			add_theme_stylebox_override("pressed", style_tapping)
			add_theme_stylebox_override("hover", style_tapping)
			add_theme_stylebox_override("hover_pressed", style_tapping)
	else:
		_apply_style()
