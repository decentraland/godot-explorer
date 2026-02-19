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

const POLL_INTERVAL_SECONDS = 30.0  # Poll every 30 seconds

## TESTING: Set to true to inject fake notifications for testing
const ENABLE_FAKE_NOTIFICATIONS = false

## TESTING: Set to false to disable type filtering and show all notifications
const ENABLE_NOTIFICATION_FILTER = true

## DEBUG: Set to true to enable random notification generation for testing
const ENABLE_DEBUG_RANDOM_NOTIFICATIONS = false

## DEBUG: Set to true to schedule a test notification 2 minutes from now using real event data
const DEBUG_SCHEDULE_TEST_EVENT_NOTIFICATION = false

## Supported notification types (whitelist)
## Only these types will be shown to the user (systems that are implemented)
const SUPPORTED_NOTIFICATION_TYPES = [
	"events_starts_soon",  # Events: Event starts soon
	"events_started",  # Events: Event has started
	"reward_assignment",  # Rewards: Reward assigned/received
	"reward_in_progress",  # Rewards: Reward being processed
	"social_service_friendship_request",  # Friends: Friend request received (server notification)
	"social_service_friendship_accepted",  # Friends: Friend request accepted (server notification)
]

# Queue management constants (Phase 3)
const MAX_OS_SCHEDULED_NOTIFICATIONS = 24  # Maximum notifications scheduled with OS at once

# Toast display: Only show notifications newer than 5 minutes
const TOAST_MAX_AGE_MS = 5 * 60 * 1000  # 5 minutes in milliseconds

# Event notification sync constants
const NOTIFICATION_ADVANCE_MINUTES = 3  # Notify 3 minutes before event starts

# Local notifications version - increment this to clear and re-sync all notifications
const LOCAL_NOTIFICATIONS_VERSION = 1

## DEBUG: Enable verbose logging of notification database operations
## Only active in debug builds (OS.is_debug_build())
var _debug_notifications_enabled: bool = false

var _notifications: Array = []
var _poll_timer: Timer = null
var _is_polling: bool = false
var _previous_notification_ids: Array = []
var _notification_queue: Array = []  # Queue for new unread notifications to show as toasts
var _debug_timer: Timer = null  # Timer for debug random notifications
var _queue_paused: bool = false  # Whether the notification queue is paused

# Local notifications wrapper
var _os_wrapper: NotificationOSWrapper = null

# DEPRECATED: Keep these for backward compatibility with tests
var _ios_plugin = null:
	set(value):
		_ios_plugin = value
		if not _os_wrapper:
			_os_wrapper = NotificationOSWrapper.new()
		_os_wrapper.set_mock_plugin(value, false)
var _android_plugin = null:
	set(value):
		_android_plugin = value
		if not _os_wrapper:
			_os_wrapper = NotificationOSWrapper.new()
		_os_wrapper.set_mock_plugin(value, true)


func _ready() -> void:
	# Enable debug logging in debug builds
	# _debug_notifications_enabled = OS.is_debug_build()

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

	# Initialize OS notification wrapper (if not already created by tests)
	if not _os_wrapper:
		_os_wrapper = NotificationOSWrapper.new()
		_os_wrapper.initialize()

	# Connect wrapper signals
	if not _os_wrapper.permission_changed.is_connected(_on_permission_changed):
		_os_wrapper.permission_changed.connect(_on_permission_changed)
	if not _os_wrapper.notification_scheduled.is_connected(_on_notification_scheduled):
		_os_wrapper.notification_scheduled.connect(_on_notification_scheduled)
	if not _os_wrapper.notification_cancelled.is_connected(_on_notification_cancelled):
		_os_wrapper.notification_cancelled.connect(_on_notification_cancelled)

	# Clear any badge from previous notifications
	_os_wrapper.clear_badge()

	# Initial queue sync on app launch (relaunch)
	_sync_notification_queue.call_deferred()


## Start polling for new notifications
func start_polling() -> void:
	# Don't poll for guests
	if not NotificationUtils.is_user_authenticated():
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


