@tool
class_name ScrollRubberContainer
extends Container

signal scroll_ended

enum ScrollMode { DISABLED, AUTO, ALWAYSSHOW, NEVERSHOW, RESERVE }  ## Wont scroll in this direction  ## Unused property  ## Unused property  ## Unused property  ## Unused property

@export_category("ScrollContainer properties")
@export var follow_focus: bool  ## Unused property
@export var draw_focus_border: bool  ## Unused property
@export var horizontal_scroll_mode := ScrollMode.AUTO
@export var vertical_scroll_mode := ScrollMode.AUTO
@export var scroll_deadzone := 50

@export_category("Rubber Band behavior")
@export var drag := 0.8
@export var rubber_force := 8.0
@export var take_hight_from_children: bool = false

var start_pos: Vector2
var child_position: Vector2
var child_drag_position: Vector2
var child_physics_position: Vector2
var drag_tween: Tween
var is_touching := false
var is_scrolling_x := false
var is_scrolling_y := false

var velocity: Vector2
var force: Vector2
var previous_position: Vector2


func reset_position():
	if not is_valid_child():
		return
	var c: Control = get_child(0)
	c.position = Vector2.ZERO
	velocity = Vector2.ZERO
	child_position = Vector2.ZERO
	child_physics_position = Vector2.ZERO
	queue_sort()


func _get_configuration_warnings() -> PackedStringArray:
	var warnings: PackedStringArray
	if not is_valid_child():
		warnings.append("Must contain one Control child, and one only.")
	return warnings


func _notification(what):
	if not is_valid_child():
		return

	if what == NOTIFICATION_SORT_CHILDREN:
		var c: Control = get_child(0)

		var child_size := c.get_combined_minimum_size()

		if c.size_flags_horizontal & SizeFlags.SIZE_EXPAND:
			child_size.x = size.x
		if c.size_flags_vertical & SizeFlags.SIZE_EXPAND:
			child_size.y = size.y

		fit_child_in_rect(c, Rect2(child_position, child_size))


func _get_minimum_size() -> Vector2:
	if not is_valid_child():
		return Vector2.ZERO
	var c: Control = get_child(0)
	var min_size := Vector2()
	# TODO change take_hight_from_children
	# for a size_flag
	if take_hight_from_children:
		min_size.y = c.get_combined_minimum_size().y

	return min_size


# NOTE accept_event() on _gui_input is not preventing
# button presses while scrolling. Using it here instead.
# TODO prevent button presses on Editor
func _input(event: InputEvent) -> void:
	if not (is_scrolling_x or is_scrolling_y):
		return
	if event is InputEventScreenTouch:
		if not event.pressed:
			accept_event()
			is_touching = false
			is_scrolling_x = false
			is_scrolling_y = false
	elif event is InputEventMouseButton:
		if not event.pressed:
			accept_event()
			is_touching = false
			is_scrolling_x = false
			is_scrolling_y = false


func _gui_input(event: InputEvent) -> void:
	if not is_valid_child():
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			is_touching = true
			is_scrolling_x = false
			is_scrolling_y = false
			start_pos = event.position

			child_drag_position = child_position
			previous_position = child_position
		else:
			is_touching = false
			is_scrolling_x = false
			is_scrolling_y = false
			if is_outside_right(200):
				#TODO pass information
				scroll_ended.emit()
	elif event is InputEventScreenDrag:
		if is_touching:
			var offset: Vector2 = event.position - start_pos

			# Lock to the dominant axis once it exceeds the deadzone
			if not is_scrolling_x and not is_scrolling_y:
				var dominant_is_x: float = abs(offset.x) >= abs(offset.y)
				if dominant_is_x and abs(offset.x) > scroll_deadzone:
					if horizontal_scroll_mode != ScrollMode.DISABLED:
						is_scrolling_x = true
						start_pos = event.position
						child_drag_position = child_position
				elif not dominant_is_x and abs(offset.y) > scroll_deadzone:
					if vertical_scroll_mode != ScrollMode.DISABLED:
						is_scrolling_y = true
						start_pos = event.position
						child_drag_position = child_position

				if not is_scrolling_x and not is_scrolling_y:
					return

			offset = event.position - start_pos
			accept_event()

			if is_scrolling_x:
				child_position.x = child_drag_position.x + offset.x
				if is_outside_left():
					child_position.x = (child_drag_position.x + offset.x) * 0.2
				if is_outside_right():
					var excess: float = is_outside_right()
					child_position.x = (child_drag_position.x + offset.x) + excess
					child_position.x -= excess * 0.2
			if is_scrolling_y:
				child_position.y = child_drag_position.y + offset.y
				if is_outside_top():
					child_position.y = (child_drag_position.y + offset.y) * 0.2
				if is_outside_bottom():
					var excess: float = is_outside_bottom()
					child_position.y = (child_drag_position.y + offset.y) + excess
					child_position.y -= excess * 0.2
			queue_sort()


func _physics_process(delta: float) -> void:
	if not is_valid_child():
		return
	var c = get_child(0)
	if is_touching:
		# Store scroll velocity while user is touching
		# the screen (Swipe gesture)
		velocity = lerp(
			velocity, (child_position - previous_position) * Engine.physics_ticks_per_second, 1.0
		)
		previous_position = child_position
		child_physics_position = child_drag_position
	else:
		# If the user is not touching.
		# Use the stored velocity
		# to simulate inertia.
		velocity += force * delta
		# Simulate drag, so de velocity
		# goes down over time.
		velocity *= 1.0 - (drag / Engine.physics_ticks_per_second)

		# If the uer scrolled beyond of the child container
		# set a velocity to move the child container
		# inside the scroll container
		if is_outside_left():
			velocity.x = (0.0 - child_position.x) * rubber_force
		elif is_outside_right():
			velocity.x = ((c.size.x - size.x) - -child_position.x) * -rubber_force
		if is_outside_top():
			velocity.y = (0.0 - child_position.y) * rubber_force
		elif is_outside_bottom():
			velocity.y = ((c.size.y - size.y) - -child_position.y) * -rubber_force

		child_position += velocity * delta
		queue_sort()


func is_outside_top(margin: float = 0.0) -> float:
	return max(0, child_position.y - margin)


func is_outside_bottom(margin: float = 0.0) -> float:
	var c = get_child(0)
	if c.size.y < size.y:
		return max(0.0, -child_position.y - margin)
	return max(0.0, -child_position.y - (c.size.y - size.y) + margin)


func is_outside_left(margin: float = 0.0) -> float:
	return max(0.0, child_position.x - margin)


func is_outside_right(margin: float = 0.0) -> float:
	var c = get_child(0)
	if c.size.x < size.x:
		return max(0.0, -child_position.x - margin)
	return max(0.0, -child_position.x - (c.size.x - size.x) + margin)


func is_valid_child() -> bool:
	if get_child_count() != 1:
		return false
	var c = get_child(0)
	if c is not Control:
		return false
	return true
