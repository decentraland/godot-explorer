extends Node

## NotificationsManager
##
## Autoload script for managing Decentraland notifications.
## Handles fetching, marking as read, and polling for new notifications.
## Also provides a unified API for local notifications on Android and iOS.

signal new_notifications(notifications: Array)
signal notifications_updated
signal notification_error(error_message: String)
signal notification_queued(notification: Dictionary)

# Local notification signals
signal local_notification_permission_changed(granted: bool)
signal local_notification_scheduled(notification_id: String)
signal local_notification_cancelled(notification_id: String)

const BASE_URL = "https://notifications.decentraland.org"
const POLL_INTERVAL_SECONDS = 30.0  # Poll every 30 seconds

## TESTING: Set to true to inject fake notifications for testing
const ENABLE_FAKE_NOTIFICATIONS = false

## TESTING: Set to false to disable type filtering and show all notifications
const ENABLE_NOTIFICATION_FILTER = true

## DEBUG: Set to true to enable random notification generation for testing
const ENABLE_DEBUG_RANDOM_NOTIFICATIONS = false

## Supported notification types (whitelist)
## Only these types will be shown to the user (systems that are implemented)
const SUPPORTED_NOTIFICATION_TYPES = [
	"event_created",  # Events: New event created
	"events_starts_soon",  # Events: Event starts soon
	"events_started",  # Events: Event has started
	"reward_assignment",  # Rewards: Reward assigned/received
	"reward_in_progress",  # Rewards: Reward being processed
]

var _notifications: Array = []
var _poll_timer: Timer = null
var _is_polling: bool = false
var _previous_notification_ids: Array = []
var _notification_queue: Array = []  # Queue for new unread notifications to show as toasts
var _debug_timer: Timer = null  # Timer for debug random notifications
var _queue_paused: bool = false  # Whether the notification queue is paused

# Local notifications
var _android_plugin = null
var _ios_plugin = null
var _local_notification_channel_id = "dcl_local_notifications"
var _local_notification_channel_name = "Decentraland Notifications"
var _local_notification_channel_description = "Local notifications for Decentraland events"

# Queue management constants (Phase 3)
const MAX_OS_SCHEDULED_NOTIFICATIONS = 24  # Maximum notifications scheduled with OS at once


func _ready() -> void:
	# Create polling timer
	_poll_timer = Timer.new()
	_poll_timer.wait_time = POLL_INTERVAL_SECONDS
	_poll_timer.timeout.connect(_on_poll_timeout)
	add_child(_poll_timer)

	# Create debug random notification timer
	if ENABLE_DEBUG_RANDOM_NOTIFICATIONS:
		_debug_timer = Timer.new()
		_debug_timer.one_shot = false
		_debug_timer.timeout.connect(_on_debug_timer_timeout)
		add_child(_debug_timer)
		_start_debug_timer()

	# Initialize platform-specific plugins
	_initialize_local_notifications()

	# Clear any badge from previous notifications
	clear_badge_and_delivered_notifications()

	# Initial queue sync on app launch (relaunch)
	_sync_notification_queue()


## Start polling for new notifications
func start_polling() -> void:
	# Don't poll for guests
	if not _is_user_authenticated():
		return

	if not _is_polling:
		_is_polling = true
		_poll_timer.start()
		# Fetch immediately
		fetch_notifications(-1, 50, false)


## Stop polling for notifications
func stop_polling() -> void:
	if _is_polling:
		_is_polling = false
		_poll_timer.stop()


## Get currently cached notifications
func get_notifications() -> Array:
	return _notifications.duplicate()


## Filter notifications to only include supported types
func _filter_notifications(notifications: Array) -> Array:
	# If filtering is disabled, return all notifications
	if not ENABLE_NOTIFICATION_FILTER:
		return notifications

	var filtered = []

	for notif in notifications:
		if notif is Dictionary and "type" in notif:
			var notif_type = notif["type"]
			if notif_type in SUPPORTED_NOTIFICATION_TYPES:
				filtered.append(notif)

	return filtered


