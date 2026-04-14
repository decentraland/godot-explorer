class_name SceneLogWebSocket
extends Node

## WebSocket for sending scene log data and receiving commands from a dedicated target.
## Used when scene-logging=ws://host:port (not reusing the preview channel).

signal command_received(cmd: String, args: Dictionary, request_id: String)

## 64 MB outbound buffer — connections are local, never drop messages.
const OUTBOUND_BUFFER_SIZE := 64 * 1024 * 1024

## Exponential backoff for auto-reconnect: starts at 1s, doubles up to 30s.
const RECONNECT_INITIAL_DELAY := 1.0
const RECONNECT_MAX_DELAY := 30.0

var _ws := WebSocketPeer.new()
var _target_url: String = ""
var _reconnect_delay: float = RECONNECT_INITIAL_DELAY
var _reconnect_timer: float = 0.0
var _was_open: bool = false


func connect_to(url: String) -> void:
	# Close existing connection if switching to a different URL
	if not _target_url.is_empty() and _target_url != url:
		_ws.close()
		_ws = WebSocketPeer.new()
	_target_url = url
	_reconnect_delay = RECONNECT_INITIAL_DELAY
	_reconnect_timer = 0.0
	_was_open = false
	_ws.set_outbound_buffer_size(OUTBOUND_BUFFER_SIZE)
	_ws.connect_to_url(_target_url)


func is_open() -> bool:
	return _ws.get_ready_state() == WebSocketPeer.STATE_OPEN


func send_json(msg: Dictionary) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(JSON.stringify(msg))


func _process(delta):
	_ws.poll()

	# Handle incoming messages
	while _ws.get_available_packet_count() > 0:
		var packet := _ws.get_packet()
		var text := packet.get_string_from_utf8()
		_handle_message(text)

	# Reset backoff once a connection has succeeded
	var state := _ws.get_ready_state()
	if state == WebSocketPeer.STATE_OPEN:
		if not _was_open:
			_was_open = true
		_reconnect_delay = RECONNECT_INITIAL_DELAY
		_reconnect_timer = 0.0
		return

	# Auto-reconnect on disconnect with exponential backoff (1s → 2s → 4s … 30s).
	if state == WebSocketPeer.STATE_CLOSED and not _target_url.is_empty():
		_reconnect_timer += delta
		if _reconnect_timer >= _reconnect_delay:
			_reconnect_timer = 0.0
			_ws.connect_to_url(_target_url)
			_reconnect_delay = min(_reconnect_delay * 2.0, RECONNECT_MAX_DELAY)


func _handle_message(text: String) -> void:
	var parsed = JSON.parse_string(text)
	if parsed == null or not parsed is Dictionary:
		return
	var msg: Dictionary = parsed
	if msg.get("type") != "SCENE_LOG_CMD":
		return
	var cmd: String = msg.get("cmd", "")
	var args: Dictionary = msg.get("args", {})
	var request_id: String = str(msg.get("id", ""))
	command_received.emit(cmd, args, request_id)
