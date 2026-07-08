extends Button

signal touch_action_changed(pressed: bool)

@export var trigger_action = "ia_primary"

var _touch_index: int = -1
var _is_action_active: bool = false  # Tracks if we're actually sending the action


func _ready() -> void:
	# Drive the pressed state manually from raw touch so it works for every finger,
	# not just the primary one that Godot synthesizes a mouse event from.
	# - toggle_mode = true unlocks set_pressed_no_signal() (a no-op on non-toggle
	#   buttons), letting us flip the themed pressed stylebox ourselves.
	# - button_mask = 0 makes the base Button ignore the emulated mouse, so it never
	#   auto-toggles or fights our manual state; the button stays momentary.
	toggle_mode = true
	button_mask = 0


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
