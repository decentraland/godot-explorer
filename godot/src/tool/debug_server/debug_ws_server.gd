class_name DebugWsServer
extends Node

## Developer-only WebSocket server that returns rich JSON snapshots of loaded
## scenes / entities on demand. Off by default — toggled from the Developer
## section of Settings. Bound to loopback only.
##
## Protocol: each text frame is a JSON object with at least `id` and `cmd`.
## Reply: `{"id": <id>, "ok": <bool>, "data": ...}` or `{"id": <id>, "ok": false, "error": "..."}`.
##
## Implementation: TCPServer + WebSocketPeer.accept_stream() — a vanilla
## bidirectional WebSocket, NOT the multiplayer routing layer (which injects
## peer-id sys packets that confuse generic clients like websocat / browsers).
##
## Supported commands:
##   ping
##   scenes
##   scene     {scene_id, filters}                — 3D entity tree
##   entity    {scene_id, entity_id, filters}     — 3D entity tree
##   ui_scene  {scene_id, filters}                — 2D UI entity tree
##   ui_entity {scene_id, entity_id, filters}     — 2D UI entity tree
##   avatars                                       — list every tracked avatar
##   avatar    {by, value, filters}                — one avatar's detail
##   app_ui    {filters}                            — Explorer's own UI tree
##                                                    (skips per-scene SDK UI
##                                                    subtree by default).
##
## `ui_scene` / `ui_entity` are identical to their 3D counterparts except the
## `godot` block reports the rendered `Control` (rect, anchors, modulate, …)
## instead of the `Node3D`. The SDK payload is unchanged — UI and 3D entities
## share the same per-scene entity-id space.
##
## `avatars` / `avatar` query the global `AvatarScene` (not per-scene CRDT).
## Identify a single avatar via `by` ∈ {"address", "alias", "entity"} +
## `value`. The avatar tree uses its own SceneEntityId space; the entity_id
## here is not addressable through the `entity` command.

const DEFAULT_PORT: int = 9230
const DEFAULT_BIND: String = "127.0.0.1"
const MAX_FRAME_BYTES: int = 65536  ## drop inbound frames larger than this
## Per-peer outbound buffer. Godot's default is 64 KiB, which a single `scene`
## or `app_ui` reply trivially exceeds — `send_text` then returns
## ERR_OUT_OF_MEMORY and the reply is silently dropped, leaving the client
## hanging. 8 MiB comfortably fits expanded scene snapshots.
const OUTBOUND_BUFFER_BYTES: int = 8 * 1024 * 1024
## Max focus-change entries retained for the `focus` diagnostic cmd.
const FOCUS_HISTORY_MAX: int = 64
const Collector := preload("res://src/tool/debug_server/debug_collector.gd")

var _tcp: TCPServer
var _peers: Array[WebSocketPeer] = []
var _running: bool = false
var _port: int = DEFAULT_PORT

## Focus tracking: poll the viewport's keyboard-focus owner each frame and log
## every change (including release-to-null, which `gui_focus_changed` misses).
## Exposed via the `focus` cmd. Diagnostic aid for "input stops working" bugs
## where movement is gated by `ui_root.has_focus()`.
var _focus_history: Array = []
var _last_focus_desc: String = "<unset>"


func _ready() -> void:
	# Off by default; settings toggle calls start() / stop().
	set_process(false)


func is_running() -> bool:
	return _running


func get_port() -> int:
	return _port


func start(port: int = DEFAULT_PORT, bind_address: String = DEFAULT_BIND) -> bool:
	if _running:
		return true
	_tcp = TCPServer.new()
	var err := _tcp.listen(port, bind_address)
	if err != OK:
		printerr("DebugWsServer: failed to bind tcp://%s:%d (err=%d)" % [bind_address, port, err])
		_tcp = null
		return false
	_port = port
	_running = true
	set_process(true)
	print("DebugWsServer: listening on ws://%s:%d" % [bind_address, port])
	return true


func stop() -> void:
	if not _running:
		return
	set_process(false)
	for peer in _peers:
		peer.close()
	_peers.clear()
	if _tcp != null:
		_tcp.stop()
		_tcp = null
	_running = false
	print("DebugWsServer: stopped")


