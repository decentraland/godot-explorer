class_name EngineHud
extends RefCounted
## Executes the shell's HUD controls (insets, chat, hide UI) and reports the native HUD state.

const CHAT_CACHE_MAX := 200

var _hidden := false
var _chat_open := false
var _unread := 0
var _profiles: Dictionary = {}


func connect_signals() -> void:
	Global.on_chat_message.connect(_async_on_chat_message)
	Global.open_backpack.connect(Global.engine.report_open_backpack)
	if Global.scene_runner != null:
		Global.scene_runner.loading_progress.connect(_on_loading_progress)


## Returns true when the control belonged to the HUD.
func handle(explorer: Explorer, kind: String, args: Dictionary) -> bool:
	match kind:
		"set_hud_insets":
			Global.engine.set_hud_insets(args.top, args.right, args.bottom, args.left)
		"set_chat_width":
			Global.chat_notifications_width = int(_to_viewport_units(args.width))
			explorer._push_scene_interactable_area()
		"chat_open":
			_set_chat_open(explorer, args.open)
		"hide_ui":
			_hidden = args.hidden
			explorer._apply_mobile_controls_hide_ui(_hidden)
			Global.engine.report_ui_state(_hidden, _chat_open)
		_:
			return false
	return true


func reset() -> void:
	_hidden = false
	_chat_open = false
	_unread = 0


func _set_chat_open(explorer: Explorer, open: bool) -> void:
	if open == _chat_open:
		return
	_chat_open = open
	if open:
		explorer._on_chat_focus_entered()
		_unread = 0
		Global.engine.report_unread_chat(0)
	else:
		explorer._on_chat_focus_exited()
	Global.engine.report_ui_state(_hidden, _chat_open)


func _to_viewport_units(pixels: float) -> float:
	var window_width := DisplayServer.window_get_size().x
	if window_width <= 0:
		return pixels
	return pixels * Global.get_viewport().get_visible_rect().size.x / window_width


func _on_loading_progress(percent: float, loaded: int, total: int) -> void:
	Global.engine.report_loading_detail(loaded, total, 0.0, percent)


func _async_on_chat_message(address: String, message: String, timestamp: float) -> void:
	var profile := {}
	if address.begins_with("0x"):
		profile = await _async_profile(address)
	if not _chat_open:
		_unread += 1
		Global.engine.report_unread_chat(_unread)
	Global.engine.report_chat(
		address,
		message,
		timestamp,
		not profile.is_empty(),
		profile.get("name", ""),
		profile.get("has_claimed_name", false),
		profile.get("profile_picture_url", "")
	)


func _async_profile(address: String) -> Dictionary:
	if not _profiles.has(address):
		var fetched: Variant = await PromiseUtils.async_awaiter(
			Global.content_provider.fetch_profile(address)
		)
		var profile: Variant = null if fetched is PromiseError else fetched
		if _profiles.size() >= CHAT_CACHE_MAX:
			_profiles.clear()
		_profiles[address] = {
			"name": profile.get_name() if profile != null else address,
			"has_claimed_name": profile.has_claimed_name() if profile != null else false,
			"profile_picture_url":
			profile.get_avatar().get_snapshots_face_url() if profile != null else "",
		}
	return _profiles[address]