## Get currently cached notifications sorted by timestamp descending (newest first)
func get_notifications() -> Array:
	var sorted = _notifications.duplicate()
	sorted.sort_custom(func(a, b): return a.get("timestamp", 0) > b.get("timestamp", 0))
	return sorted


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
				# Additional filtering: exclude friend request notifications from blocked users
				if notif_type == "social_service_friendship_request":
					# Check if the sender is blocked
					var sender_address = ""
					if "metadata" in notif and notif["metadata"] is Dictionary:
						var metadata = notif["metadata"]
						if "sender" in metadata and metadata["sender"] is Dictionary:
							sender_address = metadata["sender"].get("address", "")

					# Skip this notification if sender is blocked
					if (
						not sender_address.is_empty()
						and Global.social_blacklist.is_blocked(sender_address)
					):
						continue

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
		{
			"type": "event_created",
			"title": "Title (!)",
			"description": "Description (!)",
			"metadata":
			{
				"link":
				"https://decentraland.org/jump/events?id=5e33392d-fd7e-43db-8c65-ba18097cc7700",
				"CommunityId": "comm_id",
				"communityName": "Test Community",
			}
		},
		{
			"type": "events_started",
			"title": "Title (!)",
			"description": "Description (!)",
			"metadata":
			{
				"title": "Thank God is Friday",
				"description": "MAKUMBA SOCIAL CLUB will host",
				"link":
				"https://decentraland.org/jump/events?id=5f776ddc-bcc9-49e5-aa2c-d84f0b5dda27",
				"CommunityId": "comm_id",
				"communityName": "Test Community",
			}
		},
		{
			"type": "reward_assignment",
			"title": "Title (!)",
			"description": "Description (!)",
			"metadata":
			{
				"tokenName": "Test token name",
				"tokenImage": "https://",
				"tokenRarity": "rare",
				"title": "A test NFT",
				"description": "This is a test NFT"
			}
		},
		{
			"type": "reward_in_progress",
			"title": "Title (!)",
			"description": "Description (!)",
			"metadata":
			{
				"tokenName": "Test token name",
				"tokenImage": "https://",
				"tokenRarity": "rare",
				"tokenCategory": "Lowerbody",
				"title": "A test NFT",
				"description": "This is a test NFT"
			}
		},
		{
			"type": "social_service_friendship_request",
			"title": "Friend Request Received (!)",
			"description": "X wants to be your friend (!)",
			"metadata":
			{
				"sender":
				{
					"name": "TestUser",
					"hasClaimedName": true,
					"profileImageUrl": "",
					"address": "123456789101112",
				}
			}
		},
		{
			"type": "social_service_friendship_accepted",
			"title": "",
			"description": "",
			"metadata":
			{
				"sender":
				{
					"name": "TestUser",
					"hasClaimedName": true,
					"profileImageUrl": "",
					"address": "123456789101112",
				}
			}
		}
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
		(
			random_type["metadata"]
			if random_type.has("metadata")
			else {
				"title": random_type["title"],
				"description": random_type["description"],
				"link": "https://decentraland.org"
			}
		)
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
	if not NotificationUtils.is_user_authenticated():
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

	var url = DclUrls.notifications_api() + "/notifications" + query_string

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

				# Queue unread notifications for toast display (only if recent)
				if not notif.get("read", false):
					var notif_timestamp: int = int(notif.get("timestamp", 0))
					var current_time_ms: int = int(Time.get_unix_time_from_system() * 1000)
					var is_recent: bool = notif_timestamp > (current_time_ms - TOAST_MAX_AGE_MS)

					if is_recent:
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

	var url = DclUrls.notifications_api() + "/notifications/read"
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


## OS wrapper signal handlers
func _on_permission_changed(granted: bool) -> void:
	if granted:
		Global.metrics.track_click_button("accept", "NOTIF_PROMPT", "")
	else:
		Global.metrics.track_click_button("reject", "NOTIF_PROMPT", "")
	Global.metrics.flush.call_deferred()
	local_notification_permission_changed.emit(granted)


func _on_notification_scheduled(notification_id: String) -> void:
	local_notification_scheduled.emit(notification_id)


func _on_notification_cancelled(notification_id: String) -> void:
	local_notification_cancelled.emit(notification_id)


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
# SYSTEM TOASTS (in-app only, not OS notifications)
# =============================================================================