func _process(_dt: float) -> void:
	_poll_focus()
	if _tcp == null:
		return

	# Accept new connections: wrap each TCP stream in a WebSocketPeer that runs
	# its own RFC6455 handshake.
	while _tcp.is_connection_available():
		var stream := _tcp.take_connection()
		var peer := WebSocketPeer.new()
		peer.set_outbound_buffer_size(OUTBOUND_BUFFER_BYTES)
		var err := peer.accept_stream(stream)
		if err != OK:
			printerr("DebugWsServer: accept_stream failed err=%d" % err)
			continue
		_peers.append(peer)

	# Drive each peer one frame: poll, drain inbound packets, prune closed ones.
	for i in range(_peers.size() - 1, -1, -1):
		var peer: WebSocketPeer = _peers[i]
		peer.poll()
		var state := peer.get_ready_state()
		if state == WebSocketPeer.STATE_OPEN:
			while peer.get_available_packet_count() > 0:
				var raw := peer.get_packet()
				if raw.size() > MAX_FRAME_BYTES:
					_send(
						peer,
						{
							"id": null,
							"ok": false,
							"error": "frame too large (max %d bytes)" % MAX_FRAME_BYTES
						}
					)
					continue
				_handle_message(peer, raw.get_string_from_utf8())
		elif state == WebSocketPeer.STATE_CLOSED:
			_peers.remove_at(i)


# --------------------------------------------------------------------
# Dispatch


func _handle_message(peer: WebSocketPeer, text: String) -> void:
	var parsed = JSON.parse_string(text)
	if not (parsed is Dictionary):
		_send(peer, {"id": null, "ok": false, "error": "expected a JSON object per frame"})
		return
	var msg: Dictionary = parsed
	var request_id = msg.get("id", null)
	var cmd: String = str(msg.get("cmd", ""))
	if cmd.is_empty():
		_reply(peer, request_id, false, null, "missing 'cmd'")
		return

	match cmd:
		"ping":
			_reply(peer, request_id, true, _build_ping_data(), "")
		"focus":
			_reply(peer, request_id, true, _build_focus_data(), "")
		"scenes":
			_reply(peer, request_id, true, Collector.collect_scenes_summary(), "")
		"scene":
			var scene_id := int(msg.get("scene_id", -1))
			if scene_id < 0:
				_reply(peer, request_id, false, null, "missing 'scene_id'")
				return
			var filters: Dictionary = msg.get("filters", {})
			var data: Dictionary = Collector.collect_scene(scene_id, filters)
			if data.has("error"):
				_reply(peer, request_id, false, null, str(data["error"]))
			else:
				_reply(peer, request_id, true, data, "")
		"entity":
			var scene_id_e := int(msg.get("scene_id", -1))
			var entity_id := int(msg.get("entity_id", -1))
			if scene_id_e < 0 or entity_id < 0:
				_reply(peer, request_id, false, null, "missing 'scene_id' or 'entity_id'")
				return
			# `entity` and `scene` both take the same `filters` dict.
			# Backwards-compat: also accept `include_parents` / `include_children`
			# at the top level of the message, where the old `entity` API put them.
			var filters_e: Dictionary = (msg.get("filters", {}) as Dictionary).duplicate()
			if msg.has("include_parents") and not filters_e.has("include_parents"):
				filters_e["include_parents"] = msg["include_parents"]
			if msg.has("include_children") and not filters_e.has("include_children"):
				filters_e["include_children"] = msg["include_children"]
			var data_e: Dictionary = Collector.collect_entity(scene_id_e, entity_id, filters_e)
			if data_e.has("error"):
				_reply(peer, request_id, false, null, str(data_e["error"]))
			else:
				_reply(peer, request_id, true, data_e, "")
		"ui_scene":
			var ui_scene_id := int(msg.get("scene_id", -1))
			if ui_scene_id < 0:
				_reply(peer, request_id, false, null, "missing 'scene_id'")
				return
			var ui_filters: Dictionary = (msg.get("filters", {}) as Dictionary).duplicate()
			ui_filters["tree"] = "ui"
			var ui_data: Dictionary = Collector.collect_scene(ui_scene_id, ui_filters)
			if ui_data.has("error"):
				_reply(peer, request_id, false, null, str(ui_data["error"]))
			else:
				_reply(peer, request_id, true, ui_data, "")
		"ui_entity":
			var ui_sid := int(msg.get("scene_id", -1))
			var ui_eid := int(msg.get("entity_id", -1))
			if ui_sid < 0 or ui_eid < 0:
				_reply(peer, request_id, false, null, "missing 'scene_id' or 'entity_id'")
				return
			var ui_ef: Dictionary = (msg.get("filters", {}) as Dictionary).duplicate()
			ui_ef["tree"] = "ui"
			var ui_de: Dictionary = Collector.collect_entity(ui_sid, ui_eid, ui_ef)
			if ui_de.has("error"):
				_reply(peer, request_id, false, null, str(ui_de["error"]))
			else:
				_reply(peer, request_id, true, ui_de, "")
		"avatars":
			_reply(peer, request_id, true, Collector.collect_avatars(), "")
		"app_ui":
			var app_filters: Dictionary = (msg.get("filters", {}) as Dictionary).duplicate()
			var app_data: Dictionary = Collector.collect_app_ui(app_filters)
			if app_data.has("error"):
				_reply(peer, request_id, false, null, str(app_data["error"]))
			else:
				_reply(peer, request_id, true, app_data, "")
		"avatar":
			var by: String = str(msg.get("by", ""))
			if by.is_empty():
				_reply(
					peer,
					request_id,
					false,
					null,
					"missing 'by' (expected address|alias|entity|local)"
				)
				return
			# `local` is keyless — all other modes require `value`.
			if by != "local" and not msg.has("value"):
				_reply(peer, request_id, false, null, "missing 'value'")
				return
			var a_filters: Dictionary = (msg.get("filters", {}) as Dictionary).duplicate()
			var av_value: Variant = msg.get("value", null)
			var av: Dictionary = Collector.collect_avatar(by, av_value, a_filters)
			if av.has("error"):
				_reply(peer, request_id, false, null, str(av["error"]))
			else:
				_reply(peer, request_id, true, av, "")
		_:
			_reply(peer, request_id, false, null, "unknown command: %s" % cmd)


