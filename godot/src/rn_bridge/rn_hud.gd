class_name RnHud
extends RefCounted
## HUD commands from RN (insets, chat, hide UI) and the events that report native HUD state.

const CHAT_CACHE_MAX := 200

var _emit: Callable
var _hidden := false
var _chat_open := false
var _unread := 0
var _profiles: Dictionary = {}


func _init(emit: Callable) -> void:
	_emit = emit


func connect_signals() -> void:
	Global.on_chat_message.connect(_async_on_chat_message)
	Global.open_backpack.connect(
		func(emotes: bool) -> void: _emit.call("open_backpack", {"emotes": emotes})
	)
	if Global.scene_runner != null:
		Global.scene_runner.loading_progress.connect(_on_loading_progress)


## Returns true when the command belonged to the HUD.
func handle(explorer: Explorer, cmd: Dictionary) -> bool:
	match cmd.get("type", ""):
		"set_hud_insets":
			_set_insets(explorer, cmd)
		"set_chat_width":
			Global.chat_notifications_width = int(_to_viewport_units(float(cmd.get("width", 0))))
			explorer._push_scene_interactable_area()
		"chat_open":
			_set_chat_open(explorer, bool(cmd.get("open", false)))
		"hide_ui":
			_hidden = bool(cmd.get("hidden", false))
			explorer._apply_mobile_controls_hide_ui(_hidden)
			_emit_hud_state()
		_:
			return false
	return true


func reset() -> void:
	_hidden = false
	_chat_open = false
	_unread = 0


# RN reports device pixels; Godot lays out in its scaled viewport units.
func _set_insets(explorer: Explorer, cmd: Dictionary) -> void:
	for side: String in ["top", "right", "bottom", "left"]:
		explorer.safe_area_hud.add_theme_constant_override(
			"margin_" + side, int(_to_viewport_units(float(cmd.get(side, 0))))
		)
	explorer._push_scene_interactable_area()


func _set_chat_open(explorer: Explorer, open: bool) -> void:
	if open == _chat_open:
		return
	_chat_open = open
	if open:
		explorer._on_chat_focus_entered()
		_unread = 0
		_emit.call("unread_chat", {"count": 0})
	else:
		explorer._on_chat_focus_exited()
	_emit_hud_state()


func _emit_hud_state() -> void:
	_emit.call("hud_state", {"hidden": _hidden, "chat_open": _chat_open})


func _to_viewport_units(pixels: float) -> float:
	var window_width := DisplayServer.window_get_size().x
	if window_width <= 0:
		return pixels
	return pixels * Global.get_viewport().get_visible_rect().size.x / window_width


func _on_loading_progress(percent: float, loaded: int, total: int) -> void:
	_emit.call(
		"loading_progress", {"loaded": loaded, "total": total, "mbps": 0.0, "percent": percent}
	)


func _async_on_chat_message(address: String, message: String, timestamp: float) -> void:
	var data := {"address": address, "message": message, "timestamp": timestamp}
	if address.begins_with("0x"):
		data["profile"] = await _async_profile(address)
	if not _chat_open:
		_unread += 1
		_emit.call("unread_chat", {"count": _unread})
	_emit.call("chat", data)


func _async_profile(address: String) -> Dictionary:
	if not _profiles.has(address):
		var entry := await RnProfileLookup.async_friend_entry(address)
		if _profiles.size() >= CHAT_CACHE_MAX:
			_profiles.clear()
		_profiles[address] = {
			"name": entry["name"],
			"has_claimed_name": entry["has_claimed_name"],
			"profile_picture_url": entry["profile_picture_url"],
		}
	return _profiles[address]
