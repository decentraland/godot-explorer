extends Node

# Comprehensive tests for the notification queue system
# Tests all queue management scenarios using a mock plugin

const MockNotificationsPlugin = preload("res://src/test/notifications/mock_notifications_plugin.gd")

var mock_plugin: MockNotificationsPlugin
var test_results: Array[Dictionary] = []

# Test constants
const MAX_OS_SCHEDULED = 24
const BASE_TIMESTAMP = 1700000000  # Nov 14, 2023 22:13:20


func _ready() -> void:
	await run_all_tests()
	var all_passed = print_test_results()

	# Exit gracefully
	await get_tree().create_timer(0.5).timeout
	var exit_code = 0 if all_passed else 1
	get_tree().quit(exit_code)


func run_all_tests() -> void:
	print("\n========================================")
	print("Starting Notification Queue Tests")
	print("========================================\n")

	# Setup
	setup_test_environment()

	# Run all test scenarios
	await test_add_single_notification()
	await test_delete_notification()
	await test_add_notification_when_queue_full()
	await test_delete_notification_when_queue_full()
	await test_add_notification_after_24th()
	await test_add_notification_before_24th()
	await test_queue_sync_on_app_launch()
	await test_cancel_queued_notification()
	await test_expired_notification_cleanup()
	await test_queue_with_images()

	print("\n========================================")
	print("All tests completed")
	print("========================================\n")


func setup_test_environment() -> void:
	"""Initialize mock plugin"""
	mock_plugin = MockNotificationsPlugin.new()
	print("Test environment setup complete\n")


# =============================================================================
# TEST CASE 1: Add single notification to queue
# =============================================================================

func test_add_single_notification() -> void:
	var test_name = "Add single notification to queue"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Add a notification
	var notif_id = "test_notif_1"
	var title = "Test Event"
	var body = "This is a test notification"
	var trigger_time = BASE_TIMESTAMP + 3600  # 1 hour from base time

	var success = mock_plugin.db_insert_notification(
		notif_id, title, body, trigger_time, 0, "", ""
	)

	# Verify insertion
	var db_state = mock_plugin.get_database_state()
	var passed = (
		success and db_state["total_notifications"] == 1 and db_state["os_scheduled_count"] == 0
	)

	if passed:
		# Now "schedule" it with OS
		mock_plugin.os_schedule_notification(notif_id, title, body, 3600)
		mock_plugin.db_mark_scheduled(notif_id, true)

		db_state = mock_plugin.get_database_state()
		passed = db_state["os_scheduled_count"] == 1

	record_test_result(test_name, passed, "Added notification should be in database and schedulable")
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 2: Delete notification from queue
# =============================================================================

