class_name DisconnectHandler
extends Node

## Handles comms disconnection with automatic reconnection attempts
## and displays an overlay when reconnection fails.

## Disconnect reasons from CommunicationManager:
## 0 = DuplicateIdentity (same account logged in elsewhere)
## 1 = RoomClosed (the room was closed)
## 2 = Kicked (removed from server by admin)
## 3 = Other (server shutdown, signal close, etc.)

const MAX_RECONNECT_ATTEMPTS: int = 3

var _reconnect_attempts: int = 0
var _last_adapter_str: String = ""
var _last_disconnect_reason: int = -1
var _overlay: ColorRect = null
var _should_stop_reconnecting: bool = false


func _ready() -> void:
	Global.comms.disconnected.connect(_on_disconnected)
	Global.comms.on_adapter_changed.connect(_on_adapter_changed)


func _reset_reconnect_state() -> void:
	_reconnect_attempts = 0
	_last_adapter_str = ""
	_last_disconnect_reason = -1
	_should_stop_reconnecting = true  # Stop any pending reconnect attempts


func _on_adapter_changed(_voice_chat_enabled: bool, _adapter_str: String) -> void:
	# Successfully connected - reset reconnect attempts
	_reconnect_attempts = 0
	_last_adapter_str = ""
	_last_disconnect_reason = -1
	_should_stop_reconnecting = false


func _on_disconnected(reason: int) -> void:
	# Store the adapter string FIRST before any cleanup (might be cleared by clean())
	if _last_adapter_str.is_empty():
		_last_adapter_str = Global.comms.get_current_adapter_conn_str()

	# DuplicateIdentity means someone else logged in - don't retry, show error immediately
	if reason == 0:
		# Note: Don't reset _last_adapter_str here - we need it for reconnection
		_reconnect_attempts = 0
		_last_disconnect_reason = -1
		_should_stop_reconnecting = true  # Stop any pending reconnect attempts
		_show_disconnect_error(reason)
		return

	_last_disconnect_reason = reason
	_reconnect_attempts += 1
	_should_stop_reconnecting = false  # Allow reconnection attempts

	print("[DisconnectHandler] Disconnected (reason: %d), attempt %d/%d" % [reason, _reconnect_attempts, MAX_RECONNECT_ATTEMPTS])

	# If we haven't exhausted reconnect attempts, try to reconnect
	if _reconnect_attempts < MAX_RECONNECT_ATTEMPTS and not _last_adapter_str.is_empty():
		print("[DisconnectHandler] Attempting to reconnect...")
		_async_attempt_reconnect()
		return

	# Exhausted all attempts - show error overlay
	_show_disconnect_error(reason)


func _async_attempt_reconnect() -> void:
	# Small delay before reconnecting to avoid rapid reconnection loops
	await get_tree().create_timer(1.0).timeout

	# Check if we should stop (e.g., DuplicateIdentity received during the wait)
	if _should_stop_reconnecting:
		print("[DisconnectHandler] Reconnect cancelled")
		return

	# Check if adapter string is still valid
	if _last_adapter_str.is_empty():
		print("[DisconnectHandler] No adapter to reconnect to")
		return

	Global.comms.change_adapter(_last_adapter_str)


func _show_disconnect_error(reason: int) -> void:
	var title: String
	var message: String

	match reason:
		0:  # DuplicateIdentity
			title = "Session Ended"
			message = "Your session was ended because your account\nlogged in from another location."
		1:  # RoomClosed
			title = "Room Closed"
			message = "The room you were in has been closed."
		2:  # Kicked
			title = "Removed from Server"
			message = "You have been removed from the server\nby an administrator."
		_:  # Other
			title = "Disconnected"
			message = "You have been disconnected from the server.\nPlease try again later."

	_show_disconnect_overlay(title, message)


func _show_disconnect_overlay(title: String, message: String) -> void:
	# Remove existing overlay if any
	if _overlay != null and is_instance_valid(_overlay):
		_overlay.queue_free()

	# Full screen black background
	_overlay = ColorRect.new()
	_overlay.color = Color.BLACK
	_overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	_overlay.mouse_filter = Control.MOUSE_FILTER_STOP
	get_tree().root.add_child(_overlay)

	# Center container for the message box
	var center_container = CenterContainer.new()
	center_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	_overlay.add_child(center_container)

	# Message box container
	var message_box = VBoxContainer.new()
	message_box.custom_minimum_size = Vector2(400, 400)
	message_box.alignment = BoxContainer.ALIGNMENT_CENTER
	center_container.add_child(message_box)

	# Title label
	var title_label = Label.new()
	title_label.text = title
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title_label.add_theme_font_size_override("font_size", 32)
	title_label.add_theme_color_override("font_color", Color.WHITE)
	message_box.add_child(title_label)

	# Spacer
	var spacer1 = Control.new()
	spacer1.custom_minimum_size = Vector2(0, 40)
	message_box.add_child(spacer1)

	# Message label
	var message_label = Label.new()
	message_label.text = message
	message_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	message_label.add_theme_font_size_override("font_size", 18)
	message_label.add_theme_color_override("font_color", Color.WHITE)
	message_box.add_child(message_label)

	# Spacer
	var spacer2 = Control.new()
	spacer2.custom_minimum_size = Vector2(0, 60)
	message_box.add_child(spacer2)

	# Reconnect button
	var reconnect_button = Button.new()
	reconnect_button.text = "RECONNECT"
	reconnect_button.custom_minimum_size = Vector2(200, 50)
	reconnect_button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	reconnect_button.pressed.connect(_on_reconnect_pressed)
	message_box.add_child(reconnect_button)

	# Small spacer between buttons
	var spacer3 = Control.new()
	spacer3.custom_minimum_size = Vector2(0, 10)
	message_box.add_child(spacer3)

	# Exit button
	var exit_button = Button.new()
	exit_button.text = "EXIT"
	exit_button.custom_minimum_size = Vector2(200, 50)
	exit_button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	exit_button.pressed.connect(func(): get_tree().quit())
	message_box.add_child(exit_button)


func _on_reconnect_pressed() -> void:
	var adapter_to_reconnect = _last_adapter_str if not _last_adapter_str.is_empty() else Global.comms.get_current_adapter_conn_str()

	# Remove overlay
	if _overlay != null and is_instance_valid(_overlay):
		_overlay.queue_free()
		_overlay = null

	# Reset state
	_reconnect_attempts = 0
	_last_adapter_str = ""

	# Reconnect
	if not adapter_to_reconnect.is_empty():
		Global.comms.change_adapter(adapter_to_reconnect)
