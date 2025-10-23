extends PanelContainer

signal notification_clicked(notification: Dictionary)
signal mark_as_read_clicked(notification_id: String)

var notification_data: Dictionary = {}

@onready var notification_content: Control = %NotificationContent
@onready var label_timestamp: RichTextLabel = %LabelTimestamp
@onready var unread_dot: Panel = %UnreadDot


func _ready() -> void:
	gui_input.connect(_on_gui_input)


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


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			notification_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			# Mark as read when clicked
			if "id" in notification_data:
				mark_as_read_clicked.emit(notification_data["id"])
	elif event is InputEventScreenTouch:
		# Only handle on release (not pressed) to allow drag events for scrolling
		if not event.pressed:
			notification_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			# Mark as read when clicked
			if "id" in notification_data:
				mark_as_read_clicked.emit(notification_data["id"])