## Show a system toast notification (in-app only, for things like profile changes)
## @param title: The notification title
## @param description: The notification description
## @param notification_type: Type identifier (default: "system")
func show_system_toast(
	title: String, description: String, notification_type: String = "system"
) -> void:
	var timestamp = Time.get_unix_time_from_system() * 1000  # milliseconds
	var notif: Dictionary = {
		"id": "system_" + str(timestamp) + "_" + str(randi()),
		"type": notification_type,
		"address": "",
		"timestamp": int(timestamp),
		"read": true,  # Mark as read so it doesn't persist
		"metadata": {"title": title, "description": description, "link": ""}
	}

	# Add to queue for toast display
	_notification_queue.append(notif)

	# Emit signal to show toast if this is the only one in queue
	if _notification_queue.size() == 1 and not _queue_paused:
		notification_queued.emit(notif)


# =============================================================================
# LOCAL NOTIFICATIONS
# =============================================================================


## Request permission to show local notifications
func request_local_notification_permission(from_screen: String = "") -> void:
	if _os_wrapper:
		Global.metrics.track_screen_viewed("NOTIF_PROMPT", from_screen)
		Global.metrics.flush.call_deferred()
		_os_wrapper.request_permission()


## Check if local notification permission is granted
func has_local_notification_permission() -> bool:
	if _os_wrapper:
		return _os_wrapper.has_permission()
	return false


## Schedule a local notification
func schedule_local_notification(
	notification_id: String, title: String, body: String, delay_seconds: int
) -> bool:
	if _os_wrapper:
		return _os_wrapper.schedule(notification_id, title, body, delay_seconds)
	return false


## Cancel a scheduled local notification
func cancel_local_notification(notification_id: String) -> bool:
	if _os_wrapper:
		return _os_wrapper.cancel(notification_id)
	return false


## Cancel all scheduled local notifications
func cancel_all_local_notifications() -> bool:
	if _os_wrapper:
		return _os_wrapper.cancel_all()
	return false


## Clear the app badge number and remove delivered notifications
func clear_badge_and_delivered_notifications() -> void:
	if _os_wrapper:
		_os_wrapper.clear_badge()


## Generate random notification text for event reminders
## Returns a dictionary with "title" and "body" keys
## Each call returns a random combination from the available pools
func generate_event_notification_text(event_name: String) -> Dictionary:
	return NotificationUtils.generate_event_notification_text(event_name)


# =============================================================================
# QUEUE MANAGEMENT (Phase 3)
# =============================================================================


## Schedule a queued local notification (adds to database and schedules with OS if slots available)
##
## @param notification_id: Unique ID for this notification
## @param title: Notification title
## @param body: Notification body text
## @param trigger_timestamp: Unix timestamp (seconds) when notification should fire
## @param image_url: Optional image URL to download and attach
## @return: true if added to queue successfully
func async_queue_local_notification(
	notification_id: String,
	title: String,
	body: String,
	trigger_timestamp: int,
	image_url: String = "",
	deep_link_data: String = ""
) -> bool:
	if !Global.is_android() and !Global.is_ios():
		return false
	if notification_id.is_empty():
		push_error("Queue notification: notification_id cannot be empty")
		return false

	_debug_log(
		(
			"queue_local_notification called: id=%s, title=%s, trigger_ts=%d, has_image=%s, deep_link=%s"
			% [notification_id, title, trigger_timestamp, !image_url.is_empty(), deep_link_data]
		)
	)

	var plugin = _get_plugin()
	if not plugin:
		push_error("Queue notification: Local notifications plugin not available on this platform")
		return false

	# Download and convert image to base64 if URL provided
	var image_base64 = ""
	if not image_url.is_empty():
		image_base64 = await _async_download_image_as_base64(image_url)

	# Insert into database (is_scheduled = 0 initially)
	var success = (
		plugin.dbInsertNotification(
			notification_id, title, body, trigger_timestamp, 0, deep_link_data, image_base64
		)
		if OS.get_name() == "Android"
		else plugin.db_insert_notification(
			notification_id, title, body, trigger_timestamp, 0, deep_link_data, image_base64
		)
	)

	if not success:
		push_error("Failed to insert notification into database: id=%s" % notification_id)
		return false

	_debug_log("Notification inserted into database: id=%s" % notification_id)

	# Sync queue to schedule with OS if there are available slots
	_sync_notification_queue()

	# Print database state after insertion (dev mode only)
	_debug_print_database()

	return true


