extends PanelContainer

const MAX_PEER_ROWS := 10
const TOKEN_SUMMARY_CHARS := 16

var collapsed := false

@onready var rich_text_label: RichTextLabel = %RichTextLabel
@onready var collapse_button: Button = %CollapseButton
@onready var timer: Timer = %Timer


func _ready():
	timer.timeout.connect(_on_timer_timeout)
	collapse_button.pressed.connect(_on_collapse_button_pressed)
	timer.start()
	_on_timer_timeout()


func _on_collapse_button_pressed():
	collapsed = not collapsed
	rich_text_label.visible = not collapsed
	collapse_button.text = "▸" if collapsed else "▾"
	if not collapsed:
		_on_timer_timeout()
	# A Control never shrinks below its grown size on its own — snap the panel back to
	# its (now header-only) minimum, or the collapsed panel keeps the full height.
	_snap_height.call_deferred()


# Shrink only the height to the content's minimum. reset_size() must not be used
# here: both labels autowrap (minimum width ~0), so it would also collapse the
# scene's authored 440px width to the collapse button's width.
func _snap_height() -> void:
	size = Vector2(size.x, 0.0)


func _state_color(state: String) -> String:
	match state:
		"connected", "established":
			return "green"
		"disconnected", "dead", "none":
			return "red"
		"off", "unavailable", "disabled_for_session", "unknown":
			return "gray"
		_:
			# connecting / reconnecting / identifying / signing / idle / ...
			return "yellow"


func _colored_state(state: String) -> String:
	return "[color=%s]%s[/color]" % [_state_color(state), state]


func _short_address(address: String) -> String:
	if address.length() <= 12:
		return address
	return address.substr(0, 6) + ".." + address.right(4)


# Long credentials (LiveKit JWTs) blow up the adapter line — keep the first
# TOKEN_SUMMARY_CHARS characters of the token value and elide the rest.
func _summarize_access_token(adapter: String) -> String:
	var key := "access_token="
	var idx := adapter.find(key)
	if idx == -1:
		return adapter
	var start := idx + key.length()
	var end := adapter.find("&", start)
	if end == -1:
		end = adapter.length()
	if end - start <= TOKEN_SUMMARY_CHARS:
		return adapter
	return adapter.substr(0, start + TOKEN_SUMMARY_CHARS) + "..." + adapter.substr(end)


# Peer names come from the wire — escape the bbcode delimiter so a bracket in a
# name can't inject markup into the RichTextLabel.
func _escape_bbcode(text_value: String) -> String:
	return text_value.replace("[", "[lb]")


func _on_timer_timeout():
	if collapsed:
		return
	if not is_instance_valid(Global.comms):
		return

	var info: Dictionary = Global.comms.get_debug_room_info()
	var lines := PackedStringArray()

	var realm_name := ""
	if is_instance_valid(Global.realm):
		realm_name = String(Global.realm.realm_name)
	if not realm_name.is_empty():
		lines.append("Realm: %s" % _escape_bbcode(realm_name))

	# Server-derived strings are escaped like peer names — a stray '[' corrupts the markup.
	var adapter: String = _escape_bbcode(_summarize_access_token(info.get("adapter", "")))
	var connection_state: String = info.get("connection_state", "unknown")
	lines.append("Adapter: %s [%s]" % [adapter, connection_state])

	if info.get("comms_on_hold", false):
		lines.append("[color=yellow]COMMS ON HOLD[/color]")

	if not info.get("livekit_enabled", true):
		lines.append("[color=orange]LIVEKIT DISABLED (pulse-only mode)[/color]")

	var main_room_type: String = info.get("main_room_type", "none")
	var main_room_state: String = info.get("main_room_state", "none")
	# In archipelago mode the island room (below) is the effective main room
	if main_room_type != "none" or connection_state != "archipelago":
		lines.append("Main room (%s): %s" % [main_room_type, _colored_state(main_room_state)])

	if connection_state == "archipelago":
		var archipelago_state: String = info.get("archipelago_state", "none")
		var island_id: String = info.get("island_id", "")
		var island_room_state: String = info.get("island_room_state", "none")
		var island_label := _escape_bbcode(island_id) if not island_id.is_empty() else "-"
		lines.append(
			(
				"Archipelago: %s | island %s (%s)"
				% [
					_colored_state(archipelago_state),
					island_label,
					_colored_state(island_room_state)
				]
			)
		)

	var scene_room: String = info.get("scene_room", "")
	var scene_room_state: String = info.get("scene_room_state", "none")
	var scene_label := _escape_bbcode(scene_room) if not scene_room.is_empty() else "-"
	lines.append("Scene room: %s %s" % [scene_label, _colored_state(scene_room_state)])

	lines.append(_build_pulse_line(info))
	if info.get("pulse_disabled_for_session", false):
		lines.append("[color=red]Pulse disabled for session[/color]")

	lines.append_array(_peer_lines())

	rich_text_label.text = "\n".join(lines)
	# Shrink back when the content got shorter (fewer peers/lines) — a Control keeps
	# its grown size otherwise.
	_snap_height.call_deferred()


func _build_pulse_line(info: Dictionary) -> String:
	if not info.get("pulse_available", false):
		return "Pulse: [color=gray]not in build[/color]"
	if not info.get("pulse_enabled", false):
		return "Pulse: [color=gray]OFF[/color]"

	var pulse_state: String = info.get("pulse_state", "unavailable")
	var pulse_endpoint: String = info.get("pulse_endpoint", "")
	var pulse_failures: int = info.get("pulse_failures", 0)
	var dual_channel_label: String = "ON" if info.get("dual_channel", true) else "OFF"
	# The realm is matched exactly and never exchanged, so a wrong one looks identical to an
	# empty world — worth reading off the panel before debugging anything else.
	var pulse_realm: String = info.get("pulse_realm", "")
	if pulse_realm.is_empty():
		pulse_realm = "[color=gray]pending[/color]"
	return (
		"Pulse: %s @ %s | realm %s | fails %d | dual-ch %s"
		% [
			_colored_state(pulse_state),
			pulse_endpoint,
			pulse_realm,
			pulse_failures,
			dual_channel_label
		]
	)


func _peer_lines() -> PackedStringArray:
	var lines := PackedStringArray()
	var peers: Array = Global.comms.get_debug_peer_rooms()
	lines.append("Peers: %d" % peers.size())
	var shown: int = mini(peers.size(), MAX_PEER_ROWS)
	for i in range(shown):
		var peer: Dictionary = peers[i]
		var address: String = peer.get("address", "")
		var rooms: String = peer.get("rooms", "")
		var peer_name: String = peer.get("name", "")
		var who := _short_address(address)
		if not peer_name.is_empty():
			who = "%s %s" % [_escape_bbcode(peer_name), who]
		lines.append("  %s  %s" % [who, rooms])
	if peers.size() > MAX_PEER_ROWS:
		lines.append("  … +%d more" % (peers.size() - MAX_PEER_ROWS))
	return lines
