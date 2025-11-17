## Social Service Debug Helper
##
## Provides debugging utilities for testing the social service on login.
## Connect signals and test basic functionality.

extends RefCounted


## Initialize debugging - connect signals and run tests
static func async_debug_social_service() -> void:
	print("[SocialDebug] Starting social service debug tests...")

	# Connect signals
	_connect_signals()

	# Run tests
	await _async_run_tests()

	print("[SocialDebug] Debug tests complete")


## Connect to all social service signals for debugging
static func _connect_signals() -> void:
	print("[SocialDebug] Connecting to social service signals...")

	Global.social_service.friendship_request_received.connect(_on_friend_request_received)
	Global.social_service.friendship_request_accepted.connect(_on_friend_request_accepted)
	Global.social_service.friendship_request_rejected.connect(_on_friend_request_rejected)
	Global.social_service.friendship_deleted.connect(_on_friendship_deleted)
	Global.social_service.friendship_request_cancelled.connect(_on_friend_request_cancelled)

	print("[SocialDebug] ✅ Signals connected")


## Run debug tests - fetch friends and pending requests
static func _async_run_tests() -> void:
	await _async_test_fetch_friends()
	await _async_test_fetch_pending_requests()


## Test: Fetch friends list
static func _async_test_fetch_friends() -> void:
	print("[SocialDebug] Fetching friends list...")

	var promise = Global.social_service.get_friends(50, 0, 3)
	await promise.on_resolved

	if promise.is_rejected():
		var error = promise.get_data()
		printerr("[SocialDebug] ❌ Failed to get friends: ", error.get_error())
	else:
		var friends = promise.get_data()
		print("[SocialDebug] ✅ Friends list (", friends.size(), " friends):")
		for friend in friends:
			print("  - ", friend)


## Test: Fetch pending friend requests
static func _async_test_fetch_pending_requests() -> void:
	print("[SocialDebug] Fetching pending friend requests...")

	var promise = Global.social_service.get_pending_requests(50, 0)
	await promise.on_resolved

	if promise.is_rejected():
		var error = promise.get_data()
		printerr("[SocialDebug] ❌ Failed to get pending requests: ", error.get_error())
	else:
		var requests = promise.get_data()
		print("[SocialDebug] ✅ Pending friend requests (", requests.size(), " requests):")
		for request in requests:
			print("  - From: ", request.address)
			if request.has("message") and request.message:
				print("    Message: ", request.message)
			if request.has("created_at"):
				print("    Created at: ", request.created_at)


# ============================================================================
# Signal Handlers
# ============================================================================


static func _on_friend_request_received(address: String, message: String) -> void:
	print("[SocialDebug] 🔔 New friend request from: ", address)
	if message:
		print("  Message: ", message)


static func _on_friend_request_accepted(address: String) -> void:
	print("[SocialDebug] 🎉 Friend request accepted by: ", address)


static func _on_friend_request_rejected(address: String) -> void:
	print("[SocialDebug] ❌ Friend request rejected by: ", address)


static func _on_friendship_deleted(address: String) -> void:
	print("[SocialDebug] 💔 Friendship deleted with: ", address)


static func _on_friend_request_cancelled(address: String) -> void:
	print("[SocialDebug] 🚫 Friend request cancelled by: ", address)