## Cancel a queued local notification (removes from database and OS if scheduled)
##
## @param notification_id: The ID of the notification to cancel
## @return: true if cancelled successfully, false if notification doesn't exist or error
func cancel_queued_local_notification(notification_id: String) -> bool:
	if notification_id.is_empty():
		push_error("Cancel queued notification: notification_id cannot be empty")
		return false

	_debug_log("Cancelling queued notification: id=%s" % notification_id)

	var plugin = _get_plugin()
	if not plugin:
		push_warning("Local notifications not supported on this platform")
		return false

	# Cancel from OS (if scheduled)
	_os_cancel_notification(notification_id)

	# Delete from database
	var success = (
		plugin.dbDeleteNotification(notification_id)
		if OS.get_name() == "Android"
		else plugin.db_delete_notification(notification_id)
	)

	if success:
		_debug_log("Notification deleted from database: id=%s" % notification_id)
		# Sync queue to potentially schedule another notification
		_sync_notification_queue()
		# Print database state after deletion (dev mode only)
		_debug_print_database()
	else:
		_debug_log(
			(
				"Notification not found in database (already cancelled or never scheduled): id=%s"
				% notification_id
			)
		)

	return success


## Get all queued notifications (both pending and scheduled)
##
## @return: Array of notification dictionaries
func get_queued_local_notifications() -> Array:
	var plugin = _get_plugin()
	if not plugin:
		return []

	var results = (
		plugin.dbQueryNotifications("", "trigger_timestamp ASC", -1)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications("", "trigger_timestamp ASC", -1)
	)

	return results if results else []


## Get count of queued notifications
##
## @return: Total count of queued notifications
func get_queued_notification_count() -> int:
	var plugin = _get_plugin()
	if not plugin:
		return 0

	return (
		plugin.dbCountNotifications("")
		if OS.get_name() == "Android"
		else plugin.db_count_notifications("")
	)


## Force a queue sync
## Useful when app resumes from background to check for fired notifications
func force_queue_sync() -> void:
	_sync_notification_queue()


## Check if local notifications version has changed and clear all if needed
## Returns true if notifications were cleared due to version mismatch
func _check_and_handle_version_change() -> bool:
	var stored_version: int = Global.get_config().local_notifications_version

	if stored_version == LOCAL_NOTIFICATIONS_VERSION:
		_debug_log("Local notifications version OK (v%d)" % LOCAL_NOTIFICATIONS_VERSION)
		return false

	_debug_log(
		(
			"Local notifications version mismatch: stored=%d, current=%d - clearing all notifications"
			% [stored_version, LOCAL_NOTIFICATIONS_VERSION]
		)
	)

	# Clear all notifications from OS
	cancel_all_local_notifications()

	# Clear all notifications from database
	var plugin = _get_plugin()
	if plugin:
		if OS.get_name() == "Android":
			plugin.dbClearAll()
		else:
			plugin.db_clear_all()

	# Update stored version
	Global.get_config().local_notifications_version = LOCAL_NOTIFICATIONS_VERSION
	Global.get_config().save_to_settings_file()

	_debug_log("All notifications cleared, version updated to v%d" % LOCAL_NOTIFICATIONS_VERSION)
	return true