## Generate a fake notification for testing
func _generate_fake_notification() -> Dictionary:
	var fake_types = [
		{
			"type": "item_sold",
			"title": "Item Sold!",
			"description": "Your item 'Cool Wearable' sold for 100 MANA"
		},
		{
			"type": "bid_accepted",
			"title": "Bid Accepted",
			"description": "Your bid of 50 MANA was accepted"
		},
		{
			"type": "governance_announcement",
			"title": "DAO Announcement",
			"description": "New governance proposal is now live"
		},
		{
			"type": "land",
			"title": "Land Update",
			"description": "Your LAND at 52,-52 has been updated"
		},
		{
			"type": "worlds_permission_granted",
			"title": "Permission Granted",
			"description": "You now have access to World XYZ"
		},
	]

	var random_type = fake_types[randi() % fake_types.size()]
	var timestamp = Time.get_unix_time_from_system() * 1000  # milliseconds

	return {
		"id": "fake_notification_" + str(timestamp) + "_" + str(randi()),
		"type": random_type["type"],
		"address": "0x1234567890abcdef",
		"timestamp": int(timestamp),
		"read": false,
		"metadata":
		{
			"title": random_type["title"],
			"description": random_type["description"],
			"link": "https://decentraland.org"
		}
	}


## Fetch notifications from the API
##
## @param from_timestamp: Unix timestamp (ms) to fetch from, or -1 for all
## @param limit: Max number of notifications (1-50), or -1 for default
## @param only_unread: If true, only fetch unread notifications
## @returns: Promise that resolves with notifications array
func fetch_notifications(
	from_timestamp: int = -1, limit: int = -1, only_unread: bool = false
) -> Promise:
	var promise := Promise.new()

	# Don't fetch for guests
	if not _is_user_authenticated():
		promise.reject("User not authenticated")
		return promise

	# Build query parameters
	var query_params: Array = []
	if from_timestamp >= 0:
		query_params.append("from=%d" % from_timestamp)
	if limit > 0:
		var clamped_limit = mini(limit, 50)
		query_params.append("limit=%d" % clamped_limit)
	if only_unread:
		query_params.append("onlyUnread=true")

	var query_string = ""
	if query_params.size() > 0:
		query_string = "?" + "&".join(query_params)

	var url = BASE_URL + "/notifications" + query_string

	# Execute async fetch in a coroutine
	_async_fetch_notifications(promise, url)

	return promise


## Internal async helper for fetching notifications
func _async_fetch_notifications(promise: Promise, url: String) -> void:
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "{}")

	if response is PromiseError:
		var error_msg = "Fetch error: " + response.get_error()
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	var data: Dictionary = response.get_string_response_as_json()

	if not (data is Dictionary and "notifications" in data):
		var error_msg = "Invalid response format"
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	var notifications = data["notifications"]

	var filtered_notifications = _filter_notifications(notifications)

	# TESTING: Inject fake notification for testing toast popups
	if ENABLE_FAKE_NOTIFICATIONS:
		var fake_notif = _generate_fake_notification()
		filtered_notifications.append(fake_notif)

	# Merge new notifications with existing ones (avoid duplicates by ID)
	var existing_ids = {}
	for notif in _notifications:
		if "id" in notif:
			existing_ids[notif["id"]] = true

	# Detect new notifications and queue them
	var new_notifs: Array = []
	for notif in filtered_notifications:
		if "id" in notif:
			var notif_id = notif["id"]
			# Check if this is a new notification not in our existing set
			if notif_id not in existing_ids:
				# Add to notifications list
				_notifications.append(notif)
				existing_ids[notif_id] = true

				# Queue unread notifications for toast display
				if not notif.get("read", false):
					new_notifs.append(notif)
					_notification_queue.append(notif)

	if new_notifs.size() > 0:
		# Emit signal to start processing queue
		if _notification_queue.size() == new_notifs.size():  # Only trigger if queue was empty
			notification_queued.emit(_notification_queue[0])

	# Emit updated notifications list
	new_notifications.emit(_notifications.duplicate())
	notifications_updated.emit()
	promise.resolve_with_data(_notifications.duplicate())


