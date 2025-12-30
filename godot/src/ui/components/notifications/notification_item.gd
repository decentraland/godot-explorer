extends PanelContainer

signal notification_clicked(notification: Dictionary)
signal mark_as_read_clicked(notification_id: String)

const TAP_THRESHOLD_TIME: int = 300  # milliseconds
const TAP_THRESHOLD_DISTANCE: float = 20.0  # pixels

var notification_data: Dictionary = {}
var original_bg_color: Color
var tween: Tween
var _press_start_time: int = 0
var _press_start_pos: Vector2 = Vector2.ZERO

@onready var notification_content: Control = %NotificationContent
@onready var label_timestamp: RichTextLabel = %LabelTimestamp
@onready var unread_dot: Panel = %UnreadDot


func _ready() -> void:
	set_process_input(true)

	# Store the original background color (resource is local to scene)
	var style_box = get_theme_stylebox("panel") as StyleBoxFlat
	if style_box:
		original_bg_color = style_box.bg_color


func set_notification(notification: Dictionary) -> void:
	notification_data = notification
	_update_ui()


func _update_ui() -> void:
	if notification_data.is_empty():
		return

	# Use shared notification content component
	notification_content.set_notification(notification_data)

	# Set timestamp (specific to list item view, bold)
	if "timestamp" in notification_data:
		var timestamp: int = int(notification_data["timestamp"])
		var time_text = _format_timestamp(timestamp)
		label_timestamp.text = "[b]" + time_text + "[/b]"

	# Show/hide unread dot (specific to list item view)
	var is_read: bool = notification_data.get("read", false)
	unread_dot.visible = not is_read


func _format_timestamp(timestamp_ms: int) -> String:
	var timestamp_sec = timestamp_ms / 1000
	var current_time = Time.get_unix_time_from_system()
	var diff = current_time - timestamp_sec

	if diff < 60:
		return "just now"

	if diff < 3600:
		var minutes = int(diff / 60)
		if minutes == 1:
			return "1 minute ago"
		return "%d minutes ago" % minutes

	if diff < 86400:
		var hours = int(diff / 3600)
		if hours == 1:
			return "1 hour ago"
		return "%d hours ago" % hours

	if diff < 172800:  # Less than 2 days (86400 * 2)
		return "yesterday"

	# For all older notifications, show days ago
	var days = int(diff / 86400)
	return "%d days ago" % days


func _input(event: InputEvent) -> void:
	# Only handle input when visible
	if not is_visible_in_tree():
		return

	# Only handle touch events (includes emulated touch from mouse)
	# Ignore mouse events to avoid duplicate processing
	if not (event is InputEventScreenTouch):
		return

	# Check if touch is within this item's rectangle
	var rect = get_global_rect()
	var is_inside = rect.has_point(event.position)

	if not is_inside:
		return

	if event.pressed:
		# Record press start
		_press_start_time = Time.get_ticks_msec()
		_press_start_pos = event.position
	else:
		# Check if this was a tap (quick press/release without much movement)
		var press_duration = Time.get_ticks_msec() - _press_start_time
		var press_distance = event.position.distance_to(_press_start_pos)

		if press_duration < TAP_THRESHOLD_TIME and press_distance < TAP_THRESHOLD_DISTANCE:
			# This is a tap, not a scroll
			_handle_tap()


func _handle_tap() -> void:
	Global.send_haptic_feedback()

	# Handle the tap action
	_play_click_animation()
	_track_notification_opened()
	notification_clicked.emit(notification_data)
	Global.notification_clicked.emit(notification_data)
	# Mark as read when tapped
	if "id" in notification_data:
		mark_as_read_clicked.emit(notification_data["id"])


func _track_notification_opened() -> void:
	# Track metric: notification opened from notifications panel
	var extra_properties = JSON.stringify(
		{"notification_id": notification_data.get("id", ""), "ui_source": "notif_menu"}
	)
	Global.metrics.track_click_button(
		"notification_opened", "NOTIFICATIONS_PANEL", extra_properties
	)


func _play_click_animation() -> void:
	# Get the local style box instance
	var style_box = get_theme_stylebox("panel") as StyleBoxFlat
	if not style_box:
		return

	# Cancel any existing tween
	if tween:
		tween.kill()

	# Instantly set to click color
	var click_color = Color("#43404A")
	style_box.bg_color = click_color

	# Animate back to original color
	tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_SINE)
	tween.tween_property(style_box, "bg_color", original_bg_color, 0.5)
