class_name SceneLogBridge
extends Node

## Bridges scene log entries from the Rust SceneLogDispatcher to output channels:
## - WebSocket (preview channel or dedicated target)
## - Godot Editor Debugger (EngineDebugger)

var _preview_ws: PreviewWebSocket
var _dedicated_ws: SceneLogWebSocket
var _use_debugger: bool = false
var _session_id: String = ""


func setup(scene_logging_target: String, preview_ws: PreviewWebSocket) -> void:
	_session_id = Global.scene_log_dispatcher.get_session_id()
	_use_debugger = EngineDebugger.is_active()

	if scene_logging_target == "true":
		# Use the existing preview WebSocket if available
		_preview_ws = preview_ws
	elif scene_logging_target.begins_with("ws://") or scene_logging_target.begins_with("wss://"):
		# Open a dedicated WebSocket to custom target
		_dedicated_ws = SceneLogWebSocket.new()
		_dedicated_ws.set_name("scene_log_ws")
		add_child(_dedicated_ws)
		_dedicated_ws.connect_to(scene_logging_target)

	Global.scene_log_dispatcher.scene_log_batch.connect(_on_batch)

	if _use_debugger:
		print("SceneLogBridge: Editor debugger channel active")
	if _preview_ws:
		print("SceneLogBridge: Preview WebSocket channel active")
	if _dedicated_ws:
		print("SceneLogBridge: Dedicated WebSocket channel -> ", scene_logging_target)


func _on_batch(entries_json: String) -> void:
	# WebSocket channel
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		ws_target.send_json(
			{
				"type": "SCENE_LOG",
				"payload": {"sessionId": _session_id, "entries": JSON.parse_string(entries_json)}
			}
		)

	# Editor debugger channel (batch as single message to avoid IPC flooding)
	if _use_debugger:
		EngineDebugger.send_message("dcl_scene_log:batch", [entries_json])
