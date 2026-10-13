extends Node
## JSON command/event channel for a React Native host (--rn-host only).
## Commands arrive as `{"type": ..., ...}` strings; events leave as `{"type": ..., "data": {...}}`.

const PLUGIN_SINGLETON := "RnHostPlugin"
const STATE_INTERVAL := 0.25
const HIDE_INTERVAL := 0.2

var last_event: Dictionary = {}

var _plugin: Object = null
var _explorer: Explorer = null
var _state_elapsed := 0.0
var _hide_elapsed := 0.0
var _backpack := RnBackpack.new()
var _requests: RnRequests
var _hud: RnHud
var _avatar_host_up := false
var _in_world := false
var _exiting := false
var _entering := false
var _pending_enter: Dictionary = {}


func _ready() -> void:
	if not Global.cli.rn_host:
		set_process(false)
		return
	if Engine.has_singleton(PLUGIN_SINGLETON):
		_plugin = Engine.get_singleton(PLUGIN_SINGLETON)
		_plugin.connect("rn_command", handle_command)
	Global.loading_started.connect(_emit.bind("loading", {"state": "started"}))
	Global.loading_finished.connect(_emit.bind("loading", {"state": "finished"}))
	_requests = RnRequests.new(_emit)
	_hud = RnHud.new(_emit)
	_requests.connect_signals()
	_hud.connect_signals()
	_backpack.avatar_changed.connect(func(state: Dictionary) -> void: _emit("avatar", state))
	add_child(_backpack)


## Called by the avatar-only host scene once the identity and profile exist.
func on_avatar_host_ready() -> void:
	_avatar_host_up = true
	_exiting = false
	if _in_world:
		_in_world = false
		_emit("world", {"state": "exited"})
	_emit("ready", {})
	if not _pending_enter.is_empty():
		var pending := _pending_enter
		_pending_enter = {}
		_handle_avatar_command(pending)


func _process(dt: float) -> void:
	if _avatar_host_up:
		return
	if _explorer == null:
		_explorer = Global.get_explorer()
		if _explorer == null:
			return
		_emit("world", {"state": "entered"})
		if not Global.cli.has_arg("--rn-avatar-only"):
			_emit("ready", {})
	_hide_elapsed += dt
	if _hide_elapsed >= HIDE_INTERVAL:
		_hide_elapsed = 0.0
		RnUiFilter.hide_non_native(_explorer.ui_root)
	_state_elapsed += dt
	if _state_elapsed >= STATE_INTERVAL:
		_state_elapsed = 0.0
		_emit_state()


func handle_command(json: String) -> void:
	var cmd: Variant = JSON.parse_string(json)
	if not cmd is Dictionary:
		return
	if _requests.handle(cmd):
		return
	if cmd.get("type", "") == "focus_camera":
		_backpack.focus_camera(str(cmd.get("target", "overall")))
		return
	if _avatar_host_up:
		_handle_avatar_command(cmd)
	elif _entering and cmd.get("type", "") == "exit_world":
		_entering = false
	elif _explorer != null:
		_handle_explorer_command(cmd)


func _handle_avatar_command(cmd: Dictionary) -> void:
	match cmd.get("type", ""):
		"backpack_open":
			_backpack.async_open(_rect(cmd))
		"backpack_place":
			_backpack.place(_rect(cmd))
		"backpack_close":
			_backpack.close()
		"equip":
			_backpack.async_equip(str(cmd.get("urn", "")))
		"unequip":
			_backpack.async_unequip(str(cmd.get("urn", "")))
		"set_body_shape":
			_backpack.async_set_body_shape(str(cmd.get("urn", "")))
		"set_color":
			_backpack.async_set_color(str(cmd.get("target", "")), str(cmd.get("color", "")))
		"play_emote":
			_backpack.play_emote(str(cmd.get("urn", "")))
		"set_emote_slot":
			_backpack.set_emote_slot(int(cmd.get("slot", -1)), str(cmd.get("urn", "")))
		"randomize_avatar":
			_backpack.async_randomize()
		"set_name":
			_backpack.async_set_profile_name(str(cmd.get("name", "")))
		"enter_world":
			if _exiting:
				_pending_enter = cmd
				return
			_async_enter_world(cmd)


func _async_enter_world(cmd: Dictionary) -> void:
	_backpack.close()
	_avatar_host_up = false
	_in_world = true
	_entering = true
	_explorer = null
	var parcel := Vector2i(int(cmd.get("x", 0)), int(cmd.get("y", 0)))
	var started := await RnWorld.async_enter(
		parcel, str(cmd.get("realm", "")), func() -> bool: return not _entering
	)
	if started:
		_entering = false
	else:
		on_avatar_host_ready()


func _handle_explorer_command(cmd: Dictionary) -> void:
	match cmd.get("type", ""):
		"teleport":
			_explorer.teleport_to(Vector2i(int(cmd.get("x", 0)), int(cmd.get("y", 0))))
		"send_chat":
			_explorer._on_panel_chat_submit_message(str(cmd.get("text", "")))
		"emote":
			_player().avatar.async_play_emote(str(cmd.get("urn", "")))
		"backpack_open", "backpack_place":
			var rect := Rect2(cmd.get("x", 0), cmd.get("y", 0), cmd.get("w", 0), cmd.get("h", 0))
			if cmd["type"] == "backpack_open":
				_backpack.async_open(rect)
			else:
				_backpack.place(rect)
		"backpack_close":
			_backpack.close()
		"equip":
			_backpack.async_equip(str(cmd.get("urn", "")))
		"exit_world":
			_backpack.close()
			_hud.reset()
			_explorer = null
			_avatar_host_up = true
			_exiting = true
			RnWorld.async_exit()
		_:
			_hud.handle(_explorer, cmd)


func _rect(cmd: Dictionary) -> Rect2:
	return Rect2(cmd.get("x", 0), cmd.get("y", 0), cmd.get("w", 0), cmd.get("h", 0))


func _player() -> Player:
	return _explorer.player as Player


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


func _emit(type: String, data: Dictionary) -> void:
	last_event = {"type": type, "data": data}
	if _plugin != null:
		_plugin.call("rn_event", JSON.stringify(last_event))
