class_name SdkTouchControlsApplier
extends RefCounted

## Applies the PBTouchScreenControls component state (Global.touch_controls_hide_joystick /
## Global.touch_controls_hide_crosshair) to the explorer HUD, letting a scene hide the
## native joystick/crosshair so creators can render their own touch UI (bound via
## PBUiInputBinding). The gamepad action buttons are configured separately by the joypad
## itself (denylist / main_action).
##
## Driven from Explorer._process wherever the on-screen controls are shown (mobile +
## desktop dev, never XR). Only acts on state changes and defers to the explorer's
## hide-ui state when restoring visibility.

var _virtual_joystick: Control
var _label_crosshair: Control
var _hide_joystick_applied: bool = false
var _hide_crosshair_applied: bool = false


func _init(virtual_joystick: Control, label_crosshair: Control) -> void:
	_virtual_joystick = virtual_joystick
	_label_crosshair = label_crosshair


## `hidden_for_hide_ui` is the explorer's "hide UI" state, which wins when restoring.
func apply(hidden_for_hide_ui: bool) -> void:
	_apply_joystick(hidden_for_hide_ui)
	_apply_crosshair()


func _apply_joystick(hidden_for_hide_ui: bool) -> void:
	# Enforce the hidden state every frame (other HUD logic may re-show these), but only
	# restore visibility once, on the transition back, so we don't fight the HUD state.
	# Hide the joystick's visuals/touch area (not the whole node) so the camera (first/
	# third-person) button stays visible and usable while the native joystick is hidden.
	var hide_joystick: bool = Global.touch_controls_hide_joystick
	if hide_joystick:
		_virtual_joystick.set_visuals_hidden(true)
	elif _hide_joystick_applied and not hidden_for_hide_ui:
		_virtual_joystick.set_visuals_hidden(false)
	_hide_joystick_applied = hide_joystick


# Restores mobile shown / desktop-only-while-captured.
func _apply_crosshair() -> void:
	if _label_crosshair == null:
		return

	var hide_crosshair: bool = Global.touch_controls_hide_crosshair
	if hide_crosshair:
		_label_crosshair.hide()
	elif _hide_crosshair_applied:
		if Global.is_mobile() or Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
			_label_crosshair.show()
	_hide_crosshair_applied = hide_crosshair
