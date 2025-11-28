class_name NotificationOSWrapper
extends RefCounted

## Platform-specific wrapper for local notifications (Android/iOS)
## Handles plugin initialization, permission requests, and OS-level notification operations

signal permission_changed(granted: bool)
signal notification_scheduled(notification_id: String)
signal notification_cancelled(notification_id: String)

# Platform-specific plugin references
var _android_plugin = null
var _ios_plugin = null

# Notification channel configuration (Android)
var _channel_id = "dcl_local_notifications"
var _channel_name = "Decentraland Notifications"
var _channel_description = "Local notifications for Decentraland events"


## Initialize platform-specific plugins
func initialize() -> void:
	if OS.get_name() == "Android":
		_android_plugin = Engine.get_singleton("dcl-godot-android")
		if _android_plugin:
			# Create notification channel (Android 8.0+)
			_android_plugin.createNotificationChannel(
				_channel_id, _channel_name, _channel_description
			)
		else:
			push_warning("Local notifications: Android plugin not found")
	elif OS.get_name() == "iOS":
		_ios_plugin = Engine.get_singleton("DclGodotiOS")
		if not _ios_plugin:
			push_warning("Local notifications: iOS plugin not found")


## Request permission to show local notifications
## Android 13+: Shows permission dialog
## iOS: Shows permission dialog on first call
func request_permission() -> void:
	if OS.get_name() == "Android" and _android_plugin:
		var granted = _android_plugin.requestNotificationPermission()
		permission_changed.emit(granted)
	elif OS.get_name() == "iOS" and _ios_plugin:
		_ios_plugin.request_notification_permission()
		# Permission result is async on iOS


## Check if local notification permission is granted
func has_permission() -> bool:
	if OS.get_name() == "Android" and _android_plugin:
		return _android_plugin.hasNotificationPermission()
	if OS.get_name() == "iOS" and _ios_plugin:
		return _ios_plugin.has_notification_permission()
	return false


## Schedule a local notification
func schedule(notification_id: String, title: String, body: String, delay_seconds: int) -> bool:
	if notification_id.is_empty():
		push_error("Local notification: notification_id cannot be empty")
		return false

	if delay_seconds < 0:
		push_error("Local notification: delay_seconds must be >= 0")
		return false

	var success = false
	var plugin = get_plugin()

	if not plugin:
		push_warning("Local notifications not supported on this platform")
		return false

	# Call appropriate method based on plugin type (Android uses camelCase)
	if OS.get_name() == "Android":
		success = plugin.osScheduleNotification(notification_id, title, body, delay_seconds)
	else:
		success = plugin.os_schedule_notification(notification_id, title, body, delay_seconds)

	if success:
		notification_scheduled.emit(notification_id)
		print(
			(
				"Local notification scheduled: id=%s, title=%s, delay=%ds"
				% [notification_id, title, delay_seconds]
			)
		)

	return success


## Cancel a scheduled local notification
func cancel(notification_id: String) -> bool:
	if notification_id.is_empty():
		push_error("Local notification: notification_id cannot be empty")
		return false

	var plugin = get_plugin()
	if not plugin:
		push_warning("Local notifications not supported on this platform")
		return false

	var success = false
	if OS.get_name() == "Android":
		success = plugin.osCancelNotification(notification_id)
	else:
		success = plugin.os_cancel_notification(notification_id)

	if success:
		notification_cancelled.emit(notification_id)

	return success


## Cancel all scheduled local notifications
func cancel_all() -> bool:
	var plugin = get_plugin()
	if not plugin:
		push_warning("Local notifications not supported on this platform")
		return false

	# Note: This method doesn't exist on the mock plugin, only for real plugins
	if OS.get_name() == "Android":
		return plugin.cancelAllLocalNotifications()
	return plugin.cancel_all_local_notifications()


## Clear the app badge number and remove delivered notifications (iOS only)
func clear_badge() -> void:
	if OS.get_name() == "iOS" and _ios_plugin:
		_ios_plugin.clear_badge_number()
		print("Badge cleared on iOS")
	# Android doesn't have a standard badge system


## Get the appropriate plugin for the current platform
## Used by database queue management
func get_plugin():
	if OS.get_name() == "Android":
		return _android_plugin
	if OS.get_name() == "iOS":
		return _ios_plugin

	# For testing on macOS/other platforms: use iOS plugin if available
	# This allows tests to inject a mock plugin
	if _ios_plugin != null:
		return _ios_plugin
	if _android_plugin != null:
		return _android_plugin

	return null


## Set mock plugin for testing (bypasses platform detection)
func set_mock_plugin(mock_plugin, is_android: bool = false) -> void:
	if is_android:
		_android_plugin = mock_plugin
	else:
		_ios_plugin = mock_plugin
