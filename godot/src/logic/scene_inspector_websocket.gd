class_name SceneInspectorWebSocket
extends Node

## WebSocket for sending Scene Inspector data and receiving commands from a
## dedicated target. Used when scene-inspector=ws://host:port (not reusing the
## preview channel).

signal command_received(cmd: String, args: Dictionary, request_id: String)
## Fired on the open/close transition so the bridge can gate capture on an
## actually-connected consumer (no buffering without a peer).
signal connected
signal disconnected

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


## Send a pre-serialized JSON string. Skips the Dictionary → JSON.stringify round
## trip — use for hot paths (e.g. scene inspector batches) where the payload is
## already a JSON string produced by Rust.
func send_raw_text(text: String) -> void:
	if _ws.get_ready_state() == WebSocketPeer.STATE_OPEN:
		_ws.send_text(text)


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
			connected.emit()
		_reconnect_delay = RECONNECT_INITIAL_DELAY
		_reconnect_timer = 0.0
		return

	# No longer open. If we just dropped, notify once so capture can be gated off.
	if _was_open:
		_was_open = false
		disconnected.emit()

	# Auto-reconnect on disconnect with exponential backoff (1s → 2s → 4s … 30s).
	if state == WebSocketPeer.STATE_CLOSED and not _target_url.is_empty():
		_reconnect_timer += delta
		if _reconnect_timer >= _reconnect_delay:
			_reconnect_timer = 0.0
			_ws.connect_to_url(_target_url)
			_reconnect_delay = min(_reconnect_delay * 2.0, RECONNECT_MAX_DELAY)


## Parse and dispatch an inbound packet. Two robustness concerns are handled here:
##
##   1. A packet can hold several JSON commands concatenated (the relay coalesces
##      consumer→device frames), so we split into balanced top-level {...} objects.
##   2. Godot's WebSocketPeer inbound path can hand us a byte-corrupted frame — a
##      brace duplicated at an object boundary (e.g. `...{"depth":1}}},"id":...`),
##      which prematurely closes the object and drops the trailing `id`, so the
##      ACK is never sent and the caller times out. When a split object closes
##      mid-frame (a leftover tail beginning with `,`), we splice the tail back on
##      after dropping the spurious closing brace and re-validate. Silent by
##      design: a repaired frame dispatches normally with no log noise. The proper
##      fix belongs in WebSocketPeer; this keeps the debug channel usable meanwhile.
func _handle_message(text: String) -> void:
	# Fast path: a single, uncorrupted frame parses as-is.
	var whole = _parse_silent(text)
	if whole is Dictionary and _dispatch_frame(whole):
		return
	for frame in _extract_command_frames(text):
		_dispatch_frame(frame)


## Parse JSON without polluting the error console. `JSON.parse_string` pushes an
## error on malformed input (that is the "Expected 'EOF'" spam corrupted frames
## produce); `JSON.new().parse()` returns an error code instead. Returns the
## decoded value on success, or null on any parse error.
func _parse_silent(text: String) -> Variant:
	var json := JSON.new()
	if json.parse(text) != OK:
		return null
	return json.data


## Extract candidate command objects from a raw packet, repairing the known
## boundary-brace corruption. Returns an Array of parsed Dictionaries.
func _extract_command_frames(text: String) -> Array:
	var out: Array = []
	var length := text.length()
	var cursor := 0
	while cursor < length:
		var span := _next_balanced_object(text, cursor)
		if span.is_empty():
			break
		var obj_start: int = span[0]
		var obj_end: int = span[1]  # index of the closing brace
		var obj_text := text.substr(obj_start, obj_end - obj_start + 1)
		var tail_start := obj_end + 1
		# A tail that resumes the SAME object (starts with `,` before the next
		# top-level `{`) is the corruption signature: the object closed early on a
		# duplicated brace. Splice the tail back on, minus that spurious brace.
		var tail := text.substr(tail_start).strip_edges()
		if tail.begins_with(","):
			var repaired := obj_text.substr(0, obj_text.length() - 1) + text.substr(tail_start)
			var reparsed = _parse_silent(repaired)
			if reparsed is Dictionary:
				out.append(reparsed)
				return out  # tail consumed by the repair
		var parsed = _parse_silent(obj_text)
		if parsed is Dictionary:
			out.append(parsed)
		cursor = obj_end + 1
	return out


## Find the next brace-balanced {...} object in `text` at or after `from`.
## Returns [start_index, end_index] (end = closing brace), or [] if none.
## String literals and escapes are respected so payload braces don't miscount.
func _next_balanced_object(text: String, from: int) -> Array:
	var depth := 0
	var start := -1
	var in_string := false
	var escaped := false
	for i in range(from, text.length()):
		var c := text[i]
		if in_string:
			if escaped:
				escaped = false
			elif c == "\\":
				escaped = true
			elif c == '"':
				in_string = false
			continue
		if c == '"':
			in_string = true
		elif c == "{":
			if depth == 0:
				start = i
			depth += 1
		elif c == "}":
			if depth > 0:
				depth -= 1
				if depth == 0 and start != -1:
					return [start, i]
	return []


## Validate a parsed frame and emit it as a command. Returns true if it was a
## well-formed inspector command (type + non-empty id), false otherwise — the
## caller uses that to decide whether to fall back to the repair path.
func _dispatch_frame(msg: Dictionary) -> bool:
	if msg.get("type") != "SCENE_INSPECTOR_CMD":
		return false
	var request_id: String = str(msg.get("id", ""))
	if request_id.is_empty():
		return false
	var cmd: String = msg.get("cmd", "")
	var args: Dictionary = msg.get("args", {})
	command_received.emit(cmd, args, request_id)
	return true
