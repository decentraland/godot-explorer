class_name FtueTutorialCoordinator
extends Node

## Offers, runs and ends the guided tutorial (issue #2767).
##
## Checked once per scene entry, when the loading screen is gone: on a tutorial scene (Genesis
## Plaza by default) a player who was never offered it gets the welcome card. START runs the
## seven steps; SKIP, there or at any step, ends it. Settings > Gameplay can replay it anywhere.
##
## "Offered" and "completed" are device-local (config_data.gd).

# Flag names exactly as served by the mobile-bff payload. An absent flag keeps its default.
const FLAG_ENABLED := "ftue-tutorial"
const FLAG_SCENES := "ftue-tutorial-scenes"
const FLAG_MOVE_METERS := "ftue-tutorial-move-meters"
const FLAG_CAMERA_DEGREES := "ftue-tutorial-camera-degrees"
const FLAG_SOCIAL_SECONDS := "ftue-tutorial-social-seconds"
const FLAG_REWARD := "ftue-tutorial-reward"

# Base parcels of the Genesis City scenes that run the tutorial, as "x,y;x,y". Genesis Plaza.
const DEFAULT_SCENES := "-3,-2"
const DEFAULT_MOVE_METERS := 1.0
const DEFAULT_CAMERA_DEGREES := 30.0
const DEFAULT_SOCIAL_SECONDS := 3.0
const DEEPLINK_PARAM := "ftue-tutorial"
const SCREEN_WELCOME := "FTUE_TUTORIAL_WELCOME"
const SCREEN_STEP := "FTUE_TUTORIAL_STEP"
const SCREEN_COMPLETE := "FTUE_TUTORIAL_COMPLETE"
const SCREEN_REWARD := "FTUE_TUTORIAL_REWARD"
# Entry of RewardCampaigns.CAMPAIGNS the tutorial grants. Until it exists the reward is skipped.
const REWARD_CAMPAIGN := "FtueTutorial"
# QA: the campaign `?ftue-tutorial=reward` previews the reward card with.
const REWARD_PREVIEW_CAMPAIGN := "MobilePet"
const OVERLAY_SCENE := "res://src/ui/components/organisms/ftue_tutorial/ftue_tutorial_overlay.tscn"
const WELCOME_SCENE := "res://src/ui/components/organisms/ftue_tutorial/ftue_welcome_modal.tscn"
# i18n-keys: FTUE_TUTORIAL_REWARD_BODY
const REWARD_SCENE := "res://src/ui/components/organisms/ftue_tutorial/ftue_reward_modal.tscn"

var _overlay: FtueTutorialOverlay = null
var _runner: FtueTutorialRunner = null
var _welcome: FtueWelcomeModal = null
var _reward_layer: CanvasLayer = null
var _is_replay := false
# Bumped on every load so a stale _async_on_scene_entered bails after its await.
var _generation := 0
# QA: `?ftue-tutorial=start` offers the tutorial on the next scene entry, wherever that is and
# even if it was offered before. `?ftue-tutorial=reward` shows the reward card instead.
# Session-only, never persisted.
var _forced := false
var _forced_reward := false


func _ready() -> void:
	var deep_link := Global.deep_link_obj
	if deep_link != null and not Global.is_production():
		var value := String(deep_link.params.get(DEEPLINK_PARAM, ""))
		_forced = value == "start"
		_forced_reward = value == "reward"
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)


## `state` keys: test_mode, forced, enabled, offered, tutorial_scene, scene_loaded, modal_open,
## hud_ready.
static func should_offer(state: Dictionary) -> bool:
	if state.test_mode or not state.scene_loaded or state.modal_open or not state.hud_ready:
		return false
	if state.forced:
		return true
	return state.enabled and state.tutorial_scene and not state.offered


## Parses the scene list flag, "x,y;x,y", skipping anything malformed.
static func parse_scenes(text: String) -> Array[Vector2i]:
	var scenes: Array[Vector2i] = []
	for pair in text.split(";", false):
		var coords := pair.strip_edges().split(",")
		if coords.size() == 2 and coords[0].is_valid_int() and coords[1].is_valid_int():
			scenes.append(Vector2i(coords[0].to_int(), coords[1].to_int()))
	return scenes


