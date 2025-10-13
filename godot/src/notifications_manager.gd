extends Node

## NotificationsManager
##
## Autoload script for managing Decentraland notifications.
## Handles fetching, marking as read, and polling for new notifications.

signal new_notifications(notifications: Array)
signal notifications_updated
signal notification_error(error_message: String)

const BASE_URL = "https://notifications.decentraland.org"
const POLL_INTERVAL_SECONDS = 30.0  # Poll every 30 seconds

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


func _ready() -> void:
	# Create polling timer
	_poll_timer = Timer.new()
	_poll_timer.wait_time = POLL_INTERVAL_SECONDS
	_poll_timer.timeout.connect(_on_poll_timeout)
	add_child(_poll_timer)


## Start polling for new notifications
func start_polling() -> void:
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
	var filtered = []
	var filtered_types = {}  # Track which types were filtered out

	for notif in notifications:
		if notif is Dictionary and "type" in notif:
			var notif_type = notif["type"]
			if notif_type in SUPPORTED_NOTIFICATION_TYPES:
				filtered.append(notif)
			else:
				# Track unsupported types
				if not filtered_types.has(notif_type):
					filtered_types[notif_type] = 0
				filtered_types[notif_type] += 1

	# Log filtered types for debugging
	if filtered_types.size() > 0:
		print("NotificationsManager: Filtered out unsupported notification types:")
		for notif_type in filtered_types.keys():
			print("  - ", notif_type, " (", filtered_types[notif_type], " notifications)")

	return filtered


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

	# Check if user is authenticated
	if not Global.player_identity:
		promise.reject("Player identity not available")
		return promise

	var address = Global.player_identity.get_address_str()
	if address.is_empty():
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

	# Get signed headers first
	var headers_promise = Global.player_identity.async_get_identity_headers(url, "{}", "GET")  # Empty metadata for GET request

	headers_promise.on_resolved.connect(
		func():
			if headers_promise.is_rejected():
				var error = headers_promise.get_data()
				if error is PromiseError:
					promise.reject("Auth error: " + error.get_error())
				else:
					promise.reject("Auth error: " + str(error))
				return

			var headers: Dictionary = headers_promise.get_data()

			# Make HTTP request with signed headers
			var http_promise = Global.http_requester.request_json(
				url, HTTPClient.METHOD_GET, "", headers  # Empty body for GET
			)

			http_promise.on_resolved.connect(
				func():
					if http_promise.is_rejected():
						var error = http_promise.get_data()
						var error_msg = (
							"HTTP error: "
							+ (error.get_error() if error is PromiseError else str(error))
						)
						notification_error.emit(error_msg)
						promise.reject(error_msg)
						return

					var response = http_promise.get_data()

					# Parse JSON response
					if response is String:
						var json = JSON.new()
						var parse_result = json.parse(response)
						if parse_result == OK:
							var data = json.data
							if data is Dictionary and "notifications" in data:
								var notifications = data["notifications"]
								# Filter to only show supported notification types
								var filtered_notifications = _filter_notifications(notifications)
								_notifications = filtered_notifications
								new_notifications.emit(filtered_notifications)
								notifications_updated.emit()
								promise.resolve_with_data(filtered_notifications)
							else:
								var error_msg = "Invalid response format"
								notification_error.emit(error_msg)
								promise.reject(error_msg)
						else:
							var error_msg = "JSON parse error: " + json.get_error_message()
							notification_error.emit(error_msg)
							promise.reject(error_msg)
					else:
						var error_msg = "Unexpected response type"
						notification_error.emit(error_msg)
						promise.reject(error_msg)
			)
	)

	return promise


## Mark notifications as read
##
## @param notification_ids: Array of notification ID strings to mark as read
## @returns: Promise that resolves with number of notifications updated
func mark_as_read(notification_ids: PackedStringArray) -> Promise:
	var promise := Promise.new()

	if notification_ids.size() == 0:
		promise.reject("No notification IDs provided")
		return promise

	# Check if user is authenticated
	if not Global.player_identity:
		promise.reject("Player identity not available")
		return promise

	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		promise.reject("User not authenticated")
		return promise

	var url = BASE_URL + "/notifications/read"

	# Build request body
	var body = {"notificationIds": Array(notification_ids)}
	var body_json = JSON.stringify(body)

	# Get signed headers first
	var headers_promise = Global.player_identity.async_get_identity_headers(url, "{}", "PUT")  # Empty metadata for PUT request

	headers_promise.on_resolved.connect(
		func():
			if headers_promise.is_rejected():
				var error = headers_promise.get_data()
				if error is PromiseError:
					promise.reject("Auth error: " + error.get_error())
				else:
					promise.reject("Auth error: " + str(error))
				return

			var headers: Dictionary = headers_promise.get_data()
			headers["Content-Type"] = "application/json"

			# Make HTTP request with signed headers
			var http_promise = Global.http_requester.request_json(
				url, HTTPClient.METHOD_PUT, body_json, headers
			)

			http_promise.on_resolved.connect(
				func():
					if http_promise.is_rejected():
						var error = http_promise.get_data()
						var error_msg = (
							"HTTP error: "
							+ (error.get_error() if error is PromiseError else str(error))
						)
						notification_error.emit(error_msg)
						promise.reject(error_msg)
						return

					var response = http_promise.get_data()

					# Parse JSON response
					if response is String:
						var json = JSON.new()
						var parse_result = json.parse(response)
						if parse_result == OK:
							var data = json.data
							if data is Dictionary and "updated" in data:
								var updated_count = data["updated"]

								# Update local cache
								for notif in _notifications:
									if notif["id"] in notification_ids:
										notif["read"] = true

								notifications_updated.emit()
								promise.resolve_with_data(updated_count)
							else:
								var error_msg = "Invalid response format"
								notification_error.emit(error_msg)
								promise.reject(error_msg)
						else:
							var error_msg = "JSON parse error: " + json.get_error_message()
							notification_error.emit(error_msg)
							promise.reject(error_msg)
					else:
						var error_msg = "Unexpected response type"
						notification_error.emit(error_msg)
						promise.reject(error_msg)
			)
	)

	return promise


func _on_poll_timeout() -> void:
	if _is_polling:
		# Fetch only unread notifications during polling
		fetch_notifications(-1, 50, true)
