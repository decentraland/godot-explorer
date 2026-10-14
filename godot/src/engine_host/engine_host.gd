extends Node
## Executes the core's typed commands on the engine and reports what the engine observes
## (`Global.engine.core_command` in, `Global.engine.report_*` out). Active when the host drives the app.

const STATE_INTERVAL := 0.25
const HIDE_INTERVAL := 0.2
const AVATAR_CONTROLS := [
	"equip",
	"unequip",
	"set_body_shape",
	"set_color",
	"play_preview_emote",
	"set_emote_slot",
	"randomize_avatar",
	"set_name",
]

var _explorer: Explorer = null
var _state_elapsed := 0.0
var _hide_elapsed := 0.0
var _backpack := EngineBackpack.new()
var _hud := EngineHud.new()
var _avatar_host_up := false
var _exiting := false
var _entering := false
var _pending_enter: Dictionary = {}
var _preview_url := ""


func _ready() -> void:
	if not Global.engine.is_host_driven():
		set_process(false)
		return
	Global.engine.core_command.connect(_on_core_command)
	_hud.connect_signals()
	add_child(_backpack)
	_async_first_launch_benchmark()


## The hardware benchmark that picks the first graphics profile, once the engine is up.
func _async_first_launch_benchmark() -> void:
	await get_tree().process_frame
	await get_tree().process_frame
	if Global.should_run_first_launch_benchmark():
		Global.run_first_launch_benchmark()


## Called by the avatar-only host scene once the identity and profile exist.
func on_avatar_host_ready() -> void:
	_avatar_host_up = true
	_exiting = false
	if not _pending_enter.is_empty():
		var pending := _pending_enter
		_pending_enter = {}
		_async_enter_world(pending)


func _process(dt: float) -> void:
	if _avatar_host_up:
		return
	if _explorer == null:
		_explorer = Global.get_explorer()
		if _explorer == null:
			return
		Global.engine.report_loading_finished(
			Global.realm.get_realm_string(),
			_explorer.parcel_position.x,
			_explorer.parcel_position.y
		)
	_hide_elapsed += dt
	if _hide_elapsed >= HIDE_INTERVAL:
		_hide_elapsed = 0.0
		EngineUiFilter.hide_non_native(_explorer.ui_root)
	_state_elapsed += dt
	if _state_elapsed >= STATE_INTERVAL:
		_state_elapsed = 0.0
		_report_hud()


func _on_core_command(kind: String, args: Dictionary) -> void:
	match kind:
		"enter_world":
			_enter_world(args)
		"exit_world":
			_exit_world()
		"show_avatar_preview":
			var rect := Rect2(args.x, args.y, args.w, args.h)
			if _backpack.is_shown():
				_backpack.place(rect)
			else:
				_backpack.async_open(rect)
		"hide_avatar_preview":
			_backpack.close()
		"set_preview_url":
			_preview_url = args.url
			if _explorer != null:
				Global.scene_fetcher.set_preview_url(_preview_url)
		"focus_camera":
			_backpack.focus_camera(args.focus)
		_:
			if kind in AVATAR_CONTROLS:
				_run_avatar_control(kind, args)
			elif _explorer != null:
				_run_world_control(kind, args)


func _run_avatar_control(kind: String, args: Dictionary) -> void:
	match kind:
		"equip":
			_backpack.async_equip(args.urn)
		"unequip":
			_backpack.async_unequip(args.urn)
		"set_body_shape":
			_backpack.async_set_body_shape(args.urn)
		"set_color":
			_backpack.async_set_color(args.target, args.color)
		"play_preview_emote":
			_backpack.play_emote(args.urn)
		"set_emote_slot":
			_backpack.set_emote_slot(args.slot, args.urn)
		"randomize_avatar":
			_backpack.async_randomize()
		"set_name":
			_backpack.async_set_profile_name(args.name)


func _run_world_control(kind: String, args: Dictionary) -> void:
	match kind:
		"teleport":
			Global.engine.teleport(Vector2i(args.x, args.y))
		"send_chat":
			_explorer._on_panel_chat_submit_message(args.text)
		"emote":
			Global.engine.play_local_emote(args.urn)
		_:
			_hud.handle(_explorer, kind, args)


func _enter_world(args: Dictionary) -> void:
	if _explorer != null:
		# The core replaced the destination while a world is up: leave it, enter on the host's return.
		_pending_enter = args
		_exit_world()
		return
	if not _avatar_host_up:
		return
	if _exiting:
		_pending_enter = args
		return
	_async_enter_world(args)


func _async_enter_world(args: Dictionary) -> void:
	_backpack.close()
	_avatar_host_up = false
	_entering = true
	_explorer = null
	var parcel := Vector2i(args.x, args.y)
	if not _preview_url.is_empty():
		Global.deep_link_obj.preview = _preview_url
	var started := await EngineWorld.async_enter(
		parcel, args.realm, func() -> bool: return not _entering
	)
	if started:
		_entering = false
	else:
		on_avatar_host_ready()


func _exit_world() -> void:
	if _entering:
		_entering = false
	elif _explorer != null:
		_backpack.close()
		_hud.reset()
		_explorer = null
		_avatar_host_up = true
		_exiting = true
		EngineWorld.async_exit()


func _report_hud() -> void:
	var pos := (_explorer.player as Player).global_position
	Global.engine.report_hud(
		Global.realm.get_realm_string(),
		_explorer.parcel_position.x,
		_explorer.parcel_position.y,
		Engine.get_frames_per_second(),
		pos.x,
		pos.y,
		pos.z
	)
