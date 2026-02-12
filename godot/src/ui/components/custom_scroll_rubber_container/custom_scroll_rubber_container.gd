@tool
class_name ScrollRubberContainer
extends Container

signal scroll_ended

@export_category("ScrollContainer properties")
@export var follow_focus: bool ## Unused property
@export var draw_focus_border: bool ## Unused property
@export var horizontal_scroll_mode := ScrollMode.Auto
@export var vertical_scroll_mode := ScrollMode.Auto
@export var scroll_deadzone := 0 ## Unused property

@export_category("Rubber Band behavior")
@export var drag := 0.8
@export var rubber_force := 8.0
@export var take_hight_from_children: bool = false

enum ScrollMode {
	Disabled, ## Wont scroll in this direction
	Auto, ## Unused property
	AlwaysShow, ## Unused property
	NeverShow, ## Unused property
	Reserve ## Unused property
}

var start_pos: Vector2
var child_position: Vector2
var child_drag_position: Vector2
var child_physics_position: Vector2
var drag_tween: Tween
var is_touching := false
var is_scrolling := false

var velocity: Vector2
var force: Vector2
var previous_position: Vector2


#func _ready() -> void:
#	size_flags_vertical = Control.SIZE_EXPAND_FILL


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
	"""
	if what == NOTIFICATION_RESIZED:
		if get_parent_control():
			size.y = get_parent_control().size.y
			size.y -= get_parent_control().get_theme_constant("theme_override_constants", "margin_top")
	"""
	
	if what == NOTIFICATION_SORT_CHILDREN:
		var c: Control = get_child(0)

		var child_size := c.get_combined_minimum_size()
		#custom_minimum_size.x = child_min_size.x
		#if c.size_flags_vertical & SizeFlags.SIZE_FILL:
		#	size.y = child_min_size.y

		#if false:
		#	size.y = child_min_size.y
		#	if size.y > get_rect().size.y:
		#		size.y = get_rect().size.y

		if c.size_flags_horizontal & SizeFlags.SIZE_EXPAND:
			child_size.x = size.x
		if c.size_flags_vertical & SizeFlags.SIZE_EXPAND:
			child_size.y = size.y

		#if child_min_size.y < size.y:
			#child_min_size.y = size.y
		#child_min_size = Vector2(50,50)
		
		fit_child_in_rect(c, Rect2(child_position, child_size))


func _get_minimum_size() -> Vector2:
	if not is_valid_child():
		return Vector2.ZERO
	var c: Control = get_child(0)
	var min_size := Vector2()
	if take_hight_from_children:
		min_size.y = c.get_combined_minimum_size().y
	
	return min_size


# NOTE accept_event() on _gui_input is not preventing
# button presses while scrolling. Using it here instead.
# TODO prevent button presses on Editor
func _input(event: InputEvent) -> void:
	if not is_scrolling:
		return
	if event is InputEventScreenTouch:
		if not event.pressed:
			accept_event()
			is_touching = false
			is_scrolling = false
	elif event is InputEventMouseButton:
		if not event.pressed:
			accept_event()
			is_touching = false
			is_scrolling = false


func _gui_input(event: InputEvent) -> void:
	if not is_valid_child():
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			is_touching = true
			is_scrolling = false
			start_pos = event.position
			
			child_drag_position = child_position
			previous_position = child_position
		else:
			is_touching = false
			is_scrolling = false
			if is_outside_right(200):
				#TODO pass information
				scroll_ended.emit()
	elif event is InputEventScreenDrag:
		if is_touching:
			var offset:Vector2 = event.position - start_pos
			
			if horizontal_scroll_mode != ScrollMode.Disabled:
				child_position.x = child_drag_position.x + offset.x
			if vertical_scroll_mode != ScrollMode.Disabled:
				child_position.y = child_drag_position.y + offset.y
			
			if is_outside_right():
				child_position.x = (child_drag_position.x + offset.x) * 0.5
			
			if start_pos.distance_to(event.position) > 50:
				is_scrolling = true
			queue_sort()
		else:
			print("Not touching")


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


func is_outside_top(margin: float = 0.0) -> bool:
	return child_position.y > margin


func is_outside_bottom(margin: float = 0.0) -> bool:
	var c = get_child(0)
	if c.size.y < size.y:
		return -child_position.y > margin
	return -child_position.y > (c.size.y - size.y) + margin


func is_outside_left(margin: float = 0.0) -> bool:
	return child_position.x > margin


func is_outside_right(margin: float = 0.0) -> bool:
	var c = get_child(0)
	if c.size.x < size.x:
		return -child_position.x > margin
	return -child_position.x > (c.size.x - size.x) + margin


func is_valid_child() -> bool:
	if get_child_count() != 1:
		return false
	var c = get_child(0)
	if c is not Control:
		return false
	return true