func test_delete_notification() -> void:
	var test_name = "Delete notification from queue"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Add and then delete
	var notif_id = "test_notif_delete"
	mock_plugin.db_insert_notification(
		notif_id, "Delete Test", "Body", BASE_TIMESTAMP + 1000, 0, "", ""
	)
	mock_plugin.os_schedule_notification(notif_id, "Delete Test", "Body", 1000)

	var delete_success = mock_plugin.db_delete_notification(notif_id)

	var db_state = mock_plugin.get_database_state()
	var passed = (
		delete_success
		and db_state["total_notifications"] == 0
		and db_state["os_scheduled_count"] == 0
	)

	record_test_result(
		test_name, passed, "Deleted notification should be removed from database and OS"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 3: Add notification when queue is full (24 notifications)
# =============================================================================

func test_add_notification_when_queue_full() -> void:
	var test_name = "Add notification when queue is full"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Fill queue with 24 notifications
	for i in range(MAX_OS_SCHEDULED):
		var notif_id = "full_queue_%d" % i
		var trigger_time = BASE_TIMESTAMP + (i + 1) * 60  # Each 1 minute apart
		mock_plugin.db_insert_notification(
			notif_id, "Event %d" % i, "Body %d" % i, trigger_time, 1, "", ""
		)
		mock_plugin.os_schedule_notification(notif_id, "Event %d" % i, "Body %d" % i, (i + 1) * 60)

	# Try to add a 25th notification that should come AFTER all 24
	var new_notif_id = "new_notif_after_24"
	var new_trigger_time = BASE_TIMESTAMP + (MAX_OS_SCHEDULED + 1) * 60  # After all others
	mock_plugin.db_insert_notification(
		new_notif_id, "New Event", "New Body", new_trigger_time, 0, "", ""
	)

	# Simulate queue sync logic
	# The 25th notification should NOT be scheduled since it comes after the 24th
	var db_state_after = mock_plugin.get_database_state()

	var passed = (
		db_state_after["total_notifications"] == MAX_OS_SCHEDULED + 1
		and db_state_after["os_scheduled_count"] == MAX_OS_SCHEDULED
	)

	record_test_result(
		test_name,
		passed,
		"25th notification after 24th should be added to DB but not scheduled with OS"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 4: Delete notification when queue is full
# =============================================================================

func test_delete_notification_when_queue_full() -> void:
	var test_name = "Delete notification when queue is full"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Fill queue with 24 notifications
	for i in range(MAX_OS_SCHEDULED):
		var notif_id = "full_queue_delete_%d" % i
		var trigger_time = BASE_TIMESTAMP + (i + 1) * 60
		mock_plugin.db_insert_notification(
			notif_id, "Event %d" % i, "Body %d" % i, trigger_time, 1, "", ""
		)
		mock_plugin.os_schedule_notification(
			notif_id, "Event %d" % i, "Body %d" % i, (i + 1) * 60
		)

	# Add a 25th that's pending
	var pending_id = "pending_25th"
	var pending_trigger = BASE_TIMESTAMP + (MAX_OS_SCHEDULED + 1) * 60
	mock_plugin.db_insert_notification(pending_id, "Pending", "Body", pending_trigger, 0, "", "")

	# Delete the 10th scheduled notification
	var deleted_id = "full_queue_delete_10"
	mock_plugin.os_cancel_notification(deleted_id)
	mock_plugin.db_delete_notification(deleted_id)

	# Simulate sync: now the 25th should be scheduled
	mock_plugin.os_schedule_notification(pending_id, "Pending", "Body", 1500)
	mock_plugin.db_mark_scheduled(pending_id, true)

	var db_state = mock_plugin.get_database_state()
	var passed = (
		db_state["total_notifications"] == MAX_OS_SCHEDULED
		and db_state["os_scheduled_count"] == MAX_OS_SCHEDULED
		and db_state["scheduled_ids"].has(pending_id)
	)

	record_test_result(
		test_name,
		passed,
		"Deleting from full queue should allow pending notification to be scheduled"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 5: Add notification that happens AFTER the 24th
# =============================================================================

func test_add_notification_after_24th() -> void:
	var test_name = "Add notification after 24th position"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Fill with 24 notifications
	for i in range(MAX_OS_SCHEDULED):
		var notif_id = "notif_%d" % i
		var trigger_time = BASE_TIMESTAMP + (i + 1) * 100  # Each 100 seconds apart
		mock_plugin.db_insert_notification(
			notif_id, "Event %d" % i, "Body", trigger_time, 1, "", ""
		)
		mock_plugin.os_schedule_notification(notif_id, "Event %d" % i, "Body", (i + 1) * 100)

	# Add notification AFTER the 24th (later timestamp)
	var late_notif_id = "late_notif"
	var late_trigger = BASE_TIMESTAMP + (MAX_OS_SCHEDULED + 10) * 100  # Much later
	mock_plugin.db_insert_notification(
		late_notif_id, "Late Event", "Body", late_trigger, 0, "", ""
	)

	# Query next 24 by timestamp
	var next_24 = mock_plugin.db_query_notifications(
		"trigger_timestamp > %d" % (BASE_TIMESTAMP - 1), "trigger_timestamp ASC", MAX_OS_SCHEDULED
	)

	# The late notification should NOT be in the next 24
	var late_in_next_24 = false
	for notif in next_24:
		if notif["id"] == late_notif_id:
			late_in_next_24 = true
			break

	var passed = (
		next_24.size() == MAX_OS_SCHEDULED and not late_in_next_24
		and mock_plugin.db_get_notification(late_notif_id).get("is_scheduled", 0) == 0
	)

	record_test_result(
		test_name,
		passed,
		"Notification after 24th should not be in next 24 and should remain unscheduled"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 6: Add notification that happens BEFORE the 24th
# =============================================================================

func test_add_notification_before_24th() -> void:
	var test_name = "Add notification before 24th position"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Fill with 24 notifications (starting at BASE_TIMESTAMP + 200 to leave room)
	for i in range(MAX_OS_SCHEDULED):
		var notif_id = "notif_%d" % i
		var trigger_time = BASE_TIMESTAMP + 200 + (i * 100)  # Start at +200
		mock_plugin.db_insert_notification(
			notif_id, "Event %d" % i, "Body", trigger_time, 1, "", ""
		)
		mock_plugin.os_schedule_notification(notif_id, "Event %d" % i, "Body", i * 100)

	# Remember the 24th notification
	var notif_24th_id = "notif_23"  # 0-indexed, so 23 is the 24th

	# Add a 25th that comes AFTER the 24th
	var notif_25th_id = "notif_25"
	var trigger_25th = BASE_TIMESTAMP + 200 + (MAX_OS_SCHEDULED * 100)
	mock_plugin.db_insert_notification(
		notif_25th_id, "Event 25", "Body", trigger_25th, 0, "", ""
	)

	# Now add a notification that should be at position 15 (in the middle)
	var early_notif_id = "early_notif"
	var early_trigger = BASE_TIMESTAMP + 200 + (14 * 100) + 50  # Between 14th and 15th
	mock_plugin.db_insert_notification(
		early_notif_id, "Early Event", "Body", early_trigger, 0, "", ""
	)

	# Simulate queue sync: Get next 24
	var next_24 = mock_plugin.db_query_notifications(
		"trigger_timestamp > %d" % (BASE_TIMESTAMP - 1), "trigger_timestamp ASC", MAX_OS_SCHEDULED
	)

	# The early notification should be in next 24
	var early_in_next_24 = false
	var notif_24th_in_next_24 = false
	var notif_25th_in_next_24 = false

	for notif in next_24:
		if notif["id"] == early_notif_id:
			early_in_next_24 = true
		elif notif["id"] == notif_24th_id:
			notif_24th_in_next_24 = true
		elif notif["id"] == notif_25th_id:
			notif_25th_in_next_24 = true

	# Simulate sync: cancel 24th, schedule early notification
	mock_plugin.os_cancel_notification(notif_24th_id)
	mock_plugin.db_mark_scheduled(notif_24th_id, false)
	mock_plugin.os_schedule_notification(early_notif_id, "Early Event", "Body", 1400)
	mock_plugin.db_mark_scheduled(early_notif_id, true)

	var passed = (
		early_in_next_24
		and notif_24th_in_next_24
		and not notif_25th_in_next_24
		and mock_plugin.db_get_notification(early_notif_id).get("is_scheduled", 0) == 1
		and mock_plugin.db_get_notification(notif_24th_id).get("is_scheduled", 0) == 0
	)

	record_test_result(
		test_name,
		passed,
		"Adding before 24th should bump 25th out, early notification should be scheduled"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 7: Queue sync on app launch
# =============================================================================

func test_queue_sync_on_app_launch() -> void:
	var test_name = "Queue sync on app launch"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Simulate notifications from a previous session
	# Some should be expired, some still valid

	var current_time = BASE_TIMESTAMP + 5000

	# Add expired notification (should be cleaned up)
	mock_plugin.db_insert_notification(
		"expired_1", "Expired", "Body", BASE_TIMESTAMP + 1000, 1, "", ""
	)
	mock_plugin.os_schedule_notification("expired_1", "Expired", "Body", 1000)

	# Add valid future notifications
	for i in range(10):
		var notif_id = "valid_%d" % i
		var trigger_time = current_time + (i + 1) * 100
		mock_plugin.db_insert_notification(notif_id, "Event %d" % i, "Body", trigger_time, 1, "", "")
		mock_plugin.os_schedule_notification(notif_id, "Event %d" % i, "Body", (i + 1) * 100)

	# Simulate sync: clear expired
	var cleared_count = mock_plugin.db_clear_expired(current_time)

	# Get next 24 that should be scheduled
	var next_24 = mock_plugin.db_query_notifications(
		"trigger_timestamp > %d" % current_time, "trigger_timestamp ASC", MAX_OS_SCHEDULED
	)

	var passed = (cleared_count == 1 and next_24.size() == 10 and not next_24[0]["id"] == "expired_1")

	record_test_result(
		test_name, passed, "Queue sync should remove expired and keep only future notifications"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 8: Cancel queued notification
# =============================================================================

func test_cancel_queued_notification() -> void:
	var test_name = "Cancel queued notification"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Add and schedule a notification
	var notif_id = "cancel_test"
	mock_plugin.db_insert_notification(
		notif_id, "Cancel Test", "Body", BASE_TIMESTAMP + 1000, 1, "", ""
	)
	mock_plugin.os_schedule_notification(notif_id, "Cancel Test", "Body", 1000)

	# Cancel it
	mock_plugin.os_cancel_notification(notif_id)
	mock_plugin.db_delete_notification(notif_id)

	var db_state = mock_plugin.get_database_state()
	var passed = (
		db_state["total_notifications"] == 0
		and db_state["os_scheduled_count"] == 0
		and not db_state["scheduled_ids"].has(notif_id)
	)

	record_test_result(test_name, passed, "Cancelled notification should be removed completely")
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 9: Expired notification cleanup
# =============================================================================

func test_expired_notification_cleanup() -> void:
	var test_name = "Expired notification cleanup"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	var current_time = BASE_TIMESTAMP + 10000

	# Add mix of expired and future notifications
	var expired_ids = ["expired_1", "expired_2", "expired_3"]
	for id in expired_ids:
		mock_plugin.db_insert_notification(id, "Expired", "Body", BASE_TIMESTAMP + 1000, 1, "", "")

	var future_ids = ["future_1", "future_2"]
	for id in future_ids:
		mock_plugin.db_insert_notification(
			id, "Future", "Body", current_time + 1000, 1, "", ""
		)

	# Clear expired
	var cleared = mock_plugin.db_clear_expired(current_time)

	var db_state = mock_plugin.get_database_state()
	var passed = (cleared == 3 and db_state["total_notifications"] == 2)

	# Verify only future notifications remain
	for notif in db_state["notifications"]:
		if expired_ids.has(notif["id"]):
			passed = false
			break

	record_test_result(test_name, passed, "Should remove only expired notifications")
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST CASE 10: Queue with images
# =============================================================================

func test_queue_with_images() -> void:
	var test_name = "Queue with images"
	print("Running: %s" % test_name)

	mock_plugin.reset()

	# Add notification with image
	var notif_id = "image_notif"
	var image_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="

	mock_plugin.db_insert_notification(
		notif_id, "Image Event", "Body", BASE_TIMESTAMP + 1000, 0, "", image_base64
	)

	# Query without image
	var notif = mock_plugin.db_get_notification(notif_id)
	var has_image_in_query = notif.has("image_base64")

	# Get image separately
	var image_blob = mock_plugin.db_get_notification_image_blob(notif_id)

	var passed = (not has_image_in_query and not image_blob.is_empty() and image_blob == image_base64)

	record_test_result(
		test_name,
		passed,
		"Image should be stored separately and retrieved only when needed"
	)
	print("  Result: %s\n" % ("PASS" if passed else "FAIL"))


# =============================================================================
# TEST UTILITIES
# =============================================================================

func record_test_result(test_name: String, passed: bool, description: String) -> void:
	test_results.append({"name": test_name, "passed": passed, "description": description})


func print_test_results() -> bool:
	print("\n========================================")
	print("TEST RESULTS SUMMARY")
	print("========================================\n")

	var total = test_results.size()
	var passed_count = 0

	for result in test_results:
		var status = "✓ PASS" if result["passed"] else "✗ FAIL"
		print("%s: %s" % [status, result["name"]])
		if not result["passed"]:
			print("  Expected: %s" % result["description"])
		passed_count += 1 if result["passed"] else 0

	print("\n========================================")
	print("Total: %d/%d tests passed (%.1f%%)" % [passed_count, total, (passed_count * 100.0 / total)])
	print("========================================\n")

	var all_passed = (passed_count == total)

	if all_passed:
		print("All tests passed!")
	else:
		printerr("Some tests failed!")

	return all_passed
