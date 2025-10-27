extends Control

signal toast_clicked(notification: Dictionary)
signal toast_closed
signal mark_as_read(notification: Dictionary)

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2
const DRAG_THRESHOLD = 15.0  # Minimum pixels to start dragging
const DISMISS_DISTANCE = 100.0  # Distance to fade completely and dismiss (reduced for easier dismissal)
const SNAP_BACK_DURATION = 0.3  # Duration to snap back to original position
const VELOCITY_DISMISS_THRESHOLD = 800.0  # Pixels per second to trigger inertia dismiss
const INERTIA_DURATION = 0.4  # Duration of inertia animation

enum DragDirection { NONE, LEFT, RIGHT, UP }

var notification_data: Dictionary = {}
var _timer: Timer

# Drag state
var _is_dragging: bool = false
var _drag_started: bool = false  # Track if drag was ever started
var _drag_start_pos: Vector2 = Vector2.ZERO
var _drag_current_pos: Vector2 = Vector2.ZERO
var _drag_previous_pos: Vector2 = Vector2.ZERO
var _drag_time: float = 0.0
var _drag_previous_time: float = 0.0
var _original_position: Vector2 = Vector2.ZERO
var _locked_direction: DragDirection = DragDirection.NONE
var _initial_modulate: Color = Color.WHITE
var _current_velocity: Vector2 = Vector2.ZERO

@onready var notification_content: Control = %NotificationContent
@onready var panel: PanelContainer = $Panel


func _ready() -> void:
	gui_input.connect(_on_gui_input)

	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)


func show_notification(notification: Dictionary) -> void:
	notification_data = notification
	notification_content.set_notification(notification)

	# Store initial modulate for fade effects
	_initial_modulate = panel.modulate

	# Start above screen (position the Control node)
	position.y = -size.y

	# Animate slide in from top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", 20.0, SLIDE_IN_DURATION)
	await tween.finished

	# Store original position after animation
	_original_position = position

	# Start auto-hide timer
	_timer.start(DISPLAY_DURATION)


func _on_timer_timeout() -> void:
	# Resume queue when auto-hiding from timer (don't emit next yet, dequeue will handle it)
	NotificationsManager.resume_queue()
	async_hide_toast()


func async_hide_toast() -> void:
	# Animate slide out to top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -size.y - 20.0, SLIDE_OUT_DURATION)
	await tween.finished
	print("[NotificationToast] Emitting toast_closed signal for notification: ", notification_data.get("id", "unknown"))
	toast_closed.emit()  # Emit signal before freeing
	queue_free()


func _on_gui_input(event: InputEvent) -> void:
	# Handle mouse button events
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				_start_drag(event.position)
			else:
				_end_drag(event.position)

	# Handle mouse motion for dragging
	elif event is InputEventMouseMotion:
		if _is_dragging or (_drag_start_pos != Vector2.ZERO and event.position.distance_to(_drag_start_pos) > DRAG_THRESHOLD):
			_update_drag(event.position)

	# Handle touch events
	elif event is InputEventScreenTouch:
		if event.pressed:
			_start_drag(event.position)
		else:
			_end_drag(event.position)

	# Handle touch drag events
	elif event is InputEventScreenDrag:
		if _is_dragging or (_drag_start_pos != Vector2.ZERO and event.position.distance_to(_drag_start_pos) > DRAG_THRESHOLD):
			_update_drag(event.position)


func _track_notification_opened() -> void:
	# Track metric: notification opened from HUD toast
	var extra_properties = JSON.stringify({
		"notification_id": notification_data.get("id", ""),
		"ui_source": "HUD"
	})
	Global.metrics.track_click_button("notification_opened", "HUD", extra_properties)


func _start_drag(pos: Vector2) -> void:
	# Prevent double-start
	if _drag_started:
		print("[NotificationToast] Ignoring duplicate drag start")
		return

	print("[NotificationToast] Drag started at position: ", pos)
	_drag_started = true
	_drag_start_pos = pos
	_drag_current_pos = pos
	_drag_previous_pos = pos
	_drag_time = Time.get_ticks_msec() / 1000.0
	_drag_previous_time = _drag_time
	_current_velocity = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	# Pause the auto-hide timer while dragging
	_timer.paused = true
	# Release focus to prevent camera rotation while dragging
	Global.explorer_release_focus()
	# Pause the notification queue to prevent new toasts from appearing
	NotificationsManager.pause_queue()