## Mark notifications as read
##
## @param notification_ids: Array of notification ID strings to mark as read
## @returns: Promise that resolves with number of notifications updated
func mark_as_read(notification_ids: PackedStringArray) -> Promise:
	var promise := Promise.new()

	if notification_ids.size() == 0:
		promise.reject("No notification IDs provided")
		return promise

	if not Global.player_identity:
		promise.reject("Player identity not available")
		return promise

	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		promise.reject("User not authenticated")
		return promise

	var url = BASE_URL + "/notifications/read"
	var body = {"notificationIds": Array(notification_ids)}
	var body_json = JSON.stringify(body)

	_async_mark_as_read(promise, url, body_json, notification_ids)

	return promise


## Internal async helper for marking notifications as read
func _async_mark_as_read(
	promise: Promise, url: String, body_json: String, notification_ids: PackedStringArray
) -> void:
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PUT, body_json)

	if response is PromiseError:
		var error_msg = "Mark as read error: " + response.get_error()
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	var data: Dictionary = response.get_string_response_as_json()

	if not (data is Dictionary and "updated" in data):
		var error_msg = "Invalid response format"
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	var updated_count = data["updated"]

	for notif in _notifications:
		if notif["id"] in notification_ids:
			notif["read"] = true

	notifications_updated.emit()
	promise.resolve_with_data(updated_count)


func _on_poll_timeout() -> void:
	if _is_polling:
		fetch_notifications(-1, 50, true)


## Get the next notification from the queue
func get_next_queued_notification() -> Dictionary:
	if _notification_queue.size() > 0:
		return _notification_queue[0]
	return {}


## Remove the first notification from the queue and return the next one
func dequeue_notification() -> Dictionary:
	if _notification_queue.size() > 0:
		_notification_queue.pop_front()

		# Return next notification if available and queue is not paused
		if _notification_queue.size() > 0 and not _queue_paused:
			var next_notif = _notification_queue[0]
			# Emit signal for next notification
			notification_queued.emit(next_notif)
			return next_notif

	return {}


## Check if there are notifications in the queue
func has_queued_notifications() -> bool:
	return _notification_queue.size() > 0


## Get the number of notifications in the queue
func get_queue_size() -> int:
	return _notification_queue.size()


## Pause the notification queue (prevents new toasts from showing)
func pause_queue() -> void:
	_queue_paused = true


## Resume the notification queue (allows new toasts to show)
## @param emit_next: If true, immediately emits the next notification signal (default: false)
func resume_queue(emit_next: bool = false) -> void:
	_queue_paused = false
	# Only emit if explicitly requested (for when toast is dismissed)
	if emit_next and _notification_queue.size() > 0:
		notification_queued.emit(_notification_queue[0])


## Check if user is authenticated (not a guest)
func _is_user_authenticated() -> bool:
	if not Global.player_identity:
		return false
	var address = Global.player_identity.get_address_str()
	return not address.is_empty()


## DEBUG: Start the debug timer with a random interval between 7-10 seconds
func _start_debug_timer() -> void:
	if _debug_timer:
		var random_interval = randf_range(7.0, 10.0)
		_debug_timer.wait_time = random_interval
		_debug_timer.start()


## DEBUG: Called when debug timer times out - requeues a random old notification
func _on_debug_timer_timeout() -> void:
	# Pick a random notification from existing ones to requeue
	if _notifications.size() > 0:
		var random_index = randi() % _notifications.size()
		var notif = _notifications[random_index]

		# Add to queue for toast display
		_notification_queue.append(notif)

		# Emit signal to show toast
		notification_queued.emit(notif)

	# Restart timer with new random interval
	_start_debug_timer()


# =============================================================================
# LOCAL NOTIFICATIONS
# =============================================================================

## Initialize platform-specific local notification plugins
func _initialize_local_notifications() -> void:
	if OS.get_name() == "Android":
		_android_plugin = Engine.get_singleton("dcl-godot-android")
		if _android_plugin:
			print("Local notifications: Android plugin initialized")
			# Create notification channel (Android 8.0+)
			_android_plugin.createNotificationChannel(
				_local_notification_channel_id,
				_local_notification_channel_name,
				_local_notification_channel_description
			)
		else:
			push_warning("Local notifications: Android plugin not found")
	elif OS.get_name() == "iOS":
		_ios_plugin = Engine.get_singleton("DclGodotiOS")
		if _ios_plugin:
			print("Local notifications: iOS plugin initialized")
		else:
			push_warning("Local notifications: iOS plugin not found")


