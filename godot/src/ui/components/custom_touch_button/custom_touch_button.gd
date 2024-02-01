class_name CustomTouchButton
extends Button

var is_pressing = false


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
		elif is_pressing and not event.pressed:
			is_pressing = false
			var inside = get_global_rect().has_point(get_global_mouse_position())
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