## Sync local notifications with attended events from server
## Adds missing notifications and removes ones for unsubscribed events
## Should be called after user authentication
func async_sync_attended_events() -> void:
	_debug_log("Starting attended events sync...")

	# Skip on desktop - no local notification plugin available
	if OS.get_name() != "Android" and OS.get_name() != "iOS":
		return

	# Check version and clear all notifications if version changed
	_check_and_handle_version_change()

	# Check and request notification permission
	if not has_local_notification_permission():
		request_local_notification_permission("SYNC_ATTENDED_EVENTS")

		# Check permission after request
		# Note: On iOS this is async, but we'll try to schedule anyway
		# If permission is denied, the OS will handle it gracefully
		if not has_local_notification_permission():
			_debug_log(
				"Notification permission not granted yet, scheduling anyway (OS will handle)"
			)

	var url = DclUrls.mobile_events_api() + "/?only_attendee=true"
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if response is PromiseError:
		push_warning("Failed to fetch attended events: %s" % response.get_error())
		return

	var json = response.get_string_response_as_json()
	if not json is Dictionary or not json.has("data"):
		push_warning("Invalid attended events response format")
		return

	var all_events: Array = json.get("data", [])

	# Filter locally to only use events where attending=true
	# This is a workaround until the API's only_attendee parameter is fixed
	var events: Array = []
	for event_data in all_events:
		if event_data.get("attending", false) == true:
			events.append(event_data)

	_debug_log(
		"Found %d attended events (filtered from %d total)" % [events.size(), all_events.size()]
	)

	# Build set of attended event notification IDs
	var attended_notification_ids: Dictionary = {}
	for event_data in events:
		var event_id = event_data.get("id", "")
		if not event_id.is_empty():
			attended_notification_ids["event_" + event_id] = event_data

	# Get existing event notifications from database
	var existing_notifications = get_queued_local_notifications()
	var existing_event_ids: Dictionary = {}
	for notif in existing_notifications:
		var notif_id = notif.get("id", "")
		# Only track event notifications (prefixed with "event_")
		if notif_id.begins_with("event_"):
			existing_event_ids[notif_id] = true

	var current_time = int(Time.get_unix_time_from_system())
	var added_count = 0
	var removed_count = 0

	# REMOVE notifications for events user is no longer attending
	for existing_id in existing_event_ids:
		if not attended_notification_ids.has(existing_id):
			cancel_queued_local_notification(existing_id)
			removed_count += 1
			_debug_log("Removed notification for unsubscribed event: %s" % existing_id)

	# ADD notifications for attended events not yet scheduled

	for notification_id in attended_notification_ids:
		var event_data = attended_notification_ids[notification_id]
		var event_name = event_data.get("name", "Event")

		if existing_event_ids.has(notification_id):
			_debug_log("  [SKIP] %s - already scheduled" % event_name)
			continue

		# Parse event start time
		var start_at = event_data.get("next_start_at", event_data.get("start_at", ""))
		if start_at.is_empty():
			_debug_log("  [SKIP] %s - no start time" % event_name)
			continue

		var event_timestamp = _parse_iso_timestamp(start_at)
		if event_timestamp <= 0:
			_debug_log("  [SKIP] %s - invalid timestamp" % event_name)
			continue

		# Calculate trigger time (3 minutes before event)
		var trigger_time = event_timestamp - (NOTIFICATION_ADVANCE_MINUTES * 60)

		# Skip events that already started
		if trigger_time <= current_time:
			var mins_ago = (current_time - trigger_time) / 60
			_debug_log("  [SKIP] %s - already started (%d mins ago)" % [event_name, mins_ago])
			continue

		# Get event details
		var coordinates = Vector2i(int(event_data.get("x", 0)), int(event_data.get("y", 0)))
		var image_url = event_data.get("image", "")
		var deep_link = "decentraland://open?position=%d,%d" % [coordinates.x, coordinates.y]

		# Generate notification text
		var notification_text = generate_event_notification_text(event_name)

		var mins_until = (trigger_time - current_time) / 60
		_debug_log(
			(
				"  [SCHEDULE] %s - in %d mins at %d,%d"
				% [event_name, mins_until, coordinates.x, coordinates.y]
			)
		)

		# Schedule the notification
		var success = await async_queue_local_notification(
			notification_id,
			notification_text["title"],
			notification_text["body"],
			trigger_time,
			image_url,
			deep_link
		)

		if success:
			added_count += 1
		else:
			_debug_log("    [FAILED] Could not schedule notification")

	_debug_log(
		"Event notifications sync complete: added=%d, removed=%d" % [added_count, removed_count]
	)

	# DEBUG: Schedule a test notification 5 minutes from now using the first attended event
	if DEBUG_SCHEDULE_TEST_EVENT_NOTIFICATION and events.size() > 0:
		var test_event = events[0]
		var test_event_name = test_event.get("name", "Test Event")
		var test_notification_id = "debug_test_event_" + str(int(Time.get_unix_time_from_system()))
		var test_trigger_time = int(Time.get_unix_time_from_system()) + (2 * 60)  # 2 minutes from now
		var test_deep_link = "decentraland://open?position=100,100"
		var test_image_url = test_event.get("image", "")

		var test_notification_text = generate_event_notification_text(test_event_name)

		_debug_log("=".repeat(70))
		_debug_log("DEBUG: Scheduling test notification in 2 minutes")
		_debug_log("  Event: %s" % test_event_name)
		_debug_log("  Deep link: %s" % test_deep_link)
		_debug_log("=".repeat(70))

		await async_queue_local_notification(
			test_notification_id,
			test_notification_text["title"],
			test_notification_text["body"],
			test_trigger_time,
			test_image_url,
			test_deep_link
		)


