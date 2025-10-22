extends Node

## NotificationsManager
##
## Autoload script for managing Decentraland notifications.
## Handles fetching, marking as read, and polling for new notifications.

signal new_notifications(notifications: Array)
signal notifications_updated
signal notification_error(error_message: String)
signal notification_queued(notification: Dictionary)

const BASE_URL = "https://notifications.decentraland.org"
const POLL_INTERVAL_SECONDS = 30.0  # Poll every 30 seconds

## TESTING: Set to true to inject fake notifications for testing
const ENABLE_FAKE_NOTIFICATIONS = false

## TESTING: Set to false to disable type filtering and show all notifications
const ENABLE_NOTIFICATION_FILTER = false

## Supported notification types (whitelist)
## Only these types will be shown to the user
const SUPPORTED_NOTIFICATION_TYPES = [
	"item_sold",  # Marketplace: Item sold
	"bid_accepted",  # Marketplace: Bid accepted
	"bid_received",  # Marketplace: Bid received
	"royalties_earned",  # Marketplace: Royalties earned
	"governance_announcement",  # DAO: Governance announcements
	"governance_proposal_enacted",  # DAO: Proposal enacted
	"governance_voting_ended",  # DAO: Voting ended
	"governance_coauthor_requested",  # DAO: Co-author requested
	"land",  # Land-related notifications
	"worlds_access_restored",  # Worlds: Access restored
	"worlds_access_restricted",  # Worlds: Access restricted
	"worlds_missing_resources",  # Worlds: Missing resources
	"worlds_permission_granted",  # Worlds: Permission granted
	"worlds_permission_revoked",  # Worlds: Permission revoked
	# Excluded: "reward" - rewards system not implemented
	# Excluded: "events_*" - events system not implemented
	# Excluded: "friends_*" - friends system not implemented
]

var _notifications: Array = []
var _poll_timer: Timer = null
var _is_polling: bool = false
var _previous_notification_ids: Array = []
var _notification_queue: Array = []  # Queue for new unread notifications to show as toasts


func _ready() -> void:
	print("[NotificationsManager] Initializing NotificationsManager autoload")
	# Create polling timer
	_poll_timer = Timer.new()
	_poll_timer.wait_time = POLL_INTERVAL_SECONDS
	_poll_timer.timeout.connect(_on_poll_timeout)
	add_child(_poll_timer)
	print("[NotificationsManager] NotificationsManager ready")


## Start polling for new notifications
func start_polling() -> void:
	if not _is_polling:
		print("[NotificationsManager] Starting notification polling (interval: ", POLL_INTERVAL_SECONDS, "s)")
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

	print("[NotificationsManager] fetch_notifications called (from: ", from_timestamp, ", limit: ", limit, ", only_unread: ", only_unread, ")")

	# Check if user is authenticated
	if not Global.player_identity:
		print("[NotificationsManager] ERROR: Player identity not available")
		promise.reject("Player identity not available")
		return promise

	var address = Global.player_identity.get_address_str()
	print("[NotificationsManager] User address: ", address)
	if address.is_empty():
		print("[NotificationsManager] ERROR: User not authenticated (empty address)")
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
	print("[NotificationsManager] Fetching from URL: ", url)
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "{}")

	if response is PromiseError:
		var error_msg = "Fetch error: " + response.get_error()
		print("[NotificationsManager] Error: ", error_msg)
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	print("[NotificationsManager] Raw response received")
	print("[NotificationsManager] Response status: ", response.status_code())
	print("[NotificationsManager] Response body: ", response.get_response_as_string())

	var data: Dictionary = response.get_string_response_as_json()
	print("[NotificationsManager] Parsed data: ", JSON.stringify(data, "\t"))

	if not (data is Dictionary and "notifications" in data):
		var error_msg = "Invalid response format"
		print("[NotificationsManager] Error: ", error_msg)
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	var notifications = data["notifications"]
	print("[NotificationsManager] Total notifications received: ", notifications.size())
	print("[NotificationsManager] Notifications array: ", JSON.stringify(notifications, "\t"))

	var filtered_notifications = _filter_notifications(notifications)
	print("[NotificationsManager] Filtered notifications count: ", filtered_notifications.size())

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

	print("[NotificationsManager] New unread notifications detected: ", new_notifs.size())
	if new_notifs.size() > 0:
		print("[NotificationsManager] New notifications: ", JSON.stringify(new_notifs, "\t"))
		print("[NotificationsManager] Queue size: ", _notification_queue.size())
		# Emit signal to start processing queue
		if _notification_queue.size() == new_notifs.size():  # Only trigger if queue was empty
			notification_queued.emit(_notification_queue[0])

	# Emit updated notifications list
	new_notifications.emit(_notifications.duplicate())
	notifications_updated.emit()
	print("[NotificationsManager] Fetch complete. Total cached notifications: ", _notifications.size())
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
	print("[NotificationsManager] Marking as read - URL: ", url)
	print("[NotificationsManager] Request body: ", body_json)
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PUT, body_json)

	if response is PromiseError:
		var error_msg = "Mark as read error: " + response.get_error()
		print("[NotificationsManager] Error: ", error_msg)
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	print("[NotificationsManager] Mark as read response received")
	print("[NotificationsManager] Response status: ", response.status_code())
	print("[NotificationsManager] Response body: ", response.get_response_as_string())

	var data: Dictionary = response.get_string_response_as_json()
	print("[NotificationsManager] Parsed data: ", JSON.stringify(data, "\t"))

	if not (data is Dictionary and "updated" in data):
		var error_msg = "Invalid response format"
		print("[NotificationsManager] Error: ", error_msg)
		notification_error.emit(error_msg)
		promise.reject(error_msg)
		return

	var updated_count = data["updated"]
	print("[NotificationsManager] Successfully marked ", updated_count, " notifications as read")

	for notif in _notifications:
		if notif["id"] in notification_ids:
			notif["read"] = true

	notifications_updated.emit()
	promise.resolve_with_data(updated_count)


func _on_poll_timeout() -> void:
	if _is_polling:
		print("[NotificationsManager] Poll timeout - fetching unread notifications...")
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
		print("[NotificationsManager] Dequeued notification. Queue size: ", _notification_queue.size())

		# Return next notification if available
		if _notification_queue.size() > 0:
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
