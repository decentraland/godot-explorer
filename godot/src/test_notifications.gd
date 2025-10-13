extends Node

# Simple test script to verify NotificationsManager works from GDScript
#
# To test:
# 1. Run: cargo run -- run
# 2. Attach this script to a Node in the scene tree
# 3. Press Y key to run the test
# 4. Check the output console for results

var test_running = false

func _ready():
	print("=== Notifications API Test Ready ===")
	print("Press Y to run notifications test")

func _input(event):
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_Y:
			if not test_running:
				print("\n=== Running Notifications Test ===")
				test_running = true
				test_notifications_api()
			else:
				print("⚠ Test already running, please wait...")

func test_notifications_api():
	# Create the notifications manager
	var notifications_manager = NotificationsManager.new()
	print("✓ NotificationsManager created")

	# Check if user is authenticated
	if not Global.player_identity:
		print("⚠ Player identity not found - test will fail")
		return

	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		print("⚠ User not authenticated - test will fail")
		print("⚠ Please log in first, then run this test")
		return

	print("✓ User authenticated:", address)

	# Test fetch_notifications
	print("\n--- Testing fetch_notifications ---")
	var fetch_promise = notifications_manager.fetch_notifications(-1, 10, false)

	fetch_promise.on_resolved.connect(func():
		test_running = false  # Reset flag when test completes

		if fetch_promise.is_rejected():
			var error = fetch_promise.get_data()
			if error is PromiseError:
				print("✗ Fetch failed:", error.get_error())
			else:
				print("✗ Fetch failed:", error)
			print("\n=== Test Complete (with errors) ===")
		else:
			var json_str = fetch_promise.get_data()
			print("✓ Fetch succeeded! JSON length:", json_str.length())

			# Parse JSON
			var json = JSON.new()
			var parse_result = json.parse(json_str)
			if parse_result == OK:
				var notifications = json.data
				print("✓ Parsed", notifications.size(), "notifications")

				if notifications.size() > 0:
					print("\n--- First notification ---")
					var first = notifications[0]
					print("  ID:", first.get("id", "N/A"))
					print("  Type:", first.get("type", "N/A"))
					print("  Read:", first.get("read", "N/A"))
					if "metadata" in first:
						var meta = first["metadata"]
						print("  Title:", meta.get("title", "N/A"))
						print("  Description:", meta.get("description", "N/A"))

					# Test mark_as_read with first notification
					if not first.get("read", true):
						print("\n--- Testing mark_notifications_read ---")
						var ids = PackedStringArray([first["id"]])
						var mark_promise = notifications_manager.mark_notifications_read(ids)

						mark_promise.on_resolved.connect(func():
							if mark_promise.is_rejected():
								var error = mark_promise.get_data()
								if error is PromiseError:
									print("✗ Mark read failed:", error.get_error())
								else:
									print("✗ Mark read failed:", error)
								print("\n=== Test Complete (with errors) ===")
							else:
								var updated_count = mark_promise.get_data()
								print("✓ Marked", updated_count, "notification(s) as read")
								print("\n=== Test Complete ===")
						)
					else:
						print("\n⚠ First notification already read, skipping mark_as_read test")
						print("\n=== Test Complete ===")
				else:
					print("⚠ No notifications found")
					print("\n=== Test Complete ===")
			else:
				print("✗ JSON parse error:", json.get_error_message())
				print("\n=== Test Complete (with errors) ===")
	)

	print("⏳ Waiting for async response...")
	print("💡 You can press Y again after this test completes")