## Request permission to show local notifications
## This must be called before scheduling any notifications
## On Android 13+, this will show a permission dialog
## On iOS, this will show a permission dialog on first call
func request_local_notification_permission() -> void:
	if OS.get_name() == "Android" and _android_plugin:
		var granted = _android_plugin.requestNotificationPermission()
		local_notification_permission_changed.emit(granted)
	elif OS.get_name() == "iOS" and _ios_plugin:
		_ios_plugin.request_notification_permission()
		# Permission result is async on iOS, we can check it later with has_local_notification_permission()


## Check if local notification permission is granted
## Returns true if permission is granted, false otherwise
func has_local_notification_permission() -> bool:
	if OS.get_name() == "Android" and _android_plugin:
		return _android_plugin.hasNotificationPermission()
	elif OS.get_name() == "iOS" and _ios_plugin:
		return _ios_plugin.has_notification_permission()
	return false


## Schedule a local notification
##
## @param notification_id: Unique ID for this notification (for cancellation)
## @param title: Notification title
## @param body: Notification body text
## @param delay_seconds: Delay in seconds before showing the notification
## @return: true if scheduled successfully, false otherwise
func schedule_local_notification(
	notification_id: String, title: String, body: String, delay_seconds: int
) -> bool:
	if notification_id.is_empty():
		push_error("Local notification: notification_id cannot be empty")
		return false

	if delay_seconds < 0:
		push_error("Local notification: delay_seconds must be >= 0")
		return false

	var success = false

	if OS.get_name() == "Android" and _android_plugin:
		success = _android_plugin.scheduleLocalNotification(
			notification_id, title, body, delay_seconds
		)
	elif OS.get_name() == "iOS" and _ios_plugin:
		success = _ios_plugin.schedule_local_notification(
			notification_id, title, body, delay_seconds
		)
	else:
		push_warning("Local notifications not supported on this platform")
		return false

	if success:
		local_notification_scheduled.emit(notification_id)
		print(
			"Local notification scheduled: id=%s, title=%s, delay=%ds"
			% [notification_id, title, delay_seconds]
		)

	return success


## Cancel a scheduled local notification
##
## @param notification_id: The ID of the notification to cancel
## @return: true if cancelled successfully, false otherwise
func cancel_local_notification(notification_id: String) -> bool:
	if notification_id.is_empty():
		push_error("Local notification: notification_id cannot be empty")
		return false

	var success = false

	if OS.get_name() == "Android" and _android_plugin:
		success = _android_plugin.cancelLocalNotification(notification_id)
	elif OS.get_name() == "iOS" and _ios_plugin:
		success = _ios_plugin.cancel_local_notification(notification_id)
	else:
		push_warning("Local notifications not supported on this platform")
		return false

	if success:
		local_notification_cancelled.emit(notification_id)
		print("Local notification cancelled: id=%s" % notification_id)

	return success


## Cancel all scheduled local notifications
##
## @return: true if cancelled successfully, false otherwise
func cancel_all_local_notifications() -> bool:
	var success = false

	if OS.get_name() == "Android" and _android_plugin:
		success = _android_plugin.cancelAllLocalNotifications()
	elif OS.get_name() == "iOS" and _ios_plugin:
		success = _ios_plugin.cancel_all_local_notifications()
	else:
		push_warning("Local notifications not supported on this platform")
		return false

	if success:
		print("All local notifications cancelled")

	return success


## Clear the app badge number and remove delivered notifications
## This should be called when the app launches to clear any badge
## from notifications that were shown while the app was closed
func clear_badge_and_delivered_notifications() -> void:
	if OS.get_name() == "iOS" and _ios_plugin:
		_ios_plugin.clear_badge_number()
		print("Badge cleared on iOS")
	# Android doesn't have a standard badge system, so nothing to do


