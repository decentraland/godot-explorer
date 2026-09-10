class_name PreviewWebSocket
extends Node

## Emitted for the whole-scene reload path: legacy JSON `SCENE_UPDATE` frames
## and protobuf `WsSceneMessage.update_scene` frames both land here.
signal scene_update(scene_id: String)

## Emitted for protobuf `WsSceneMessage.update_model` frames — a single .glb/.gltf
## changed (`removed` = the file was deleted, i.e. `UMT_REMOVE`). `src` is the
## project-root-relative path; `hash` is derived from that path (not the file
## contents) so it stays the same across edits, and is empty when removed.
signal model_update(scene_id: String, src: String, hash: String, removed: bool)

## 64 MB outbound buffer — connections are local, never drop messages.
const OUTBOUND_BUFFER_SIZE := 64 * 1024 * 1024

var _ws := WebSocketPeer.new()
var _pending_url: String = ""
var _dirty_connected: bool = false
var _dirty_closed: bool = false

# The server sends the protobuf message AND the legacy JSON one for every change
# (`__LEGACY__updateScene`, kept until Bevy and Godot migrate). Acting on both
# would follow every precise model swap with a full scene reload, so once this
# server has proven it speaks protobuf, its legacy frames are dropped.
#
# Known gap from dropping them: the server debounces file events through a single
# shared timer (sdk-commands `logic/debounce.ts`, 800ms, last-args-win), so saving
# two models within that window reports only the last one. The legacy frame used
# to paper over that by reloading everything; now the first file stays stale until
# it is saved again. Worth fixing upstream with a per-path debounce.
#
# Measured on device (#2795): a Blender "glTF Separate" export writes .bin, then the
# textures, then the .gltf, all inside the same millisecond, so the whole export
# collapses into one message naming only the .gltf — which is harmless, because a
# .gltf falls through to a full reload_scene() that purges the entire mapping. The
# damage needs TWO models in one window: with a .glb written last the message names
# it, scene_fetcher takes the per-model fast path (force_reload_gltf on that one
# container), no purge runs, and every other model that changed is left stale with
# nothing logged. Reversing the write order heals it, which is the proof.
#
# This cannot be worked around on the client: preview hashes are path-derived, so a
# single updateModel is indistinguishable from one that stood in for several changed
# files. The fix belongs in sdk-commands
# `src/commands/start/server/file-watch-notifier.ts` — accumulate the changed set
# over the window and emit updateScene unless exactly one model changed.
var _speaks_protobuf: bool = false


func set_url(url: String) -> void:
	_pending_url = (url.to_lower().replace("http://", "ws://").replace("https://", "wss://"))


func is_open() -> bool:
	return _ws.get_ready_state() == WebSocketPeer.STATE_OPEN


func send_json(msg: Dictionary) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(JSON.stringify(msg))


## Legacy protocol (`__LEGACY__updateScene`), kept as a fallback for older CLIs.
## The server also sends a bare "update" text frame alongside the JSON one; it
## isn't valid JSON and is ignored here, same as before the protobuf migration.
func _handle_text_packet(packet: String) -> void:
	if _speaks_protobuf:
		return

	var json = JSON.parse_string(packet)
	if json == null or not json is Dictionary:
		return

	var msg_type = json.get("type", "")
	match msg_type:
		"SCENE_UPDATE":
			var scene_id = json.get("payload", {}).get("sceneId", "unknown")
			scene_update.emit(scene_id)
		_:
			printerr("preview-ws > unknown message type ", msg_type)


## Current protocol: protobuf `WsSceneMessage` (decentraland/sdk/development).
func _handle_binary_packet(packet: PackedByteArray) -> void:
	var msg := DclPreviewMessage.decode(packet)
	if msg.is_empty():
		printerr("preview-ws > undecodable binary frame (", packet.size(), " bytes)")
		return

	if not _speaks_protobuf:
		print("preview-ws > server speaks protobuf; ignoring its legacy JSON frames")
	_speaks_protobuf = true

	match msg.get("type", ""):
		"update_scene":
			scene_update.emit(msg.get("scene_id", "unknown"))
		"update_model":
			model_update.emit(
				msg.get("scene_id", ""),
				msg.get("src", ""),
				msg.get("hash", ""),
				msg.get("removed", false)
			)


func _process(_delta):
	_ws.poll()

	var state = _ws.get_ready_state()
	if state == WebSocketPeer.STATE_OPEN:
		if not _pending_url.is_empty():
			_ws.close()

		if _dirty_connected:
			_dirty_connected = false
			_dirty_closed = true
			print("preview-ws > connected, waiting for file-change events")

		while _ws.get_available_packet_count():
			var packet := _ws.get_packet()
			if _ws.was_string_packet():
				_handle_text_packet(packet.get_string_from_utf8())
			else:
				_handle_binary_packet(packet)

	elif state == WebSocketPeer.STATE_CLOSING:
		_dirty_closed = true
	elif state == WebSocketPeer.STATE_CLOSED:
		if _dirty_closed:
			_dirty_closed = false

		if not _pending_url.is_empty():
			_ws.set_outbound_buffer_size(OUTBOUND_BUFFER_SIZE)
			# Re-probed per connection: a reconnect may land on a different
			# (older) server than the one that spoke protobuf before.
			_speaks_protobuf = false
			_ws.connect_to_url(_pending_url)
			_pending_url = ""
			_dirty_connected = true