func is_running() -> bool:
	return is_instance_valid(_overlay)


## True when this scene entry belongs to the tutorial: it is on screen, about to be offered,
## or this is one of its scenes. The controls overlay (#3014) never shows then.
func owns_scene_entry() -> bool:
	if is_running() or is_instance_valid(_welcome) or _forced or _forced_reward:
		return true
	return Global.feature_flags.is_enabled(FLAG_ENABLED, false) and _is_tutorial_scene()


## Settings > Gameplay > Replay: step 1 again, no welcome card, once the menu has closed.
func start_replay() -> void:
	_async_start_replay()


func _on_loading_started() -> void:
	_generation += 1
	_close_welcome()


func _on_loading_finished() -> void:
	_async_on_scene_entered()


func _async_on_scene_entered() -> void:
	_generation += 1
	var generation := _generation
	var explorer := Global.get_explorer()
	# Polled, not awaited on its signal: a freed explorer would leave this pending forever.
	while is_instance_valid(explorer) and explorer.loading_ui.visible:
		await get_tree().process_frame
		if generation != _generation:
			return
	if not is_instance_valid(explorer) or is_running():
		return
	if _forced_reward:
		_forced_reward = false
		_async_show_reward(RewardCampaigns.CAMPAIGNS[REWARD_PREVIEW_CAMPAIGN])
		return

	var offer := should_offer(
		{
			"test_mode": ControlsFtueCoordinator.is_automated_run() or Global.is_xr(),
			"forced": _forced,
			"enabled": Global.feature_flags.is_enabled(FLAG_ENABLED, false),
			"offered": Global.get_config().ftue_tutorial_offered,
			"tutorial_scene": _is_tutorial_scene(),
			"scene_loaded": Global.scene_runner.get_current_parcel_scene_id() >= 0,
			"modal_open": Global.modal_manager.is_any_modal_open(),
			"hud_ready": _is_hud_ready(explorer),
		}
	)
	if not offer:
		return
	_forced = false
	_welcome = load(WELCOME_SCENE).instantiate()
	_welcome.start_pressed.connect(_on_welcome_answered.bind(true))
	_welcome.skip_pressed.connect(_on_welcome_answered.bind(false))
	explorer.ui_root.add_child(_welcome)
	_track_screen(SCREEN_WELCOME, {})


func _on_welcome_answered(start: bool) -> void:
	_close_welcome()
	# The pressed button took keyboard focus with it; movement is gated on the explorer having it.
	Global.explorer_grab_focus()
	var config: ConfigData = Global.get_config()
	config.ftue_tutorial_offered = true
	config.save_to_settings_file()
	_track_click("start" if start else "skip", SCREEN_WELCOME, {"step": 0})
	var explorer := Global.get_explorer()
	if start and is_instance_valid(explorer):
		_start(explorer, false)


func _close_welcome() -> void:
	if is_instance_valid(_welcome):
		_welcome.queue_free()
	_welcome = null


func _async_start_replay() -> void:
	var explorer := Global.get_explorer()
	while is_instance_valid(explorer) and explorer.control_menu.visible:
		await get_tree().process_frame
	if is_instance_valid(explorer) and not is_running():
		_start(explorer, true)


func _start(explorer: Explorer, is_replay: bool) -> void:
	var flags: FeatureFlags = Global.feature_flags
	_is_replay = is_replay
	_overlay = load(OVERLAY_SCENE).instantiate()
	explorer.ui_root.add_child(_overlay)
	_runner = FtueTutorialRunner.new()
	_overlay.add_child(_runner)
	_runner.step_entered.connect(_on_step_entered)
	_runner.completed.connect(_on_completed)
	_runner.skipped.connect(_on_skipped)
	_runner.start(
		explorer,
		_overlay,
		flags.get_number(FLAG_MOVE_METERS, DEFAULT_MOVE_METERS),
		flags.get_number(FLAG_CAMERA_DEGREES, DEFAULT_CAMERA_DEGREES),
		flags.get_number(FLAG_SOCIAL_SECONDS, DEFAULT_SOCIAL_SECONDS)
	)


