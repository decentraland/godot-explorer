extends Button

signal touch_action_changed(pressed: bool)

@export var trigger_action = "ia_primary"

var _touch_index: int = -1
var _is_action_active: bool = false  # Tracks if we're actually sending the action

# Optional scene-provided icon (PBTouchScreenControls), rendered on a dedicated overlay so
# it never interferes with the button's native glyph (icon/text). Toggle via set/clear.
var _custom_icon: TextureRect


func _ready() -> void:
	# Drive the pressed state manually from raw touch so it works for every finger,
	# not just the primary one that Godot synthesizes a mouse event from.
	# - toggle_mode = true unlocks set_pressed_no_signal() (a no-op on non-toggle
	#   buttons), letting us flip the themed pressed stylebox ourselves.
	# - button_mask = 0 makes the base Button ignore the emulated mouse, so it never
	#   auto-toggles or fights our manual state; the button stays momentary.
	toggle_mode = true
	button_mask = 0

	_custom_icon = TextureRect.new()
	_custom_icon.name = "CustomIcon"
	_custom_icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_custom_icon.set_anchors_preset(Control.PRESET_FULL_RECT)
	_custom_icon.offset_left = 6
	_custom_icon.offset_top = 6
	_custom_icon.offset_right = -6
	_custom_icon.offset_bottom = -6
	_custom_icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_custom_icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	_custom_icon.hide()
	add_child(_custom_icon)


## Show a scene-provided icon on the overlay (leaves the native glyph untouched underneath).
func set_custom_icon(texture: Texture2D) -> void:
	if _custom_icon == null:
		return
	_custom_icon.texture = texture
	_custom_icon.show()


## Hide the scene-provided icon overlay, revealing the native glyph again.
func clear_custom_icon() -> void:
	if _custom_icon == null:
		return
	_custom_icon.texture = null
	_custom_icon.hide()


func _on_gui_input(event: InputEvent) -> void:
	if disabled:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			if _touch_index == -1:
				_touch_index = event.index
				_is_action_active = true
				set_pressed_no_signal(true)
				Input.action_press(trigger_action)
				button_down.emit()
				touch_action_changed.emit(true)
			accept_event()
		elif not event.pressed and event.index == _touch_index:
			if _is_action_active:
				Input.action_release(trigger_action)
				_is_action_active = false
				button_up.emit()
				touch_action_changed.emit(false)
			set_pressed_no_signal(false)
			_touch_index = -1
			accept_event()
