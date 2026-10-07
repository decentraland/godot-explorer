class_name ControlsFtueCoordinator
extends Node

## Decides when to show the controls overlay (issue #3014) and which controls it labels.
##
## Checked once per scene entry, when the loading screen is gone. Only controls that are on
## screen are labelled, and each control is labelled once per install: the overlay comes back on
## a later entry only if a control it has not labelled yet is visible, and then labels just that.
##
## The labelled set is device-local (config_data.gd) and written only when the player closes the
## overlay, so an app kill or a teleport while it is up shows the same labels again.

enum Decision { SHOW, DEFER, NEVER }

# Flag names exactly as served by the mobile-bff payload. An absent flag keeps its default.
const FLAG_ENABLED := "ftue-overlay"
const FLAG_LOCK_SECONDS := "ftue-overlay-lock-seconds"
const FLAG_TAP_THROUGH := "ftue-overlay-tap-through"
const FLAG_EXISTING_PLAYERS := "ftue-overlay-existing-players"

const DEFAULT_LOCK_SECONDS := 3.0
const SCREEN_NAME := "CONTROLS_FTUE"
const DEEPLINK_PARAM := "controls-ftue"
const OVERLAY_SCENE := "res://src/ui/components/organisms/controls_ftue_overlay/controls_ftue_overlay.tscn"

var _overlay: ControlsFtueOverlay = null
var _overlay_elements: Array[String] = []
# Bumped on every load start/finish so a stale _async_try_show bails after its await.
var _generation := 0
var _shown_at_msec := 0
# Evaluations still deciding whether to show; see async_wait_until_clear().
var _evaluating := 0
# The install had already been played before this launch.
var _existing_player := false
# QA: `?controls-ftue=show` labels every visible control on the next entry, even those already
# labelled. Session-only, never persisted.
var _forced := false


func _ready() -> void:
	_existing_player = Global.get_config().first_move_in_world_sent
	var deep_link := Global.deep_link_obj
	if deep_link != null and not Global.is_production():
		_forced = String(deep_link.params.get(DEEPLINK_PARAM, "")) == "show"
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)


## `state` keys: test_mode, tutorial_busy, forced, enabled, existing_player,
## show_existing_players, scene_loaded, modal_open, hud_usable, new_elements (count of visible
## controls not labelled yet).
static func decide(state: Dictionary) -> Decision:
	# The guided tutorial (#2767) teaches the same controls; the two never share a scene entry.
	if state.test_mode or state.tutorial_busy:
		return Decision.DEFER
	if not state.forced:
		if not state.enabled:
			return Decision.DEFER
		if state.existing_player and not state.show_existing_players:
			return Decision.NEVER
	if not state.scene_loaded or state.modal_open or not state.hud_usable:
		return Decision.DEFER
	if state.new_elements == 0:
		return Decision.DEFER
	return Decision.SHOW


## The visible controls still to label: everything visible when forced, otherwise what is not
## in `already_shown`.
static func pending_elements(
	visible: Array[String], already_shown: PackedStringArray, forced: bool
) -> Array[String]:
	if forced:
		return visible
	return visible.filter(func(element: String) -> bool: return not already_shown.has(element))


func _on_loading_started() -> void:
	_cancel()


# Drops a pending or visible overlay without recording its labels.
func _cancel() -> void:
	_generation += 1
	if is_instance_valid(_overlay):
		_overlay.queue_free()
	_overlay = null


func _on_loading_finished() -> void:
	_async_evaluate()


func _async_evaluate() -> void:
	_evaluating += 1
	await _async_try_show()
	_evaluating -= 1


## Returns once the overlay is neither pending nor on screen, so modals and HUD messages that
## would cover it or land under its labels can wait for it. Does not suspend when already clear.
##
## `urgent` is for what cannot wait behind it (scene crash, ban, connection lost): the overlay
## is dropped instead, unsaved, so it comes back on a later scene entry.
func async_wait_until_clear(urgent := false) -> void:
	if urgent:
		_cancel()
	while _evaluating > 0 or is_instance_valid(_overlay):
		await get_tree().process_frame


