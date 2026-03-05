extends Control

signal input_received(event: InputEvent)


## _unhandled_input() fires only for events
## not already consumed by the GUI.
## When the backpack is open, any touch
## on a wearable item travels through
## the GUI tree and reaches the Backpack root node
## (which has the default MOUSE_FILTER_STOP).
## That stop-filter control automatically calls accept_event(),
## marking the event as handled — so _unhandled_input
## never fires on the ActiveArea,
## and the joystick leaves the tap alone.
func _unhandled_input(event: InputEvent) -> void:
	input_received.emit(event)
