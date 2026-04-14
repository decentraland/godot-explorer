class_name PreviewWebSocket
extends Node

signal scene_update(scene_id: String)
signal scene_inspector_command(cmd: String, args: Dictionary, request_id: String)

## 64 MB outbound buffer — connections are local, never drop messages.
const OUTBOUND_BUFFER_SIZE := 64 * 1024 * 1024

var _ws := WebSocketPeer.new()
var _pending_url: String = ""
var _dirty_connected: bool = false
var _dirty_closed: bool = false


func set_url(url: String) -> void:
	_pending_url = (url.to_lower().replace("http://", "ws://").replace("https://", "wss://"))


func is_open() -> bool:
	return _ws.get_ready_state() == WebSocketPeer.STATE_OPEN


func send_json(msg: Dictionary) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(JSON.stringify(msg))


## Send a pre-serialized JSON string. Skips the Dictionary → JSON.stringify round
## trip — use for hot paths (e.g. scene inspector batches) where the payload is
## already a JSON string produced by Rust.
func send_raw_text(text: String) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(text)


func _process(_delta):
	_ws.poll()

	var state = _ws.get_ready_state()
	if state == WebSocketPeer.STATE_OPEN:
		if not _pending_url.is_empty():
			_ws.close()

		if _dirty_connected:
			_dirty_connected = false
			_dirty_closed = true

		while _ws.get_available_packet_count():
			var packet = _ws.get_packet().get_string_from_utf8()
			var json = JSON.parse_string(packet)
			if json != null and json is Dictionary:
				var msg_type = json.get("type", "")
				match msg_type:
					"SCENE_UPDATE":
						var scene_id = json.get("payload", {}).get("sceneId", "unknown")
						scene_update.emit(scene_id)
					"SCENE_INSPECTOR_CMD":
						var cmd: String = json.get("cmd", "")
						var args: Dictionary = json.get("args", {})
						var request_id: String = str(json.get("id", ""))
						scene_inspector_command.emit(cmd, args, request_id)
					_:
						printerr("preview-ws > unknown message type ", msg_type)

	elif state == WebSocketPeer.STATE_CLOSING:
		_dirty_closed = true
	elif state == WebSocketPeer.STATE_CLOSED:
		if _dirty_closed:
			_dirty_closed = false

		if not _pending_url.is_empty():
			_ws.set_outbound_buffer_size(OUTBOUND_BUFFER_SIZE)
			_ws.connect_to_url(_pending_url)
			_pending_url = ""
			_dirty_connected = true