func _async_try_show() -> void:
	_generation += 1
	var generation := _generation
	var explorer := Global.get_explorer()
	if explorer == null:
		return
	# loading_finished fires when the loading screen starts fading; wait until it is gone.
	# Polled, not awaited on its signal: a freed explorer would leave this pending forever.
	while is_instance_valid(explorer) and explorer.loading_ui.visible:
		await get_tree().process_frame
		if generation != _generation:
			return
	if not is_instance_valid(explorer):
		return

	var config: ConfigData = Global.get_config()
	var flags: FeatureFlags = Global.feature_flags
	var elements := pending_elements(
		_visible_elements(explorer), config.controls_ftue_shown, _forced
	)
	var decision := decide(
		{
			"test_mode": is_automated_run() or Global.is_xr(),
			"tutorial_busy": Global.ftue_tutorial_coordinator.owns_scene_entry(),
			"forced": _forced,
			"enabled": flags.is_enabled(FLAG_ENABLED, true),
			"existing_player": _existing_player,
			"show_existing_players": flags.is_enabled(FLAG_EXISTING_PLAYERS, true),
			"scene_loaded": Global.scene_runner.get_current_parcel_scene_id() >= 0,
			"modal_open": Global.modal_manager.is_any_modal_open(),
			"hud_usable": _is_hud_usable(explorer),
			"new_elements": elements.size(),
		}
	)
	if decision == Decision.NEVER:
		_mark_shown(ControlsFtueOverlay.ALL_ELEMENTS)
	if decision != Decision.SHOW:
		return

	_overlay = load(OVERLAY_SCENE).instantiate()
	_overlay_elements = elements
	_overlay.setup(
		_menu_button(explorer),
		_chat_button(explorer),
		explorer.virtual_joystick,
		_emotes_button(explorer),
		elements,
		flags.get_number(FLAG_LOCK_SECONDS, DEFAULT_LOCK_SECONDS),
		flags.is_enabled(FLAG_TAP_THROUGH, false)
	)
	_overlay.dismissed.connect(_on_overlay_dismissed)
	# A finger held through the loading screen would otherwise keep the avatar walking.
	explorer.virtual_joystick.cancel()
	explorer.ui_root.add_child(_overlay)
	_shown_at_msec = Time.get_ticks_msec()
	if Global.metrics != null:
		Global.metrics.track_screen_viewed(SCREEN_NAME, JSON.stringify({"elements": elements}))


func _on_overlay_dismissed() -> void:
	_overlay = null
	_forced = false
	_mark_shown(_overlay_elements)
	if Global.metrics != null:
		var seconds_shown := snappedf((Time.get_ticks_msec() - _shown_at_msec) / 1000.0, 0.1)
		Global.metrics.track_click_button(
			"dismiss",
			SCREEN_NAME,
			JSON.stringify({"seconds_shown": seconds_shown, "elements": _overlay_elements})
		)


func _mark_shown(elements: Array[String]) -> void:
	var config: ConfigData = Global.get_config()
	for element in elements:
		if not config.controls_ftue_shown.has(element):
			config.controls_ftue_shown.append(element)
	config.save_to_settings_file()


# Scene tests, client tests, renderers and benchmarks start from a fresh config and must
# never have their input or their screenshots covered.
static func is_automated_run() -> bool:
	var cli := Global.cli
	return (
		Global.testing_scene_mode
		or cli.scene_renderer_mode
		or cli.client_test_mode
		or cli.dcl_benchmark
		or cli.measure_perf
		or Global.is_gp_benchmark()
	)


# With a menu open, in portrait or under a cinematic camera the in-world HUD is not what the
# player is looking at, whatever its nodes report.
func _is_hud_usable(explorer: Explorer) -> bool:
	return not (
		Global.is_orientation_portrait()
		or explorer.control_menu.visible
		or Global.current_camera_mode == Global.CameraMode.CINEMATIC
	)


func _visible_elements(explorer: Explorer) -> Array[String]:
	var joystick := explorer.virtual_joystick
	var on_screen := {
		ControlsFtueOverlay.MENU: _menu_button(explorer).is_visible_in_tree(),
		ControlsFtueOverlay.CHAT: _chat_button(explorer).is_visible_in_tree(),
		# A scene can hide the joystick's graphic (PBTouchScreenControls) without hiding the node.
		ControlsFtueOverlay.MOVEMENT:
		(
			joystick.is_visible_in_tree()
			and joystick.modulate.a > 0.0
			and not Global.touch_controls_hide_joystick
		),
		ControlsFtueOverlay.EMOTES: _emotes_button(explorer).is_visible_in_tree(),
	}
	var elements: Array[String] = []
	for element in ControlsFtueOverlay.ALL_ELEMENTS:
		if on_screen[element]:
			elements.append(element)
	return elements


func _menu_button(explorer: Explorer) -> Control:
	return explorer.navbar.get_node("%Button")


func _chat_button(explorer: Explorer) -> Control:
	return explorer.chat_panel.get_node("%Chatbar").get_node("%Button_Chat")


func _emotes_button(explorer: Explorer) -> Control:
	return explorer.emote_wheel.get_node("Button_Emotes")