func _update_drag(pos: Vector2) -> void:
	# Update velocity tracking
	var current_time = Time.get_ticks_msec() / 1000.0
	var time_delta = current_time - _drag_previous_time

	if time_delta > 0:
		var pos_delta = pos - _drag_previous_pos
		_current_velocity = pos_delta / time_delta

	_drag_previous_pos = pos
	_drag_previous_time = current_time
	_drag_current_pos = pos
	var delta = _drag_current_pos - _drag_start_pos

	# Check if we've exceeded the drag threshold
	if delta.length() < DRAG_THRESHOLD:
		return

	# Lock to cardinal direction on first significant movement
	if _locked_direction == DragDirection.NONE:
		_is_dragging = true
		var abs_x = abs(delta.x)
		var abs_y = abs(delta.y)

		# Determine which direction has the largest movement
		if abs_x > abs_y:
			# Horizontal movement - determine left or right
			_locked_direction = DragDirection.LEFT if delta.x < 0 else DragDirection.RIGHT
			print("[NotificationToast] Direction locked to: ", "LEFT" if delta.x < 0 else "RIGHT")
		else:
			# Vertical movement - only allow upward
			if delta.y < 0:
				_locked_direction = DragDirection.UP
				print("[NotificationToast] Direction locked to: UP")
			else:
				# Don't allow downward drag - treat as cancelled drag
				print("[NotificationToast] Downward drag blocked - cancelling drag")
				_cancel_drag()
				return

	# Apply movement based on locked direction
	var offset = Vector2.ZERO
	match _locked_direction:
		DragDirection.LEFT:
			offset.x = min(0, delta.x)  # Only allow negative (left) movement
		DragDirection.RIGHT:
			offset.x = max(0, delta.x)  # Only allow positive (right) movement
		DragDirection.UP:
			offset.y = min(0, delta.y)  # Only allow negative (up) movement

	# Update position
	position = _original_position + offset

	# Calculate fade based on distance
	var distance = abs(offset.length())
	var fade_progress = clampf(distance / DISMISS_DISTANCE, 0.0, 1.0)

	# Update opacity (fade out as distance increases)
	panel.modulate = _initial_modulate
	panel.modulate.a = 1.0 - fade_progress


