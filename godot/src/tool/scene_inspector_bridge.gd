class_name SceneInspectorBridge
extends Node

## Bridges Scene Inspector entries from the Rust SceneInspectorDispatcher to
## output channels:
## - WebSocket (preview channel or dedicated target)
## Also handles incoming commands from external tools (SCENE_INSPECTOR_CMD protocol).

var _preview_ws: PreviewWebSocket
var _dedicated_ws: SceneInspectorWebSocket
var _session_id: String = ""


func setup(scene_inspector_target: String, preview_ws: PreviewWebSocket) -> void:
	_session_id = Global.scene_inspector_dispatcher.get_session_id()

	_connect_to_target(scene_inspector_target, preview_ws)

	Global.scene_inspector_dispatcher.scene_inspector_batch.connect(_on_batch)

	# Listen for deeplink changes to reconnect to new targets
	Global.deep_link_router.deep_link_received.connect(_on_deep_link_received)


func _connect_to_target(target: String, preview_ws: PreviewWebSocket = null) -> void:
	if target == "true":
		# Use the existing preview WebSocket if available
		if preview_ws and not _preview_ws:
			_preview_ws = preview_ws
			_preview_ws.scene_inspector_command.connect(_on_command)
	elif target.begins_with("ws://") or target.begins_with("wss://"):
		if not _dedicated_ws:
			_dedicated_ws = SceneInspectorWebSocket.new()
			_dedicated_ws.set_name("scene_inspector_ws")
			add_child(_dedicated_ws)
			_dedicated_ws.command_received.connect(_on_command)
		# Connect (or reconnect) to the target URL
		_dedicated_ws.connect_to(target)

	# Emit session_start entry so the receiver knows device info
	Global.scene_inspector_dispatcher.emit_session_start()

	if _preview_ws:
		print("SceneInspectorBridge: Preview WebSocket channel active")
	if _dedicated_ws:
		print("SceneInspectorBridge: Dedicated WebSocket channel -> ", target)

	# Hot-connect: send current CRDT state so inspector can reconstruct entity tree
	_send_crdt_snapshot()


func _on_deep_link_received() -> void:
	var new_target := Global.deep_link_obj.scene_inspector
	if new_target.is_empty():
		return
	# Reconnect dedicated WS to the new target
	if new_target.begins_with("ws://") or new_target.begins_with("wss://"):
		print("SceneInspectorBridge: Reconnecting to new target -> ", new_target)
		_connect_to_target(new_target)


func _on_batch(entries_json: String) -> void:
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		# `entries_json` is already valid JSON from Rust — splicing it into the
		# envelope as a literal avoids parse → re-stringify on every frame (up
		# to 500 entries each). `session_id` goes through JSON.stringify so any
		# special characters are properly escaped.
		var envelope := (
			'{"type":"SCENE_INSPECTOR","payload":{"sessionId":'
			+ JSON.stringify(_session_id)
			+ ',"entries":'
			+ entries_json
			+ "}}"
		)
		ws_target.send_raw_text(envelope)


func _on_command(cmd: String, args: Dictionary, request_id: String) -> void:
	var dispatcher = Global.scene_inspector_dispatcher
	var ok := true
	var data := {}

	match cmd:
		"pause":
			# Pause all scene processing (JS execution stops). Inspector streaming continues.
			_set_all_scenes_paused(true)
			dispatcher.set_paused(true)

		"resume":
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
				"scene_inspector_active": Global.scene_inspector_active,
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

		"set_include_bin_payload":
			# Opt-in to always attach raw hex payload alongside JSON. Off by default:
			# hex is redundant when JSON decodes, and doubles per-entry size.
			var enabled: bool = args.get("enabled", false)
			dispatcher.set_include_bin_payload(enabled)

		_:
			ok = false
			data = {"error": "unknown command: " + cmd}

	_send_ack(request_id, ok, data)


func _send_crdt_snapshot() -> void:
	var snapshot_json := Global.scene_inspector_dispatcher.get_crdt_snapshot_json()
	if snapshot_json.is_empty():
		return
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		var envelope := (
			'{"type":"SCENE_INSPECTOR","payload":{"sessionId":'
			+ JSON.stringify(_session_id)
			+ ',"entries":'
			+ snapshot_json
			+ "}}"
		)
		ws_target.send_raw_text(envelope)


func _set_all_scenes_paused(paused: bool) -> void:
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode:
			Global.scene_runner.set_scene_is_paused(child.get_scene_id(), paused)


func _send_ack(request_id: String, ok: bool, data: Dictionary) -> void:
	if request_id.is_empty():
		return
	var ws_target = _dedicated_ws if _dedicated_ws else _preview_ws
	if ws_target and ws_target.is_open():
		ws_target.send_json(
			{"type": "SCENE_INSPECTOR_CMD_ACK", "id": request_id, "ok": ok, "data": data}
		)
