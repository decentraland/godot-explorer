class_name SceneInspectorBridge
extends Node

## Bridges Scene Inspector entries from the Rust SceneInspectorDispatcher to a
## dedicated WebSocket target. Also handles incoming commands from external
## tools (SCENE_INSPECTOR_CMD protocol) — the single transport for the live
## client. Inspection/eval verbs run through the shared DebugWs command backend
## (`run_command`).

const Collector := preload("res://src/tool/debug_server/debug_collector.gd")

var _dedicated_ws: SceneInspectorWebSocket
var _session_id: String = ""


func setup(scene_inspector_target: String) -> void:
	_session_id = Global.scene_inspector_dispatcher.get_session_id()

	_connect_to_target(scene_inspector_target)

	Global.scene_inspector_dispatcher.scene_inspector_batch.connect(_on_batch)

	# Listen for deeplink changes to reconnect to new targets
	Global.deep_link_router.deep_link_received.connect(_on_deep_link_received)


func _connect_to_target(target: String) -> void:
	if not (target.begins_with("ws://") or target.begins_with("wss://")):
		printerr("SceneInspectorBridge: unsupported target %s (expected ws:// or wss://)" % target)
		return

	if not _dedicated_ws:
		_dedicated_ws = SceneInspectorWebSocket.new()
		_dedicated_ws.set_name("scene_inspector_ws")
		add_child(_dedicated_ws)
		_dedicated_ws.command_received.connect(_on_command)
		_dedicated_ws.connected.connect(_on_ws_connected)
		_dedicated_ws.disconnected.connect(_on_ws_disconnected)

	# Connect (or reconnect) to the target URL. session_start + the hot-connect
	# CRDT snapshot are sent from `_on_ws_connected`, once the socket is actually
	# open — sending them here (before the handshake completes) would drop them.
	_dedicated_ws.connect_to(target)

	print("SceneInspectorBridge: Dedicated WebSocket channel -> ", target)


func _on_ws_connected() -> void:
	# A consumer attached: open the master capture gate so producers may run.
	# Classic streams (crdt/lifecycle/perf) flow by default; logs/network stay
	# opt-in via `subscribe`.
	Global.scene_inspector_dispatcher.set_consumer_connected(true)
	Global.scene_inspector_dispatcher.emit_session_start()
	_send_crdt_snapshot()


func _on_ws_disconnected() -> void:
	# Consumer gone: close the gate so nothing keeps buffering without a peer.
	Global.scene_inspector_dispatcher.set_consumer_connected(false)


func _on_deep_link_received() -> void:
	var new_target := Global.deep_link_obj.scene_inspector
	if new_target.is_empty():
		return
	# Reconnect dedicated WS to the new target
	if new_target.begins_with("ws://") or new_target.begins_with("wss://"):
		print("SceneInspectorBridge: Reconnecting to new target -> ", new_target)
		_connect_to_target(new_target)


func _on_batch(entries_json: String) -> void:
	if _dedicated_ws and _dedicated_ws.is_open():
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
		_dedicated_ws.send_raw_text(envelope)


func _on_command(cmd: String, args: Dictionary, request_id: String) -> void:
	var dispatcher = Global.scene_inspector_dispatcher
	var ok := true
	# Untyped: shared-backend query results may be an Array (scenes / avatars).
	var data = {}

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

		"subscribe":
			# Opt in to high-volume streams (log / network) and the per-tick
			# lifecycle firehose. Nothing is captured until a consumer subscribes.
			_apply_subscribe(args, true)
			data = {"streams": args.get("streams", [])}

		"unsubscribe":
			_apply_subscribe(args, false)
			data = {"streams": args.get("streams", [])}

		_:
			# Inspection / eval verbs (ping, scenes, scene, entity, ui_scene,
			# ui_entity, avatars, avatar, app_ui, focus, eval) run through the
			# shared DebugWs command backend.
			var res: Dictionary = DebugWs.run_command(cmd, args)
			ok = res.get("ok", false)
			if ok:
				data = res.get("data", {})
			else:
				data = {"error": str(res.get("error", "unknown command: " + cmd))}

	_send_ack(request_id, ok, data)


## Apply (or revert) a `subscribe` / `unsubscribe` request. Maps stream names to
## the dispatcher's opt-in capture toggles. crdt / ops / perf are classic streams
## that flow whenever a consumer is connected, so they are not toggled here.
func _apply_subscribe(args: Dictionary, enable: bool) -> void:
	var dispatcher = Global.scene_inspector_dispatcher
	for s in args.get("streams", []):
		match str(s):
			"log", "logs":
				dispatcher.set_stream_logs(enable)
			"network":
				dispatcher.set_stream_network(enable)
			"lifecycle":
				dispatcher.set_lifecycle_verbose(enable)


func _send_crdt_snapshot() -> void:
	var snapshot_json := Global.scene_inspector_dispatcher.get_crdt_snapshot_json()
	if snapshot_json.is_empty():
		return
	if _dedicated_ws and _dedicated_ws.is_open():
		var envelope := (
			'{"type":"SCENE_INSPECTOR","payload":{"sessionId":'
			+ JSON.stringify(_session_id)
			+ ',"entries":'
			+ snapshot_json
			+ "}}"
		)
		_dedicated_ws.send_raw_text(envelope)


func _set_all_scenes_paused(paused: bool) -> void:
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode:
			Global.scene_runner.set_scene_is_paused(child.get_scene_id(), paused)


func _send_ack(request_id: String, ok: bool, data) -> void:
	if request_id.is_empty():
		return
	if _dedicated_ws and _dedicated_ws.is_open():
		_dedicated_ws.send_json(
			{"type": "SCENE_INSPECTOR_CMD_ACK", "id": request_id, "ok": ok, "data": data}
		)
