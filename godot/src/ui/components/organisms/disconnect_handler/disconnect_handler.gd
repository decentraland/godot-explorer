class_name DisconnectHandler
extends Node

## Handles comms disconnection with automatic reconnection attempts
## and shows a modal (via ModalManager) when reconnection fails.

## Disconnect reasons from CommunicationManager (must match Rust DisconnectReason enum)
const REASON_DUPLICATE_IDENTITY: int = 0
const REASON_ROOM_CLOSED: int = 1
const REASON_KICKED: int = 2
const REASON_OTHER: int = 3

const MAX_RECONNECT_ATTEMPTS: int = 3

var _reconnect_attempts: int = 0
var _last_adapter_str: String = ""
var _last_disconnect_reason: int = -1
var _should_stop_reconnecting: bool = false
var _scene_banned: bool = false
var _is_loading: bool = false


func _ready() -> void:
	Global.comms.disconnected.connect(_on_disconnected)
	Global.comms.on_adapter_changed.connect(_on_adapter_changed)
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)
	Global.on_menu_close.connect(_async_on_menu_close_ban_recheck)
	Global.modal_manager.session_ended_sign_in.connect(_on_session_ended_sign_in)
	Global.modal_manager.session_ended_retry.connect(_on_session_ended_retry)
	Global.modal_manager.session_ended_exit.connect(_on_session_ended_exit)


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
	_scene_banned = false


func _on_disconnected(reason: int) -> void:
	# Store the adapter string FIRST before any cleanup (might be cleared by clean())
	if _last_adapter_str.is_empty():
		_last_adapter_str = Global.comms.get_current_adapter_conn_str()

	# DuplicateIdentity means someone else logged in - don't retry, show error immediately
	if reason == REASON_DUPLICATE_IDENTITY:
		# Note: Don't reset _last_adapter_str here - we need it for reconnection
		_reconnect_attempts = 0
		_last_disconnect_reason = -1
		_should_stop_reconnecting = true  # Stop any pending reconnect attempts
		_show_disconnect_modal(reason)
		return

	# Kicked/Banned - don't retry, show ban modal (defer if still loading)
	if reason == REASON_KICKED:
		_should_stop_reconnecting = true
		_scene_banned = true
		if not _is_loading:
			Global.modal_manager.async_show_ban_kicked_modal()
		return

	_last_disconnect_reason = reason
	_reconnect_attempts += 1
	_should_stop_reconnecting = false  # Allow reconnection attempts

	print(
		(
			"[DisconnectHandler] Disconnected (reason: %d), attempt %d/%d"
			% [reason, _reconnect_attempts, MAX_RECONNECT_ATTEMPTS]
		)
	)

	# If we haven't exhausted reconnect attempts, try to reconnect
	if _reconnect_attempts < MAX_RECONNECT_ATTEMPTS and not _last_adapter_str.is_empty():
		print("[DisconnectHandler] Attempting to reconnect...")
		_async_attempt_reconnect()
		return

	# Exhausted all attempts - show modal
	_show_disconnect_modal(reason)


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


func _show_disconnect_modal(reason: int) -> void:
	match reason:
		REASON_DUPLICATE_IDENTITY:
			Global.modal_manager.async_show_session_ended_modal()
		REASON_ROOM_CLOSED:
			Global.modal_manager.async_show_room_closed_modal()
		_:
			Global.modal_manager.async_show_disconnected_modal()


func _on_session_ended_sign_in() -> void:
	# DuplicateIdentity: another client took over this account. Sign the user out
	# and route to the lobby so they can re-authenticate (possibly with a different account).
	_reset_reconnect_state()
	Global.sign_out()


func _on_session_ended_retry() -> void:
	var adapter_to_reconnect = (
		_last_adapter_str
		if not _last_adapter_str.is_empty()
		else Global.comms.get_current_adapter_conn_str()
	)

	# Reset state
	_reconnect_attempts = 0
	_last_adapter_str = ""

	# Reconnect
	if not adapter_to_reconnect.is_empty():
		Global.comms.change_adapter(adapter_to_reconnect)


func _on_session_ended_exit() -> void:
	get_tree().quit()


func _on_loading_started() -> void:
	_is_loading = true
	# Close any visible ban modal during loading, but keep _scene_banned
	# so _on_loading_finished can re-show the deferred modal.
	Global.modal_manager.close_current_modal()


func _on_loading_finished() -> void:
	_is_loading = false

	# If kicked during loading (scene room 403 or room metadata ban), show the deferred modal now
	if _scene_banned:
		Global.modal_manager.async_show_ban_kicked_modal()


## Re-check ban when the user closes the menu (back from discover).
## Waits one frame so loading_started can clear _scene_banned if the user navigated.
func _async_on_menu_close_ban_recheck() -> void:
	if not _scene_banned:
		return
	await get_tree().process_frame
	if not _scene_banned or _is_loading:
		return
	# Clear suppress — this is an intentional re-show, not a stale signal.
	Global.modal_manager.clear_suppress_ban_kicked()
	Global.modal_manager.async_show_ban_kicked_modal()
