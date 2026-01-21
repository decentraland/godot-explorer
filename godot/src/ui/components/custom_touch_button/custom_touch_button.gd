class_name CustomTouchButton
extends Button

const DRAG_THRESHOLD = 10.0

var is_pressing = false
var press_position: Vector2 = Vector2.ZERO
var is_dragging = false


func _init():
	button_mask = 0
	mouse_filter = Control.MOUSE_FILTER_PASS
	gui_input.connect(self._on_gui_input)


func _set_group_pressed_button(_pressed: bool):
	if is_instance_valid(button_group) and is_instance_valid(button_group.get_pressed_button()):
		button_group.get_pressed_button().set_pressed(_pressed)


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			is_pressing = true
			is_dragging = false
			press_position = get_global_mouse_position()
		elif is_pressing:
			is_pressing = false

			var release_position = get_global_mouse_position()

			# Check if finger moved beyond threshold (scroll gesture)
			if press_position.distance_to(release_position) > DRAG_THRESHOLD:
				is_dragging = true

			if is_dragging:
				return

			var inside = get_global_rect().has_point(release_position)
			if not inside:
				return

			if is_instance_valid(button_group):
				if button_group.allow_unpress:
					if button_group.get_pressed_button() == self:
						if button_pressed:
							set_pressed(false)
					else:
						_set_group_pressed_button(false)
						button_pressed = not button_pressed
				else:
					if button_group.get_pressed_button() != self:
						_set_group_pressed_button(false)
						button_pressed = not button_pressed
			else:
				button_pressed = not button_pressed
