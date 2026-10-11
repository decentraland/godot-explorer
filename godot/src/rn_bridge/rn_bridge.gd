extends Node
## VIBE-CODED SPIKE: JSON command/event channel for a React Native host (--rn-host only).
## Commands arrive as `{"type": ..., ...}` strings; events leave as `{"type": ..., "data": {...}}`.

const PLUGIN_SINGLETON := "RnHostPlugin"
const STATE_INTERVAL := 0.25
const HIDE_INTERVAL := 0.2

var last_event: Dictionary = {}

var _plugin: Object = null
var _explorer: Explorer = null
var _state_elapsed := 0.0
var _hide_elapsed := 0.0


func _ready() -> void:
	if not Global.cli.rn_host:
		set_process(false)
		return
	if Engine.has_singleton(PLUGIN_SINGLETON):
		_plugin = Engine.get_singleton(PLUGIN_SINGLETON)
		_plugin.connect("rn_command", handle_command)
	Global.loading_started.connect(_emit.bind("loading", {"state": "started"}))
	Global.loading_finished.connect(_emit.bind("loading", {"state": "finished"}))
	Global.on_chat_message.connect(_on_chat_message)


func _process(dt: float) -> void:
	if _explorer == null:
		_explorer = Global.get_explorer()
		if _explorer == null:
			return
		_emit("ready", {})
	_hide_elapsed += dt
	if _hide_elapsed >= HIDE_INTERVAL:
		_hide_elapsed = 0.0
		_hide_godot_ui()
	_state_elapsed += dt
	if _state_elapsed >= STATE_INTERVAL:
		_state_elapsed = 0.0
		_emit_state()


func handle_command(json: String) -> void:
	var cmd: Variant = JSON.parse_string(json)
	if not cmd is Dictionary or _explorer == null:
		return
	match cmd.get("type", ""):
		"teleport":
			_explorer.teleport_to(Vector2i(int(cmd.get("x", 0)), int(cmd.get("y", 0))))
		"send_chat":
			_explorer._on_panel_chat_submit_message(str(cmd.get("text", "")))
		"emote":
			_player().avatar.async_play_emote(str(cmd.get("urn", "")))


func _player() -> Player:
	return _explorer.player as Player


# Hide, never free: explorer.gd keeps @onready refs to these nodes. Scene UI, the joystick,
# the on-screen gamepad and the camera-look catcher stay native. UI root keeps focus or the
# player ignores movement input.
func _hide_godot_ui() -> void:
	var native_controls: Array[Node] = [
		_explorer.ui_safe_area,
		_explorer.safe_area_controls,
		_explorer.virtual_joystick,
		_explorer.mobile_camera_input,
	]
	for child in _explorer.ui_root.get_children():
		if child is CanvasItem and child.visible and not native_controls.has(child):
			child.hide()
	if not _explorer.ui_root.has_focus():
		_explorer.ui_root.grab_focus()


func _emit_state() -> void:
	var pos := _player().global_position
	_emit(
		"state",
		{
			"parcel": [_explorer.parcel_position.x, _explorer.parcel_position.y],
			"realm": Global.realm.get_realm_string(),
			"fps": Engine.get_frames_per_second(),
			"position": [pos.x, pos.y, pos.z],
		}
	)


func _on_chat_message(address: String, message: String, timestamp: float) -> void:
	_emit("chat", {"address": address, "message": message, "timestamp": timestamp})


func _emit(type: String, data: Dictionary) -> void:
	last_event = {"type": type, "data": data}
	if _plugin != null:
		_plugin.call("rn_event", JSON.stringify(last_event))