func _on_step_entered(step: FtueTutorialRunner.Step) -> void:
	_track_screen(SCREEN_STEP, _step_properties(step))


func _on_completed() -> void:
	_end()
	_track_screen(SCREEN_COMPLETE, {"is_replay": _is_replay})
	var config: ConfigData = Global.get_config()
	config.ftue_tutorial_completed = true
	# The tutorial covered every control the overlay (#3014) labels.
	config.controls_ftue_shown = PackedStringArray(ControlsFtueOverlay.ALL_ELEMENTS)
	config.save_to_settings_file()
	var campaign: Dictionary = RewardCampaigns.CAMPAIGNS.get(REWARD_CAMPAIGN, {})
	if (
		not _is_replay
		and not campaign.is_empty()
		and Global.feature_flags.is_enabled(FLAG_REWARD, false)
	):
		_async_show_reward(campaign)


func _on_skipped(step: FtueTutorialRunner.Step) -> void:
	_end()
	_track_click("skip", SCREEN_STEP, _step_properties(step))


# On its own CanvasLayer above the HUD, like the modals ModalManager shows.
func _async_show_reward(campaign: Dictionary) -> void:
	_close_reward()
	_reward_layer = CanvasLayer.new()
	_reward_layer.layer = 100
	get_tree().root.add_child(_reward_layer)
	var modal: RewardModal = load(REWARD_SCENE).instantiate()
	_reward_layer.add_child(modal)
	modal.dismissed.connect(_close_reward)
	modal.button_claim.pressed.connect(_track_click.bind("claim", SCREEN_REWARD, {}))
	await modal.async_setup(campaign)
	_track_screen(SCREEN_REWARD, {})


func _close_reward() -> void:
	if is_instance_valid(_reward_layer):
		_reward_layer.queue_free()
	_reward_layer = null


func _end() -> void:
	if is_instance_valid(_overlay):
		_overlay.queue_free()
	_overlay = null
	_runner = null


# The backpack is one step for the player, two for the runner.
func _step_properties(step: FtueTutorialRunner.Step) -> Dictionary:
	return {
		"step": mini(step, FtueTutorialRunner.Step.BACKPACK) + 1,
		"step_name": FtueTutorialRunner.STEP_NAMES[step],
		"is_replay": _is_replay,
	}


func _is_tutorial_scene() -> bool:
	var scene_id: int = Global.scene_runner.get_current_parcel_scene_id()
	# A world can declare any base parcel, so the parcel alone does not identify a scene.
	if scene_id < 0 or not Realm.is_genesis_city(Global.realm.realm_url):
		return false
	var scenes := parse_scenes(Global.feature_flags.get_text(FLAG_SCENES, DEFAULT_SCENES))
	return scenes.has(Global.scene_runner.get_scene_base_parcel(scene_id))


# Step 1 needs the joystick and step 4 onwards the whole HUD, in landscape.
func _is_hud_ready(explorer: Explorer) -> bool:
	if Global.is_orientation_portrait() or explorer.control_menu.visible:
		return false
	if Global.current_camera_mode == Global.CameraMode.CINEMATIC:
		return false
	return (
		explorer.hud_content.is_visible_in_tree()
		and explorer.virtual_joystick.is_visible_in_tree()
		and not Global.touch_controls_hide_joystick
	)


func _track_screen(screen: String, properties: Dictionary) -> void:
	if Global.metrics != null:
		Global.metrics.track_screen_viewed(
			screen, "" if properties.is_empty() else JSON.stringify(properties)
		)


func _track_click(button: String, screen: String, properties: Dictionary) -> void:
	if Global.metrics != null:
		Global.metrics.track_click_button(button, screen, JSON.stringify(properties))
