extends Control

signal toast_clicked(notification: Dictionary)
signal toast_closed
signal mark_as_read(notification: Dictionary)

enum DragDirection { NONE, LEFT, RIGHT, UP }

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2
const DRAG_THRESHOLD = 15.0  # Minimum pixels to distinguish tap from swipe
const VELOCITY_DISMISS_THRESHOLD = 800.0  # Pixels per second to trigger swipe dismiss
const INERTIA_DURATION = 0.4  # Duration of inertia animation

var notification_data: Dictionary = {}
var _timer: Timer

# Swipe state
var _swipe_started: bool = false  # Track if swipe was ever started
var _swipe_start_pos: Vector2 = Vector2.ZERO
var _swipe_previous_pos: Vector2 = Vector2.ZERO
var _swipe_previous_time: float = 0.0
var _locked_direction: DragDirection = DragDirection.NONE
var _current_velocity: Vector2 = Vector2.ZERO

@onready var notification_content: Control = %NotificationContent
@onready var panel: PanelContainer = $Panel


func _ready() -> void:
	gui_input.connect(_on_gui_input)

	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)


func async_show_notification(notification: Dictionary) -> void:
	notification_data = notification
	notification_content.set_notification(notification)

	# Start above screen (position the Control node)
	position.y = -size.y

	# Animate slide in from top (align visible panel with profile button at 20px from top)
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -15.0, SLIDE_IN_DURATION)
	await tween.finished

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
#	print(
#		"[NotificationToast] Emitting toast_closed signal for notification: ",
#		notification_data.get("id", "unknown")
#	)
	toast_closed.emit()  # Emit signal before freeing
	queue_free()


func _on_gui_input(event: InputEvent) -> void:
	# Handle mouse button events
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				_start_swipe(event.position)
			else:
				_end_swipe(event.position)

	# Handle mouse motion for velocity tracking
	elif event is InputEventMouseMotion:
		if _swipe_started:
			_update_velocity(event.position)

	# Handle touch events
	elif event is InputEventScreenTouch:
		if event.pressed:
			_start_swipe(event.position)
		else:
			_end_swipe(event.position)

	# Handle touch drag events for velocity tracking
	elif event is InputEventScreenDrag:
		if _swipe_started:
			_update_velocity(event.position)


func _track_notification_opened() -> void:
	# Track metric: notification opened from HUD toast
	var extra_properties = JSON.stringify(
		{"notification_id": notification_data.get("id", ""), "ui_source": "HUD"}
	)
	Global.metrics.track_click_button("notification_opened", "HUD", extra_properties)


func _start_swipe(pos: Vector2) -> void:
	# Prevent double-start
	if _swipe_started:
		return

	_swipe_started = true
	_swipe_start_pos = pos
	_swipe_previous_pos = pos
	_swipe_previous_time = Time.get_ticks_msec() / 1000.0
	_current_velocity = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	# Pause the auto-hide timer while tracking swipe
	_timer.paused = true
	# Release focus to prevent camera rotation while swiping
	Global.explorer_release_focus()
	# Pause the notification queue to prevent new toasts from appearing
	NotificationsManager.pause_queue()


func _update_velocity(pos: Vector2) -> void:
	# Update velocity tracking
	var current_time = Time.get_ticks_msec() / 1000.0
	var time_delta = current_time - _swipe_previous_time

	if time_delta > 0:
		var pos_delta = pos - _swipe_previous_pos
		_current_velocity = pos_delta / time_delta

	_swipe_previous_pos = pos
	_swipe_previous_time = current_time

	# Lock to cardinal direction on first significant movement
	if _locked_direction == DragDirection.NONE:
		var delta = pos - _swipe_start_pos
		if delta.length() >= DRAG_THRESHOLD:
			var abs_x = abs(delta.x)
			var abs_y = abs(delta.y)

			# Determine which direction has the largest movement
			if abs_x > abs_y:
				# Horizontal movement - determine left or right
				_locked_direction = DragDirection.LEFT if delta.x < 0 else DragDirection.RIGHT
#				print(
#					"[NotificationToast] Direction locked to: ", "LEFT" if delta.x < 0 else "RIGHT"
#				)
#			else:
				# Vertical movement - only allow upward
				if delta.y < 0:
					_locked_direction = DragDirection.UP
				else:
					# Don't allow downward swipe - treat as cancelled
					_cancel_swipe()


func _end_swipe(pos: Vector2) -> void:
	# Prevent double-end or end without start
	if not _swipe_started:
		return

	var delta = pos - _swipe_start_pos
#	print(
#		"[NotificationToast] Swipe ended at position: ",
#		pos,
#		", total delta: ",
#		delta,
#		", length: ",
#		delta.length()
#	)

	# Mark swipe as completed to prevent double execution
	_swipe_started = false

	# Check if it was a tap (small movement)
	if delta.length() < DRAG_THRESHOLD:
		# It's a tap - trigger notification clicked
#		print(
#			"[NotificationToast] Tap detected - emitting toast_clicked signal for notification: ",
#			notification_data.get("id", "unknown")
#		)
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

#	print(
#		"[NotificationToast] Current velocity: ",
#		_current_velocity,
#		", directional velocity: ",
#		int(directional_velocity),
#		" px/s"
#	)

	# Fast swipe detected - apply inertia and dismiss
	if directional_velocity >= VELOCITY_DISMISS_THRESHOLD:
#		print(
#			"[NotificationToast] Fast swipe detected (velocity: ",
#			int(directional_velocity),
#			" px/s) - dismissing with inertia for notification: ",
#			notification_data.get("id", "unknown")
#		)
		async_dismiss_with_inertia()
		return

	# Not a fast swipe - just resume timer
#	print(
#		"[NotificationToast] Slow swipe detected - toast continues for notification: ",
#		notification_data.get("id", "unknown")
#	)
#	# Resume queue and restore focus
	NotificationsManager.resume_queue()
	Global.explorer_grab_focus()

	# Reset swipe state
	_swipe_start_pos = Vector2.ZERO
	_swipe_previous_pos = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	_current_velocity = Vector2.ZERO

	# Resume the auto-hide timer
	_timer.paused = false


func _cancel_swipe() -> void:
	# Called when a swipe is cancelled (e.g., downward swipe blocked)
	# Resume queue and restore state
	NotificationsManager.resume_queue()
	Global.explorer_grab_focus()

	# Reset swipe state
	_swipe_started = false
	_swipe_start_pos = Vector2.ZERO
	_swipe_previous_pos = Vector2.ZERO
	_locked_direction = DragDirection.NONE
	_current_velocity = Vector2.ZERO

	# Resume the auto-hide timer
	_timer.paused = false


func async_dismiss_with_inertia() -> void:
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

	# Reset swipe state
	_swipe_start_pos = Vector2.ZERO
	_swipe_previous_pos = Vector2.ZERO
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
#	print(
#		"[NotificationToast] Emitting toast_closed signal for notification: ",
#		notification_data.get("id", "unknown")
#	)
#	toast_closed.emit()
#	queue_free()
