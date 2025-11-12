extends RefCounted
# gdlint: disable=function-name,class-definitions-order

# Mock implementation of iOS/Android notification plugins for testing
# Simulates both DclGodotiOS and GodotAndroidPlugin interfaces

var _notifications_db: Dictionary = {}  # id -> notification data
var _os_scheduled_ids: Array[String] = []  # IDs scheduled with "OS"
var _has_permission: bool = true  # Permission state
var _image_blobs: Dictionary = {}  # id -> image data (separate for performance)

# Mock time offset for testing (seconds to add to current time)
var time_offset: int = 0

# Track calls for assertions
var schedule_calls: Array[Dictionary] = []
var cancel_calls: Array[String] = []
var insert_calls: Array[Dictionary] = []

# =============================================================================
# PERMISSION API
# =============================================================================


func request_notification_permission() -> void:
	_has_permission = true


func has_notification_permission() -> bool:
	return _has_permission


func requestNotificationPermission() -> bool:
	_has_permission = true
	return true


func hasNotificationPermission() -> bool:
	return _has_permission


# =============================================================================
# DATABASE API - iOS style (snake_case)
# =============================================================================


func db_insert_notification(
	id: String,
	title: String,
	body: String,
	trigger_timestamp: int,
	is_scheduled: int,
	data: String,
	image_base64: String
) -> bool:
	if id.is_empty():
		return false

	_notifications_db[id] = {
		"id": id,
		"title": title,
		"body": body,
		"trigger_timestamp": trigger_timestamp,
		"is_scheduled": is_scheduled,
		"data": data
	}

	if not image_base64.is_empty():
		_image_blobs[id] = image_base64

	insert_calls.append(
		{
			"id": id,
			"title": title,
			"body": body,
			"trigger_timestamp": trigger_timestamp,
			"is_scheduled": is_scheduled
		}
	)

	return true


func db_update_notification(id: String, updates: Dictionary) -> bool:
	if not _notifications_db.has(id):
		return false

	for key in updates.keys():
		_notifications_db[id][key] = updates[key]

	return true


func db_delete_notification(id: String) -> bool:
	if not _notifications_db.has(id):
		return false

	_notifications_db.erase(id)
	_image_blobs.erase(id)
	_os_scheduled_ids.erase(id)
	return true


func db_query_notifications(
	where_clause: String, order_by: String, limit: int
) -> Array[Dictionary]:
	var results: Array[Dictionary] = []

	# Get all notifications
	for notif in _notifications_db.values():
		# Simple WHERE clause parsing
		if _matches_where_clause(notif, where_clause):
			results.append(notif.duplicate())

	# Sort if order_by specified
	if not order_by.is_empty():
		results = _sort_notifications(results, order_by)

	# Apply limit
	if limit > 0 and results.size() > limit:
		results = results.slice(0, limit)

	return results


func db_count_notifications(where_clause: String) -> int:
	var count = 0

	for notif in _notifications_db.values():
		if _matches_where_clause(notif, where_clause):
			count += 1

	return count


func db_clear_expired(current_timestamp: int) -> int:
	var deleted_count = 0
	var to_delete: Array[String] = []

	for id in _notifications_db.keys():
		var notif = _notifications_db[id]
		if notif.get("trigger_timestamp", 0) <= current_timestamp:
			to_delete.append(id)

	for id in to_delete:
		db_delete_notification(id)
		deleted_count += 1

	return deleted_count


func db_mark_scheduled(id: String, is_scheduled: bool) -> bool:
	if not _notifications_db.has(id):
		return false

	_notifications_db[id]["is_scheduled"] = 1 if is_scheduled else 0
	return true


func db_get_notification(id: String) -> Dictionary:
	if not _notifications_db.has(id):
		return {}

	return _notifications_db[id].duplicate()


func db_clear_all() -> int:
	var count = _notifications_db.size()
	_notifications_db.clear()
	_image_blobs.clear()
	_os_scheduled_ids.clear()
	return count


func db_get_notification_image_blob(id: String) -> String:
	return _image_blobs.get(id, "")


# =============================================================================
# DATABASE API - Android style (camelCase)
# =============================================================================


func dbInsertNotification(
	id: String,
	title: String,
	body: String,
	trigger_timestamp: int,
	is_scheduled: int,
	data: String,
	image_base64: String
) -> bool:
	return db_insert_notification(
		id, title, body, trigger_timestamp, is_scheduled, data, image_base64
	)


func dbUpdateNotification(id: String, updates: Dictionary) -> bool:
	return db_update_notification(id, updates)


func dbDeleteNotification(id: String) -> bool:
	return db_delete_notification(id)


func dbQueryNotifications(where_clause: String, order_by: String, limit: int) -> Array[Dictionary]:
	return db_query_notifications(where_clause, order_by, limit)


func dbCountNotifications(where_clause: String) -> int:
	return db_count_notifications(where_clause)


func dbClearExpired(current_timestamp: int) -> int:
	return db_clear_expired(current_timestamp)


