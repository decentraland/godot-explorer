class_name SceneLogWebSocket
extends Node

## Minimal WebSocket for sending scene log data to a dedicated target.
## Used when scene-logging=ws://host:port (not reusing the preview channel).

var _ws := WebSocketPeer.new()
var _target_url: String = ""


func connect_to(url: String) -> void:
	_target_url = url
	_ws.connect_to_url(_target_url)


func is_open() -> bool:
	return _ws.get_ready_state() == WebSocketPeer.STATE_OPEN


func send_json(msg: Dictionary) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(JSON.stringify(msg))


func _process(_delta):
	_ws.poll()

	# Auto-reconnect on disconnect
	if _ws.get_ready_state() == WebSocketPeer.STATE_CLOSED and not _target_url.is_empty():
		_ws.connect_to_url(_target_url)