func _poll_focus() -> void:
	var tree := get_tree()
	if tree == null:
		return
	var vp := tree.root
	if vp == null:
		return
	var owner := vp.gui_get_focus_owner()
	var desc := _describe_focus(owner)
	if desc == _last_focus_desc:
		return
	(
		_focus_history
		. append(
			{
				"t_ms": Time.get_ticks_msec(),
				"frame": Engine.get_process_frames(),
				"from": _last_focus_desc,
				"to": desc,
			}
		)
	)
	if _focus_history.size() > FOCUS_HISTORY_MAX:
		_focus_history = _focus_history.slice(_focus_history.size() - FOCUS_HISTORY_MAX)
	_last_focus_desc = desc


func _describe_focus(node: Control) -> String:
	if node == null:
		return "<none>"
	return "%s [%s]" % [str(node.get_path()), node.get_class()]


func _build_focus_data() -> Dictionary:
	# `explorer_has_focus` (and thus mobile walk/jump) is true iff this matches
	# the explorer's `ui_root` (%UI). Compare `current` against it.
	var ui_root_path := "<no explorer>"
	var explorer := get_node_or_null("/root/explorer")
	if explorer != null and explorer.get("ui_root") != null:
		ui_root_path = str(explorer.ui_root.get_path())
	return {
		"current": _last_focus_desc,
		"ui_root_path": ui_root_path,
		"history": _focus_history,
	}


func _build_ping_data() -> Dictionary:
	var version: String = str(ProjectSettings.get_setting("application/config/version", "unknown"))
	var loaded: PackedInt32Array
	if is_instance_valid(Global.scene_runner):
		loaded = Global.scene_runner.debug_get_loaded_scene_ids()
	return {
		"version": version,
		"engine": Engine.get_version_info().get("string", ""),
		"scenes_loaded": loaded.size(),
	}


# --------------------------------------------------------------------
# Reply helpers


func _reply(peer: WebSocketPeer, request_id, ok: bool, data, err_msg: String) -> void:
	var reply: Dictionary = {"id": request_id, "ok": ok}
	if ok:
		reply["data"] = data
	else:
		reply["error"] = err_msg
	_send(peer, reply)


func _send(peer: WebSocketPeer, reply: Dictionary) -> void:
	if peer.get_ready_state() != WebSocketPeer.STATE_OPEN:
		return
	var payload := JSON.stringify(reply)
	var err := peer.send_text(payload)
	if err == OK:
		return
	# Most common cause: payload exceeds `outbound_buffer_size`. The failed
	# message is not enqueued, so a tiny replacement frame still fits and the
	# client gets a usable error instead of hanging on a dropped reply.
	var fallback := (
		JSON
		. stringify(
			{
				"id": reply.get("id", null),
				"ok": false,
				"error":
				(
					(
						"reply dropped (err=%d, payload=%d bytes, buffer=%d). "
						+ "Narrow `filters` (e.g. add `component`, set `include_children:false`)."
					)
					% [err, payload.length(), OUTBOUND_BUFFER_BYTES]
				),
			}
		)
	)
	peer.send_text(fallback)