## Parse ISO 8601 timestamp to Unix seconds
func _parse_iso_timestamp(iso_string: String) -> int:
	if iso_string.is_empty():
		return 0

	var date_parts = iso_string.split("T")
	if date_parts.size() != 2:
		return 0

	var date_part = date_parts[0]
	var time_part = date_parts[1].replace("Z", "").split(".")[0]

	var date_components = date_part.split("-")
	var time_components = time_part.split(":")

	if date_components.size() != 3 or time_components.size() != 3:
		return 0

	var date_dict = {
		"year": int(date_components[0]),
		"month": int(date_components[1]),
		"day": int(date_components[2]),
		"hour": int(time_components[0]),
		"minute": int(time_components[1]),
		"second": int(time_components[2])
	}

	return Time.get_unix_time_from_datetime_dict(date_dict)


## Sync notification queue - ensures next 24 notifications are scheduled with OS
## This is called automatically, but can be called manually to force a sync
func _sync_notification_queue() -> void:
	var plugin = _get_plugin()
	if not plugin:
		return

	_debug_log("Starting queue sync...")

	var current_time = Time.get_unix_time_from_system()

	# Step 1: Clear expired/triggered notifications from database
	# This removes any notifications whose trigger time has already passed
	if OS.get_name() == "Android":
		plugin.dbClearExpired(current_time)
	else:
		plugin.db_clear_expired(current_time)

	# Step 2: Get currently scheduled notification IDs from OS
	var os_scheduled_ids = _os_get_scheduled_ids()

	# Step 3: Get all scheduled notifications from database and check if they're still in OS
	# Remove any that were scheduled but are no longer in OS and have passed their trigger time
	var scheduled_in_db = (
		plugin.dbQueryNotifications("is_scheduled = 1", "", -1)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications("is_scheduled = 1", "", -1)
	)

	for notif in scheduled_in_db:
		var notif_id = notif.get("id", "")
		var trigger_ts = notif.get("trigger_timestamp", 0)

		# If notification was scheduled but is no longer in OS and has already triggered
		if (
			not notif_id.is_empty()
			and trigger_ts < current_time
			and notif_id not in os_scheduled_ids
		):
			# This notification has fired and should be removed
			if OS.get_name() == "Android":
				plugin.dbDeleteNotification(notif_id)
			else:
				plugin.db_delete_notification(notif_id)

	# Step 4: Mark all remaining notifications as unscheduled in database first
	var all_notifications = (
		plugin.dbQueryNotifications("", "", -1)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications("", "", -1)
	)
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

	# Step 6: Ensure the next 24 notifications by timestamp are scheduled
	# Get the next 24 future notifications that SHOULD be scheduled
	var should_be_scheduled = (
		plugin.dbQueryNotifications(
			"trigger_timestamp > %d" % current_time,
			"trigger_timestamp ASC",
			MAX_OS_SCHEDULED_NOTIFICATIONS
		)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications(
			"trigger_timestamp > %d" % current_time,
			"trigger_timestamp ASC",
			MAX_OS_SCHEDULED_NOTIFICATIONS
		)
	)

	# Step 6a: Cancel OS notifications that aren't in the top 24
	for os_id in os_scheduled_ids:
		var should_keep = false
		for notif in should_be_scheduled:
			if notif.get("id", "") == os_id:
				should_keep = true
				break

		if not should_keep:
			# This notification is scheduled but shouldn't be (not in top 24)
			_debug_log("Cancelling OS notification (not in top 24): id=%s" % os_id)
			var cancel_success = _os_cancel_notification(os_id)
			if cancel_success:
				# Mark as unscheduled in database
				if OS.get_name() == "Android":
					plugin.dbMarkScheduled(os_id, false)
				else:
					plugin.db_mark_scheduled(os_id, false)

	# Step 6b: Schedule notifications that should be in top 24 but aren't in OS
	for notif in should_be_scheduled:
		var notif_id = notif.get("id", "")
		if notif_id.is_empty():
			continue

		# Check if already scheduled in OS
		if notif_id in os_scheduled_ids:
			continue

		# Schedule it
		var title = notif.get("title", "")
		var body = notif.get("body", "")
		var trigger_ts = notif.get("trigger_timestamp", 0)
		var data = notif.get("data", "")
		var delay_seconds = maxi(1, trigger_ts - current_time)

		var success = _os_schedule_notification(notif_id, title, body, delay_seconds)
		if success:
			# Log the scheduling with deep link info (dev mode only)
			_debug_log_notification_scheduled(
				notif_id, title, body, trigger_ts, delay_seconds, data
			)

			# Mark as scheduled in database
			if OS.get_name() == "Android":
				plugin.dbMarkScheduled(notif_id, true)
			else:
				plugin.db_mark_scheduled(notif_id, true)

	_debug_log("Queue sync completed")


