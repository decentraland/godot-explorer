class_name SceneLogWebSocket
extends Node

## WebSocket for sending scene log data and receiving commands from a dedicated target.
## Used when scene-logging=ws://host:port (not reusing the preview channel).

signal command_received(cmd: String, args: Dictionary, request_id: String)

## 64 MB outbound buffer — connections are local, never drop messages.
const OUTBOUND_BUFFER_SIZE := 64 * 1024 * 1024

var _ws := WebSocketPeer.new()
var _target_url: String = ""


func connect_to(url: String) -> void:
	# Close existing connection if switching to a different URL
	if not _target_url.is_empty() and _target_url != url:
		_ws.close()
		_ws = WebSocketPeer.new()
	_target_url = url
	_ws.set_outbound_buffer_size(OUTBOUND_BUFFER_SIZE)
	_ws.connect_to_url(_target_url)


func is_open() -> bool:
	return _ws.get_ready_state() == WebSocketPeer.STATE_OPEN


func send_json(msg: Dictionary) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(JSON.stringify(msg))


func _process(_delta):
	_ws.poll()

	# Handle incoming messages
	while _ws.get_available_packet_count() > 0:
		var packet := _ws.get_packet()
		var text := packet.get_string_from_utf8()
		_handle_message(text)

	# Auto-reconnect on disconnect
	if _ws.get_ready_state() == WebSocketPeer.STATE_CLOSED and not _target_url.is_empty():
		_ws.connect_to_url(_target_url)


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