# =============================================================================
# QUEUE MANAGEMENT (Phase 3)
# =============================================================================

## Schedule a queued local notification (adds to database and schedules with OS if slots available)
##
## @param notification_id: Unique ID for this notification
## @param title: Notification title
## @param body: Notification body text
## @param trigger_timestamp: Unix timestamp (seconds) when notification should fire
## @return: true if added to queue successfully
func queue_local_notification(
	notification_id: String, title: String, body: String, trigger_timestamp: int
) -> bool:
	if notification_id.is_empty():
		push_error("Queue notification: notification_id cannot be empty")
		return false

	var plugin = _get_plugin()
	if not plugin:
		push_warning("Local notifications not supported on this platform")
		return false

	# Insert into database (is_scheduled = 0 initially)
	var success = plugin.dbInsertNotification(
		notification_id, title, body, trigger_timestamp, 0, ""
	) if OS.get_name() == "Android" else plugin.db_insert_notification(
		notification_id, title, body, trigger_timestamp, 0, ""
	)

	if not success:
		push_error("Failed to insert notification into database: id=%s" % notification_id)
		return false

	# Sync queue to schedule with OS if there are available slots
	_sync_notification_queue()

	return true


## Cancel a queued local notification (removes from database and OS if scheduled)
##
## @param notification_id: The ID of the notification to cancel
## @return: true if cancelled successfully
func cancel_queued_local_notification(notification_id: String) -> bool:
	if notification_id.is_empty():
		push_error("Cancel queued notification: notification_id cannot be empty")
		return false

	var plugin = _get_plugin()
	if not plugin:
		push_warning("Local notifications not supported on this platform")
		return false

	# Cancel from OS (if scheduled)
	_os_cancel_notification(notification_id)

	# Delete from database
	var success = plugin.dbDeleteNotification(notification_id) if OS.get_name() == "Android" else plugin.db_delete_notification(notification_id)

	if success:
		# Sync queue to potentially schedule another notification
		_sync_notification_queue()

	return success


## Get all queued notifications (both pending and scheduled)
##
## @return: Array of notification dictionaries
func get_queued_local_notifications() -> Array:
	var plugin = _get_plugin()
	if not plugin:
		return []

	var results = plugin.dbQueryNotifications("", "trigger_timestamp ASC", -1) if OS.get_name() == "Android" else plugin.db_query_notifications("", "trigger_timestamp ASC", -1)

	return results if results else []


## Get count of queued notifications
##
## @return: Total count of queued notifications
func get_queued_notification_count() -> int:
	var plugin = _get_plugin()
	if not plugin:
		return 0

	return plugin.dbCountNotifications("") if OS.get_name() == "Android" else plugin.db_count_notifications("")


## Force a queue sync
## Useful when app resumes from background to check for fired notifications
func force_queue_sync() -> void:
	_sync_notification_queue()


