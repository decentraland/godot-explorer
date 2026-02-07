@tool
class_name ScrollRubberContainer
extends Container

signal request(offsett: int, limit: int)

@export var scroll_horizontal := true
@export var scroll_vertical := true
@export var scroll_deadzone := 0.0
@export var drag := 0.8
@export var item_container: Container = null

var start_pos: Vector2
var offset: Vector2
var child_drag_position: Vector2
var child_physics_position: Vector2
var drag_tween: Tween
var is_touching := false
var is_scrolling_mouse := false
var is_scrolling_touch := false

var velocity: Vector2
var force: Vector2
var previous_position: Vector2

var current_offset = 0
var threshold_limit = 10


func start():
	#self.scroll_ended.connect(self._on_scroll_container_scroll_ended)
	emit_request()


func restart():
	current_offset = 0
	emit_request()


func emit_request():
	request.emit(current_offset, threshold_limit)


func _on_scroll_container_scroll_ended():
	if false:
		emit_request()


func _get_configuration_warnings() -> PackedStringArray:
	var warnings: PackedStringArray
	if not is_valid_child():
		warnings.append("Must contain one Control child, and one only.")
	return warnings


func _notification(what):
	if not is_valid_child(): return
	if what == NOTIFICATION_SORT_CHILDREN:
		var c: Control = get_child(0)
		var child_position: Vector2
		if scroll_horizontal:
			child_position.x += offset.x + child_physics_position.x
		if scroll_vertical:
			child_position.y += offset.y + child_physics_position.y
		var child_min_size := c.get_combined_minimum_size()
		custom_minimum_size.y = child_min_size.y
		fit_child_in_rect(c, Rect2(child_position, child_min_size))

"""
## Destroy all events inside ScrollRubber
## that are unrelated to scrolling
func _input(event: InputEvent) -> void:
	if not event.get("position"): return
	if not get_global_rect().has_point(event.position):
		return
	if event is InputEventMouseButton:
		if event.is_pressed:
			is_scrolling_mouse = false
		if is_scrolling_mouse:
			accept_event()
			print("Destroy event mouse!")
	if event is InputEventScreenTouch:
		if is_scrolling_touch:
			accept_event()
			print("Destroy event touch!")
		if not event.pressed:
			is_touching = false
			is_scrolling_mouse = false
"""

## Cancelling on _gui_input is not working

func _gui_input(event: InputEvent) -> void:
	accept_event()
	get_viewport().set_input_as_handled()
	if not is_valid_child(): return
	var c: Control = get_child(0)
	c.accept_event()
	if event is InputEventMouseButton:
		if event.pressed:
			is_scrolling_mouse = false
			accept_event()
			print("Mouse pressed")
		else:
			if is_scrolling_mouse:
				accept_event()
			is_scrolling_mouse = false
	if event is InputEventScreenTouch:
		if event.pressed:
			accept_event()
			is_touching = true
			is_scrolling_touch = false
			start_pos = event.position
			child_drag_position = c.position
			previous_position = c.position
			print("Touch pressed")
		else:
			if is_scrolling_touch:
				accept_event()
			is_touching = false
			is_scrolling_touch = false
	elif event is InputEventScreenDrag:
		if is_touching:
			offset = event.position - start_pos
			if offset.length() > 50:
				is_scrolling_mouse = true
				is_scrolling_touch = true
			queue_sort()


func _physics_process(delta: float) -> void:
	if not is_valid_child(): return
	var c = get_child(0)
	if is_touching:
		velocity = lerp(velocity, (c.position - previous_position) * Engine.physics_ticks_per_second, 1.0)
		previous_position = c.position
		child_physics_position = child_drag_position
	else:
		force = -c.position * 10.0
		velocity += force * delta
		velocity *= 1.0 - (drag / Engine.physics_ticks_per_second)
		child_physics_position += velocity * delta
		queue_sort()


func tween_to(drag_position: Vector2) -> void:
	if drag_tween and drag_tween.is_running():
		drag_tween.stop()
		drag_tween = null
	drag_tween = create_tween().set_trans(Tween.TRANS_QUART)
	drag_tween.tween_property(self, "position", drag_position, 0.2)


func is_valid_child() -> bool:
	if get_child_count() != 1: return false
	var c = get_child(0)
	if c is not Control: return false
	return true
