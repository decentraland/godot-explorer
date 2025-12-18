extends Button

@export var trigger_action = "ia_primary"

var _touch_index: int = -1
var _is_action_active: bool = false  # Tracks if we're actually sending the action


func _ready() -> void:
	# Disable toggle_mode for normal button behavior
	toggle_mode = false


func _input(event: InputEvent) -> void:
	if disabled:
		return

	if not Global.is_mobile():
		return

	if event is InputEventScreenTouch:
		var touch_pos = event.position
		var is_inside = _is_point_inside_button(touch_pos)

		if event.pressed:
			if is_inside and _touch_index == -1:
				_touch_index = event.index
				_is_action_active = true

				set_pressed_no_signal(true)
				Input.action_press(trigger_action)
				_close_combo_menu()
		else:
			if event.index == _touch_index:
				if _is_action_active:
					Input.action_release(trigger_action)
					_is_action_active = false
				set_pressed_no_signal(false)
				_touch_index = -1
	elif event is InputEventScreenDrag:
		if _touch_index == event.index:
			var touch_pos = event.position
			var is_inside = _is_point_inside_button(touch_pos)

			if is_inside and not _is_action_active:
				pass
			elif not is_inside and _is_action_active:
				Input.action_release(trigger_action)
				_is_action_active = false
				set_pressed_no_signal(false)


func _on_gui_input(event: InputEvent) -> void:
	if disabled:
		return

	if event is InputEventScreenTouch:
		var touch_pos = event.position
		var is_inside = get_rect().has_point(touch_pos)

		if event.pressed and is_inside:
			if _touch_index == -1:
				_touch_index = event.index
				_is_action_active = true
				set_pressed_no_signal(true)
				Input.action_press(trigger_action)
				_close_combo_menu()
			accept_event()
		elif not event.pressed and event.index == _touch_index:
			if _is_action_active:
				Input.action_release(trigger_action)
				_is_action_active = false
			set_pressed_no_signal(false)
			_touch_index = -1
			accept_event()
	elif event is InputEventScreenDrag:
		if _touch_index == event.index:
			var touch_pos = event.position
			var is_inside = get_rect().has_point(touch_pos)

			if is_inside and not _is_action_active:
				pass
			elif not is_inside and _is_action_active:
				Input.action_release(trigger_action)
				_is_action_active = false
				set_pressed_no_signal(false)


func _is_point_inside_button(point: Vector2) -> bool:
	var global_rect = Rect2(global_position, size * get_global_transform_with_canvas().get_scale())
	return global_rect.has_point(point)


func _close_combo_menu() -> void:
	var actions_to_close = ["ia_action_3", "ia_action_4", "ia_action_5", "ia_action_6"]
	if trigger_action in actions_to_close:
		Global.close_combo.emit()