func dbMarkScheduled(id: String, is_scheduled: bool) -> bool:
	return db_mark_scheduled(id, is_scheduled)


func dbGetNotification(id: String) -> Dictionary:
	return db_get_notification(id)


func dbClearAll() -> int:
	return db_clear_all()


func dbGetNotificationImageBlob(id: String) -> String:
	return db_get_notification_image_blob(id)


# =============================================================================
# OS NOTIFICATION API
# =============================================================================


func os_schedule_notification(
	notification_id: String, title: String, body: String, delay_seconds: int
) -> bool:
	if notification_id.is_empty():
		return false

	# Add to scheduled list if not already there
	if not _os_scheduled_ids.has(notification_id):
		_os_scheduled_ids.append(notification_id)

	# Mark as scheduled in database
	db_mark_scheduled(notification_id, true)

	schedule_calls.append(
		{"id": notification_id, "title": title, "body": body, "delay_seconds": delay_seconds}
	)

	return true


func os_cancel_notification(notification_id: String) -> bool:
	_os_scheduled_ids.erase(notification_id)
	db_mark_scheduled(notification_id, false)

	cancel_calls.append(notification_id)
	return true


func os_get_scheduled_ids() -> PackedStringArray:
	var result = PackedStringArray()
	for id in _os_scheduled_ids:
		result.append(id)
	return result


func osScheduleNotification(
	notification_id: String, title: String, body: String, delay_seconds: int
) -> bool:
	return os_schedule_notification(notification_id, title, body, delay_seconds)


func osCancelNotification(notification_id: String) -> bool:
	return os_cancel_notification(notification_id)


func osGetScheduledIds() -> PackedStringArray:
	return os_get_scheduled_ids()


# =============================================================================
# HELPER FUNCTIONS
# =============================================================================


func _matches_where_clause(notif: Dictionary, where_clause: String) -> bool:
	if where_clause.is_empty():
		return true

	# Simple parser for common WHERE clauses
	# Supports: "is_scheduled = 0", "is_scheduled = 1", "trigger_timestamp > X", combinations with AND

	var clauses = where_clause.split(" AND ")

	for clause in clauses:
		clause = clause.strip_edges()

		# Parse "is_scheduled = X"
		if clause.begins_with("is_scheduled"):
			var parts = clause.split("=")
			if parts.size() == 2:
				var expected_value = parts[1].strip_edges().to_int()
				var actual_value = notif.get("is_scheduled", 0)
				if actual_value != expected_value:
					return false

		# Parse "trigger_timestamp > X"
		elif clause.contains("trigger_timestamp >"):
			var parts = clause.split(">")
			if parts.size() == 2:
				var threshold = parts[1].strip_edges().to_int()
				var actual_value = notif.get("trigger_timestamp", 0)
				if actual_value <= threshold:
					return false

		# Parse "trigger_timestamp < X"
		elif clause.contains("trigger_timestamp <"):
			var parts = clause.split("<")
			if parts.size() == 2:
				var threshold = parts[1].strip_edges().to_int()
				var actual_value = notif.get("trigger_timestamp", 0)
				if actual_value >= threshold:
					return false

		# Parse "trigger_timestamp <= X"
		elif clause.contains("trigger_timestamp <="):
			var parts = clause.split("<=")
			if parts.size() == 2:
				var threshold = parts[1].strip_edges().to_int()
				var actual_value = notif.get("trigger_timestamp", 0)
				if actual_value > threshold:
					return false

	return true


func _sort_notifications(notifications: Array[Dictionary], order_by: String) -> Array[Dictionary]:
	if order_by.is_empty():
		return notifications

	# Parse ORDER BY clause
	var ascending = true
	var field = "trigger_timestamp"

	if "DESC" in order_by:
		ascending = false

	if "trigger_timestamp" in order_by:
		field = "trigger_timestamp"

	# Sort using bubble sort (simple for testing)
	var sorted = notifications.duplicate()
	var n = sorted.size()

	for i in range(n):
		for j in range(0, n - i - 1):
			var a = sorted[j].get(field, 0)
			var b = sorted[j + 1].get(field, 0)

			var should_swap = (a > b) if ascending else (a < b)

			if should_swap:
				var temp = sorted[j]
				sorted[j] = sorted[j + 1]
				sorted[j + 1] = temp

	return sorted


# =============================================================================
# TEST UTILITIES
# =============================================================================


func reset() -> void:
	"""Reset the mock to initial state"""
	_notifications_db.clear()
	_os_scheduled_ids.clear()
	_image_blobs.clear()
	schedule_calls.clear()
	cancel_calls.clear()
	insert_calls.clear()
	_has_permission = true
	time_offset = 0


func get_database_state() -> Dictionary:
	"""Get current state for debugging"""
	return {
		"total_notifications": _notifications_db.size(),
		"os_scheduled_count": _os_scheduled_ids.size(),
		"notifications": _notifications_db.values(),
		"scheduled_ids": _os_scheduled_ids.duplicate()
	}


func set_permission(has_perm: bool) -> void:
	"""Set permission state for testing"""
	_has_permission = has_perm
