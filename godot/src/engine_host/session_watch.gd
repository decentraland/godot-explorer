class_name SessionWatch
extends Node
## Reconnects a dropped comms session and tells the shell when it could not: the account logged in
## elsewhere, the room closed, the connection did not come back, or the scene's room banned the player.

## Disconnect reasons from CommunicationManager (must match the Rust DisconnectReason enum)
const REASON_DUPLICATE_IDENTITY := 0
const REASON_ROOM_CLOSED := 1
const REASON_KICKED := 2

const MAX_RECONNECT_ATTEMPTS := 3

var _attempts := 0
var _adapter := ""
var _stop := false
var _banned := false
var _loading := false


func _ready() -> void:
	Global.comms.disconnected.connect(_on_disconnected)
	Global.comms.on_adapter_changed.connect(_on_adapter_changed)
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)


## The player chose to try again after a disconnect notice.
func reconnect() -> void:
	var adapter := (
		_adapter if not _adapter.is_empty() else Global.comms.get_current_adapter_conn_str()
	)
	_attempts = 0
	_adapter = ""
	if not adapter.is_empty():
		Global.comms.change_adapter(adapter)


func _on_adapter_changed(_voice_chat_enabled: bool, _adapter_str: String) -> void:
	_attempts = 0
	_adapter = ""
	_stop = false
	_banned = false


func _on_disconnected(reason: int) -> void:
	# Taken before any cleanup clears it.
	if _adapter.is_empty():
		_adapter = Global.comms.get_current_adapter_conn_str()

	if reason == REASON_DUPLICATE_IDENTITY:
		_attempts = 0
		_stop = true
		Global.engine.report_notice("disconnected_duplicate", "")
		return

	if reason == REASON_KICKED:
		_stop = true
		_banned = true
		if not _loading:
			Global.engine.report_notice("banned", "")
		return

	_attempts += 1
	_stop = false
	if _attempts < MAX_RECONNECT_ATTEMPTS and not _adapter.is_empty():
		_async_reconnect()
		return

	Global.engine.report_notice(
		"disconnected_room_closed" if reason == REASON_ROOM_CLOSED else "disconnected_other", ""
	)


func _async_reconnect() -> void:
	# A pause between tries so a flapping connection does not spin.
	await get_tree().create_timer(1.0).timeout
	if _stop or _adapter.is_empty():
		return
	Global.comms.change_adapter(_adapter)


func _on_loading_started() -> void:
	_loading = true


func _on_loading_finished() -> void:
	_loading = false
	# Kicked while loading: the notice waited for the world.
	if _banned:
		Global.engine.report_notice("banned", "")