## Get the appropriate plugin for the current platform (used by queue management)
func _get_plugin():
	if _os_wrapper:
		return _os_wrapper.get_plugin()
	return null


## Wrapper for os_schedule_notification that works on both platforms
func _os_schedule_notification(
	notification_id: String, title: String, body: String, delay_seconds: int
) -> bool:
	if _os_wrapper:
		return _os_wrapper.schedule(notification_id, title, body, delay_seconds)
	return false


## Wrapper for os_cancel_notification that works on both platforms
func _os_cancel_notification(notification_id: String) -> bool:
	if _os_wrapper:
		return _os_wrapper.cancel(notification_id)
	return false


## Wrapper for os_get_scheduled_ids that works on both platforms
func _os_get_scheduled_ids() -> Array:
	var plugin = _get_plugin()
	if not plugin:
		return []

	var result = (
		plugin.osGetScheduledIds() if OS.get_name() == "Android" else plugin.os_get_scheduled_ids()
	)
	return Array(result) if result else []


## Print notification queue state for debugging on app launch/refocus
func _print_queue_state(current_time: int, scheduled_count: int, pending_count: int) -> void:
	if not _debug_notifications_enabled:
		return

	var plugin = _get_plugin()
	if not plugin:
		return

	# Get total count
	var total_count = (
		plugin.dbCountNotifications("")
		if OS.get_name() == "Android"
		else plugin.db_count_notifications("")
	)

	# Get next few scheduled notifications
	var scheduled_notifs = (
		plugin.dbQueryNotifications("is_scheduled = 1", "trigger_timestamp ASC", 3)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications("is_scheduled = 1", "trigger_timestamp ASC", 3)
	)

	# Get next few pending notifications
	var where_clause = "is_scheduled = 0 AND trigger_timestamp > %d" % current_time
	var pending_notifs = (
		plugin.dbQueryNotifications(where_clause, "trigger_timestamp ASC", 3)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications(where_clause, "trigger_timestamp ASC", 3)
	)

	print("\n=== Local Notification Queue State ===")
	print("Total notifications in database: %d" % total_count)
	print("Scheduled with OS: %d / %d" % [scheduled_count, MAX_OS_SCHEDULED_NOTIFICATIONS])
	print("Pending (not yet scheduled): %d" % pending_count)

	if scheduled_notifs.size() > 0:
		print("\nNext scheduled notifications:")
		for notif in scheduled_notifs:
			var title = notif.get("title", "Unknown")
			var trigger_ts = notif.get("trigger_timestamp", 0)
			var time_until = trigger_ts - current_time
			var mins_until = int(time_until / 60.0)
			print("  - '%s' in %d minutes" % [title, mins_until])

	if pending_notifs.size() > 0:
		print("\nNext pending notifications:")
		for notif in pending_notifs:
			var title = notif.get("title", "Unknown")
			var trigger_ts = notif.get("trigger_timestamp", 0)
			var time_until = trigger_ts - current_time
			var mins_until = int(time_until / 60.0)
			print("  - '%s' in %d minutes" % [title, mins_until])

	print("======================================\n")