## Sync notification queue - ensures next 24 notifications are scheduled with OS
## This is called automatically, but can be called manually to force a sync
func _sync_notification_queue() -> void:
	var plugin = _get_plugin()
	if not plugin:
		return

	var current_time = Time.get_unix_time_from_system()

	# Step 1: Clear expired/triggered notifications from database
	# This removes any notifications whose trigger time has already passed
	var expired_count = plugin.dbClearExpired(current_time) if OS.get_name() == "Android" else plugin.db_clear_expired(current_time)

	# Step 2: Get currently scheduled notification IDs from OS
	var os_scheduled_ids = _os_get_scheduled_ids()

	# Step 3: Get all scheduled notifications from database and check if they're still in OS
	# Remove any that were scheduled but are no longer in OS and have passed their trigger time
	var scheduled_in_db = plugin.dbQueryNotifications(
		"is_scheduled = 1", "", -1
	) if OS.get_name() == "Android" else plugin.db_query_notifications(
		"is_scheduled = 1", "", -1
	)

	var cleaned_count = 0
	for notif in scheduled_in_db:
		var notif_id = notif.get("id", "")
		var trigger_ts = notif.get("trigger_timestamp", 0)

		# If notification was scheduled but is no longer in OS and has already triggered
		if not notif_id.is_empty() and trigger_ts < current_time and notif_id not in os_scheduled_ids:
			# This notification has fired and should be removed
			if OS.get_name() == "Android":
				plugin.dbDeleteNotification(notif_id)
			else:
				plugin.db_delete_notification(notif_id)
			cleaned_count += 1

	# Step 4: Mark all remaining notifications as unscheduled in database first
	var all_notifications = plugin.dbQueryNotifications("", "", -1) if OS.get_name() == "Android" else plugin.db_query_notifications("", "", -1)
	for notif in all_notifications:
		if notif.has("id"):
			if OS.get_name() == "Android":
				plugin.dbMarkScheduled(notif["id"], false)
			else:
				plugin.db_mark_scheduled(notif["id"], false)

	# Step 5: Mark notifications that are still in OS as scheduled in database
	for notif_id in os_scheduled_ids:
		if OS.get_name() == "Android":
			plugin.dbMarkScheduled(notif_id, true)
		else:
			plugin.db_mark_scheduled(notif_id, true)

	# Step 6: Get pending notifications (not scheduled, future timestamps)
	var where_clause = "is_scheduled = 0 AND trigger_timestamp > %d" % current_time
	var pending = plugin.dbQueryNotifications(
		where_clause, "trigger_timestamp ASC", MAX_OS_SCHEDULED_NOTIFICATIONS
	) if OS.get_name() == "Android" else plugin.db_query_notifications(
		where_clause, "trigger_timestamp ASC", MAX_OS_SCHEDULED_NOTIFICATIONS
	)

	# Step 7: Schedule pending notifications with OS (up to max limit)
	var scheduled_count = os_scheduled_ids.size()
	var available_slots = MAX_OS_SCHEDULED_NOTIFICATIONS - scheduled_count
	var newly_scheduled = 0

	if available_slots > 0 and pending.size() > 0:
		var to_schedule = mini(available_slots, pending.size())

		for i in range(to_schedule):
			var notif = pending[i]
			var notif_id = notif["id"] if notif.has("id") else ""
			var title = notif["title"] if notif.has("title") else ""
			var body = notif["body"] if notif.has("body") else ""
			var trigger_ts = notif["trigger_timestamp"] if notif.has("trigger_timestamp") else 0

			if notif_id.is_empty():
				continue

			# Calculate delay in seconds from now
			var delay_seconds = maxi(1, trigger_ts - current_time)

			# Schedule with OS
			var success = _os_schedule_notification(notif_id, title, body, delay_seconds)

			if success:
				# Mark as scheduled in database
				if OS.get_name() == "Android":
					plugin.dbMarkScheduled(notif_id, true)
				else:
					plugin.db_mark_scheduled(notif_id, true)
				newly_scheduled += 1


## Get the appropriate plugin for the current platform
func _get_plugin():
	if OS.get_name() == "Android":
		return _android_plugin
	elif OS.get_name() == "iOS":
		return _ios_plugin
	return null


## Wrapper for os_schedule_notification that works on both platforms
func _os_schedule_notification(
	notification_id: String, title: String, body: String, delay_seconds: int
) -> bool:
	var plugin = _get_plugin()
	if not plugin:
		return false

	if OS.get_name() == "Android":
		return plugin.osScheduleNotification(notification_id, title, body, delay_seconds)
	else:
		return plugin.os_schedule_notification(notification_id, title, body, delay_seconds)


## Wrapper for os_cancel_notification that works on both platforms
func _os_cancel_notification(notification_id: String) -> bool:
	var plugin = _get_plugin()
	if not plugin:
		return false

	if OS.get_name() == "Android":
		return plugin.osCancelNotification(notification_id)
	else:
		return plugin.os_cancel_notification(notification_id)


## Wrapper for os_get_scheduled_ids that works on both platforms
func _os_get_scheduled_ids() -> Array:
	var plugin = _get_plugin()
	if not plugin:
		return []

	var result = plugin.osGetScheduledIds() if OS.get_name() == "Android" else plugin.os_get_scheduled_ids()
	return Array(result) if result else []
