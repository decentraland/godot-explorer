extends Button

signal touch_action_changed(pressed: bool)

@export var trigger_action = "ia_primary"

var _touch_index: int = -1
var _is_action_active: bool = false  # Tracks if we're actually sending the action

# Optional scene-provided icon (PBTouchScreenControls), rendered on a dedicated overlay so
# it never interferes with the button's native glyph (icon/text). Toggle via set/clear.
# The overlay node lives in button_touch_action.tscn (hidden by default).
@onready var _custom_icon: TextureRect = $CustomIcon


func _ready() -> void:
	# Drive the pressed state manually from raw touch so it works for every finger,
	# not just the primary one that Godot synthesizes a mouse event from.
	# - toggle_mode = true unlocks set_pressed_no_signal() (a no-op on non-toggle
	#   buttons), letting us flip the themed pressed stylebox ourselves.
	# - button_mask = 0 makes the base Button ignore the emulated mouse, so it never
	#   auto-toggles or fights our manual state; the button stays momentary.
	toggle_mode = true
	button_mask = 0


## Latch the orb "Hold" look (e.g. glider active). No-op until an OrbSkin child is present.
func set_hold(on: bool) -> void:
	for child in get_children():
		if child is OrbSkin:
			child.set_hold(on)


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

	# Desktop runs "mobile simulation": treat left mouse clicks like screen
	# touches so the on-screen buttons work the same as on a phone.
	var is_press_event := false
	var is_release_event := false
	if event is InputEventScreenTouch:
		is_press_event = event.pressed and (event.index == _touch_index or _touch_index == -1)
		is_release_event = not event.pressed and event.index == _touch_index
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		is_press_event = event.pressed
		is_release_event = not event.pressed

	if is_press_event:
		if event is InputEventScreenTouch:
			_touch_index = event.index
		_is_action_active = true
		set_pressed_no_signal(true)
		Input.action_press(trigger_action)
		DclGlobal.set_joypad_input_active(true)
		button_down.emit()
		touch_action_changed.emit(true)
		accept_event()
	elif is_release_event:
		if _is_action_active:
			Input.action_release(trigger_action)
			_is_action_active = false
			button_up.emit()
			touch_action_changed.emit(false)
		set_pressed_no_signal(false)
		_touch_index = -1
		DclGlobal.set_joypad_input_active(false)
		accept_event()