# =============================================================================
# DEBUG LOGGING (Dev Mode Only)
# =============================================================================


## Print a debug message for notification operations (only in dev mode)
func _debug_log(message: String) -> void:
	if _debug_notifications_enabled:
		print("[NotificationsManager DEBUG] %s" % message)


## Print the entire notification database contents (only in dev mode)
func _debug_print_database() -> void:
	if not _debug_notifications_enabled:
		return

	var plugin = _get_plugin()
	if not plugin:
		print("[NotificationsManager DEBUG] Cannot print database: plugin not available")
		return

	var all_notifications = (
		plugin.dbQueryNotifications("", "trigger_timestamp ASC", -1)
		if OS.get_name() == "Android"
		else plugin.db_query_notifications("", "trigger_timestamp ASC", -1)
	)

	var current_time = int(Time.get_unix_time_from_system())

	print("\n" + "=".repeat(70))
	print("[NotificationsManager DEBUG] DATABASE DUMP")
	print("=".repeat(70))
	print("Current time: %d" % current_time)
	print("Total notifications in database: %d" % all_notifications.size())
	print("-".repeat(70))

	if all_notifications.size() == 0:
		print("  (database is empty)")
	else:
		for notif in all_notifications:
			var notif_id = notif.get("id", "?")
			var title = notif.get("title", "?")
			var body = notif.get("body", "?")
			var trigger_ts = notif.get("trigger_timestamp", 0)
			var is_scheduled = notif.get("is_scheduled", 0)
			var data = notif.get("data", "")

			var time_until = trigger_ts - current_time
			var mins_until = int(time_until / 60.0)
			var status = "SCHEDULED" if is_scheduled == 1 else "PENDING"
			var time_str = "%d min" % mins_until if time_until > 0 else "EXPIRED"

			print("  [%s] id=%s" % [status, notif_id])
			print("    title: %s" % title)
			print("    body: %s" % body)
			print("    trigger_timestamp: %d (%s)" % [trigger_ts, time_str])
			print("    data (deep link): %s" % data)
			print("")

	print("=".repeat(70) + "\n")


## Log a notification being queued/scheduled with deep link info
func _debug_log_notification_scheduled(
	notification_id: String,
	title: String,
	body: String,
	trigger_timestamp: int,
	delay_seconds: int,
	data: String = ""
) -> void:
	if not _debug_notifications_enabled:
		return

	print("\n" + "-".repeat(50))
	print("[NotificationsManager DEBUG] NOTIFICATION SCHEDULED")
	print("-".repeat(50))
	print("  id: %s" % notification_id)
	print("  title: %s" % title)
	print("  body: %s" % body)
	print("  trigger_timestamp: %d" % trigger_timestamp)
	print("  delay_seconds: %d" % delay_seconds)
	if not data.is_empty():
		print("  deep_link_data: %s" % data)
	else:
		print("  deep_link_data: (none)")
	print("-".repeat(50) + "\n")


## Download an image from URL and convert to base64 string
## @param image_url: URL of the image to download
## @return: Base64 encoded string of the image, or empty string if failed
func _async_download_image_as_base64(image_url: String) -> String:
	if image_url.is_empty():
		return ""

	# Use Global.content_provider to download the image
	var url_hash = NotificationUtils.get_hash_from_url(image_url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, image_url)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		push_error("Failed to download notification image: " + result.get_error())
		return ""

	# Get the Image from the texture
	var texture: Texture2D = result.texture
	if not texture:
		push_error("Downloaded texture is null")
		return ""

	var image: Image = texture.get_image()
	if not image:
		push_error("Failed to get image from texture")
		return ""

	# Save to PNG buffer
	var png_buffer = image.save_png_to_buffer()
	if png_buffer.is_empty():
		push_error("Failed to convert image to PNG")
		return ""

	# Convert to base64
	var base64_string = Marshalls.raw_to_base64(png_buffer)
	return base64_string
