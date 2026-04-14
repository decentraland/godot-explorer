class_name SceneLogBridge
extends Node

## Bridges scene log entries from the Rust SceneLogDispatcher to output channels:
## - WebSocket (preview channel or dedicated target)
## Also handles incoming commands from external tools (SCENE_LOG_CMD protocol).

var _preview_ws: PreviewWebSocket
var _dedicated_ws: SceneLogWebSocket
var _session_id: String = ""


func setup(scene_logging_target: String, preview_ws: PreviewWebSocket) -> void:
	_session_id = Global.scene_log_dispatcher.get_session_id()

	_connect_to_target(scene_logging_target, preview_ws)

	Global.scene_log_dispatcher.scene_log_batch.connect(_on_batch)

	# Listen for deeplink changes to reconnect to new targets
	Global.deep_link_router.deep_link_received.connect(_on_deep_link_received)


func _connect_to_target(target: String, preview_ws: PreviewWebSocket = null) -> void:
	if target == "true":
		# Use the existing preview WebSocket if available
		if preview_ws and not _preview_ws:
			_preview_ws = preview_ws
			_preview_ws.scene_log_command.connect(_on_command)
	elif target.begins_with("ws://") or target.begins_with("wss://"):
		if not _dedicated_ws:
			_dedicated_ws = SceneLogWebSocket.new()
			_dedicated_ws.set_name("scene_log_ws")
			add_child(_dedicated_ws)
			_dedicated_ws.command_received.connect(_on_command)
		# Connect (or reconnect) to the target URL
		_dedicated_ws.connect_to(target)

	# Emit session_start entry so the receiver knows device info
	Global.scene_log_dispatcher.emit_session_start()

	if _preview_ws:
		print("SceneLogBridge: Preview WebSocket channel active")
	if _dedicated_ws:
		print("SceneLogBridge: Dedicated WebSocket channel -> ", target)

	# Hot-connect: send current CRDT state so inspector can reconstruct entity tree
	_send_crdt_snapshot()


func _on_deep_link_received() -> void:
	var new_target := Global.deep_link_obj.scene_logging
	if new_target.is_empty():
		return
	# Reconnect dedicated WS to the new target
	if new_target.begins_with("ws://") or new_target.begins_with("wss://"):
		print("SceneLogBridge: Reconnecting to new target -> ", new_target)
		_connect_to_target(new_target)


func _on_batch(entries_json: String) -> void:
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		ws_target.send_json(
			{
				"type": "SCENE_LOG",
				"payload": {"sessionId": _session_id, "entries": JSON.parse_string(entries_json)}
			}
		)


func _on_command(cmd: String, args: Dictionary, request_id: String) -> void:
	var dispatcher = Global.scene_log_dispatcher
	var ok := true
	var data := {}

	match cmd:
		"pause", "pause_logging":
			# Pause all scene processing (JS execution stops). Logging/streaming continues.
			if cmd == "pause_logging":
				push_warning("pause_logging is deprecated, use pause")
			_set_all_scenes_paused(true)
			dispatcher.set_paused(true)

		"resume", "resume_logging":
			if cmd == "resume_logging":
				push_warning("resume_logging is deprecated, use resume")
			_set_all_scenes_paused(false)
			dispatcher.set_paused(false)

		"reload_scene":
			# Reload the current realm (same mechanism as /reload chat command)
			if Global.realm:
				Global.realm.async_set_realm(Global.realm.get_realm_string())
			else:
				ok = false
				data = {"error": "no active realm"}

		"get_status":
			data = {
				"paused": dispatcher.is_paused(),
				"session_id": _session_id,
				"scene_logging_active": Global.scene_logging_active,
				"file_logging": dispatcher.is_file_logging(),
				"entry_count": dispatcher.get_entry_count(),
				"perf_interval": dispatcher.get_perf_interval(),
			}

		"set_file_logging":
			var enabled: bool = args.get("enabled", false)
			dispatcher.set_file_logging(enabled)

		"set_perf_interval":
			var seconds: float = args.get("seconds", 2.0)
			dispatcher.set_perf_interval(seconds)

		"set_lifecycle_verbose":
			# Suppress per-tick lifecycle entries while keeping CRDT/ops/log entries.
			var enabled: bool = args.get("enabled", true)
			dispatcher.set_lifecycle_verbose(enabled)

		_:
			ok = false
			data = {"error": "unknown command: " + cmd}

	_send_ack(request_id, ok, data)


func _send_crdt_snapshot() -> void:
	var snapshot_json := Global.scene_log_dispatcher.get_crdt_snapshot_json()
	if snapshot_json.is_empty():
		return
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		ws_target.send_json(
			{
				"type": "SCENE_LOG",
				"payload": {"sessionId": _session_id, "entries": JSON.parse_string(snapshot_json)}
			}
		)


func _set_all_scenes_paused(paused: bool) -> void:
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode:
			Global.scene_runner.set_scene_is_paused(child.get_scene_id(), paused)


func _send_ack(request_id: String, ok: bool, data: Dictionary) -> void:
	if request_id.is_empty():
		return
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		ws_target.send_json({"type": "SCENE_LOG_CMD_ACK", "id": request_id, "ok": ok, "data": data})
