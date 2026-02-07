extends Button

const NOTIFICATION_ADVANCE_MINUTES = 3  # Notify 3 minutes before event starts

# DEBUG: Set to true to trigger notifications in 10 seconds instead of actual event time
const DEBUG_TRIGGER_IN_10_SECONDS = false
const COLOR_PRESSED = Color("#FD4766")
const COLOR_NORMAL = Color("#FCFCFC")
const COLOR_WHILE = Color("#CFCDD4")

var bell_texture = load("res://assets/ui/bell.svg")
var check_texture = load("res://assets/ui/check.svg")
var event_id_value: String
var event_tags: String
var event_start_timestamp: int = 0  # Unix timestamp (seconds) when event starts
var event_name: String = ""
var event_coordinates: Vector2i = Vector2i(0, 0)
var event_cover_image_url: String = ""

@onready var texture_rect_icon: TextureRect = %TextureRect_Icon
@onready var label: Label = %Label


func _ready() -> void:
	_set_loading(false)


func _async_on_toggled(toggled_on: bool) -> void:
	if event_id_value == null:
		printerr("NO ID")
		set_pressed_no_signal(!toggled_on)
		return

	_set_loading(true)

	var url = DclUrls.events_api() + "/" + event_id_value + "/attendees"
	var method: HTTPClient.Method

	if toggled_on:
		method = HTTPClient.METHOD_POST
		Global.metrics.track_click_button(
			"reminder_set",
			"EVENT_DETAILS",
			JSON.stringify({"event_id": event_id_value, "event_tag": event_tags})
		)
	else:
		method = HTTPClient.METHOD_DELETE
		Global.metrics.track_click_button(
			"reminder_remove",
			"EVENT_DETAILS",
			JSON.stringify({"event_id": event_id_value, "event_tag": event_tags})
		)

	var response = await Global.async_signed_fetch(url, method)
	if response is PromiseError:
		printerr("Error unpdating attend intention: ", response.get_error())
		set_pressed_no_signal(!toggled_on)
	elif response != null:
		update_styles(toggled_on)

		# Handle local notification scheduling/cancellation
		if toggled_on:
			_async_schedule_local_notification()
		else:
			_cancel_local_notification()
	else:
		set_pressed_no_signal(!toggled_on)
		printerr("Error unpdating attend intention")

	_set_loading(false)


func _set_loading(status: bool) -> void:
	disabled = status
	texture_rect_icon.texture = bell_texture
	texture_rect_icon.modulate = COLOR_WHILE
	label.label_settings.font_color = COLOR_WHILE
	if status == false:
		update_styles(button_pressed)


func update_styles(toggled_on):
	var guest_profile := Global.player_identity.is_guest
	if guest_profile:
		disabled = true
		label.text = "SIGN IN TO USE REMINDERS"
		texture_rect_icon = null
	else:
		disabled = false
		label.text = "REMIND ME"
		if toggled_on:
			texture_rect_icon.texture = check_texture
			texture_rect_icon.modulate = COLOR_PRESSED
			label.label_settings.font_color = COLOR_PRESSED
		else:
			texture_rect_icon.texture = bell_texture
			texture_rect_icon.modulate = COLOR_NORMAL
			label.label_settings.font_color = COLOR_NORMAL


func _async_schedule_local_notification() -> void:
	# Validate event data
	if event_id_value.is_empty():
		printerr("Cannot schedule notification: event_id is empty")
		return

	if event_name.is_empty():
		printerr("Cannot schedule notification: event name is empty")
		return

	# Check and request notification permission
	if not NotificationsManager.has_local_notification_permission():
		NotificationsManager.request_local_notification_permission()

		# Check permission after request
		# Note: On iOS this is async, but we'll try to schedule anyway
		# If permission is denied, the OS will handle it gracefully
		if not NotificationsManager.has_local_notification_permission():
			printerr("Notification permission not granted yet, scheduling anyway (OS will handle)")

	# Calculate trigger time
	var current_time = Time.get_unix_time_from_system()
	var notification_trigger_time: int

	if DEBUG_TRIGGER_IN_10_SECONDS:
		# DEBUG MODE: Trigger in 10 seconds for testing
		notification_trigger_time = int(current_time) + 10
	else:
		# PRODUCTION MODE: Trigger 3 minutes before event starts
		if event_start_timestamp <= 0:
			printerr(
				(
					"Cannot schedule notification: invalid event start timestamp. event_id=%s, timestamp=%d"
					% [event_id_value, event_start_timestamp]
				)
			)
			return

		notification_trigger_time = event_start_timestamp - (NOTIFICATION_ADVANCE_MINUTES * 60)

		# Validate that trigger time is in the future
		if notification_trigger_time <= current_time:
			printerr(
				"Cannot schedule notification: trigger time is in the past. Event starts at: ",
				event_start_timestamp,
				", current time: ",
				current_time
			)
			return

	# Generate notification ID based on event ID
	var notification_id = "event_" + event_id_value

	# Generate random title and description
	var notification_text = NotificationsManager.generate_event_notification_text(event_name)
	var notification_title = notification_text["title"]
	var notification_body = notification_text["body"]

	# Construct deep link for event location
	var deep_link = (
		"decentraland://open?position=%d,%d" % [event_coordinates.x, event_coordinates.y]
	)

	# Queue the notification with image and deep link
	var success = await NotificationsManager.async_queue_local_notification(
		notification_id,
		notification_title,
		notification_body,
		notification_trigger_time,
		event_cover_image_url,
		deep_link
	)

	if not success:
		printerr("Failed to schedule notification for event: ", event_id_value)


func _cancel_local_notification() -> void:
	if event_id_value.is_empty():
		return

	var notification_id = "event_" + event_id_value

	# Cancel returns false if notification doesn't exist, which is fine
	NotificationsManager.cancel_queued_local_notification(notification_id)