func _end_drag(pos: Vector2) -> void:
	# Prevent double-end or end without start
	if not _drag_started:
		print("[NotificationToast] Ignoring drag end - no drag was started")
		return

	var delta = pos - _drag_start_pos
	print("[NotificationToast] Drag ended at position: ", pos, ", total delta: ", delta, ", length: ", delta.length())

	# Mark drag as completed to prevent double execution
	_drag_started = false

	# Check if it was a tap (small movement)
	if delta.length() < DRAG_THRESHOLD:
		# It's a tap - trigger notification clicked
		print("[NotificationToast] Tap detected - emitting toast_clicked signal for notification: ", notification_data.get("id", "unknown"))
		# Resume queue (dequeue will handle showing next) and restore focus before hiding
		NotificationsManager.resume_queue()
		Global.explorer_grab_focus()
		_track_notification_opened()
		toast_clicked.emit(notification_data)
		Global.notification_clicked.emit(notification_data)
		async_hide_toast()
		return

	# Check velocity for inertia-based dismissal
	# Get velocity in the locked direction
	var directional_velocity = 0.0
	match _locked_direction:
		DragDirection.LEFT:
			directional_velocity = abs(_current_velocity.x) if _current_velocity.x < 0 else 0
		DragDirection.RIGHT:
			directional_velocity = abs(_current_velocity.x) if _current_velocity.x > 0 else 0
		DragDirection.UP:
			directional_velocity = abs(_current_velocity.y) if _current_velocity.y < 0 else 0

	print("[NotificationToast] Current velocity: ", _current_velocity, ", directional velocity: ", int(directional_velocity), " px/s")

	# Fast drag detected - apply inertia and dismiss
	if directional_velocity >= VELOCITY_DISMISS_THRESHOLD:
		print("[NotificationToast] Fast drag detected (velocity: ", int(directional_velocity), " px/s) - dismissing with inertia for notification: ", notification_data.get("id", "unknown"))
		_dismiss_with_inertia()
		return

	# Calculate final distance
	var distance = abs((_drag_current_pos - _drag_start_pos).length())
	var fade_progress = distance / DISMISS_DISTANCE
	print("[NotificationToast] Distance: ", int(distance), " px, fade progress: ", fade_progress)

	# Check if we've dragged far enough to dismiss (mark as read)
	if fade_progress >= 1.0:
		# Trigger mark as read
		print("[NotificationToast] Full drag detected - emitting mark_as_read signal for notification: ", notification_data.get("id", "unknown"))
		# Resume queue (dequeue will handle showing next) and restore focus before hiding
		NotificationsManager.resume_queue()
		Global.explorer_grab_focus()
		mark_as_read.emit(notification_data)
		async_hide_toast()
	else:
		# Snap back to original position
		print("[NotificationToast] Partial drag - snapping back, toast continues for notification: ", notification_data.get("id", "unknown"))
		# Resume queue (but toast continues), restore focus
		NotificationsManager.resume_queue()
		Global.explorer_grab_focus()
		_snap_back_to_original()

	# Reset drag state
	_is_dragging = false
	_drag_start_pos = Vector2.ZERO
	_drag_current_pos = Vector2.ZERO
	_drag_previous_pos = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	_current_velocity = Vector2.ZERO

	# Resume the auto-hide timer (only if we're not closing)
	if fade_progress < 1.0:
		_timer.paused = false


func _snap_back_to_original() -> void:
	# Animate back to original position
	var tween = create_tween()
	tween.set_parallel(true)
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position", _original_position, SNAP_BACK_DURATION)
	tween.tween_property(panel, "modulate", _initial_modulate, SNAP_BACK_DURATION)


func _cancel_drag() -> void:
	# Called when a drag is cancelled (e.g., downward drag blocked)
	# Resume queue and restore state
	NotificationsManager.resume_queue()
	Global.explorer_grab_focus()

	# Reset drag state
	_is_dragging = false
	_drag_started = false
	_drag_start_pos = Vector2.ZERO
	_drag_current_pos = Vector2.ZERO
	_drag_previous_pos = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	_current_velocity = Vector2.ZERO

	# Resume the auto-hide timer
	_timer.paused = false


func _dismiss_with_inertia() -> void:
	print("[NotificationToast] Starting inertia dismissal animation")
	# Dismiss notification with momentum animation
	# Mark as read and restore state
	NotificationsManager.resume_queue()
	Global.explorer_grab_focus()
	mark_as_read.emit(notification_data)

	# Calculate target position based on direction and add extra distance for momentum
	var target_offset = Vector2.ZERO
	var momentum_distance = 300.0  # Extra distance to travel with momentum

	match _locked_direction:
		DragDirection.LEFT:
			target_offset.x = -momentum_distance
		DragDirection.RIGHT:
			target_offset.x = momentum_distance
		DragDirection.UP:
			target_offset.y = -momentum_distance

	var target_position = position + target_offset

	# Reset drag state
	_is_dragging = false
	_drag_start_pos = Vector2.ZERO
	_drag_current_pos = Vector2.ZERO
	_drag_previous_pos = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	_current_velocity = Vector2.ZERO

	# Animate with easing for momentum feel
	var tween = create_tween()
	tween.set_parallel(true)
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_CUBIC)
	tween.tween_property(self, "position", target_position, INERTIA_DURATION)
	tween.tween_property(panel, "modulate:a", 0.0, INERTIA_DURATION)

	await tween.finished
	print("[NotificationToast] Emitting toast_closed signal for notification: ", notification_data.get("id", "unknown"))
	toast_closed.emit()
	queue_free()
