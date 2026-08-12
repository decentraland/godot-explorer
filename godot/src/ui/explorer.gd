class_name Explorer
extends Node

# Friendship/connectivity subscribe retry policy: bounded exponential backoff
# 5s, 10s, 20s, 40s, 60s, 60s — caps at ~3min total before giving up.
const _SUBSCRIBE_RETRY_MAX_ATTEMPTS: int = 6
const _SUBSCRIBE_RETRY_BASE_DELAY: float = 5.0
const _SUBSCRIBE_RETRY_MAX_DELAY: float = 60.0

const _MULTIPLAYER_DEBUG_PANEL_SCENE := preload(
	"res://src/ui/components/organisms/multiplayer_debug/multiplayer_debug_panel.tscn"
)

## Dev-only translucent overlay of the scene interactable area, off by default. The
## production guard in _update_interactable_area_debug always wins; this only toggles it
## within dev builds, flipped at runtime from Settings > Dev Tools.
var show_interactable_area: bool = false

var is_genesis_city: bool
var player: Node3D = null
var scene_title: String
var parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0
var dirty_save_position: bool = false

var multiplayer_debug_panel = null
var disable_move_to = false

var virtual_joystick_orig_position: Vector2i

var _chat_commands := ChatCommands.new(self)
var _first_time_refresh_warning = true

var _last_parcel_position: Vector2i = Vector2i.MAX
var _avatar_under_crosshair: Avatar = null
var _last_outlined_avatar: Avatar = null
var _last_outlined_entity: Node3D = null
var _is_loading: bool = true  # Start as loading
var _ban_check_generation: int = 0
var _pending_notification_toast: Dictionary = {}  # Store notification waiting to be shown
var _subscription_reconnecting: bool = false  # Debounce for subscription_dropped
var _resubscribe_timer: Timer = null
## True between social-service init and player logout. Gates retry loops so they
## exit cleanly when the session ends mid-await instead of re-subscribing after sign-out.
var _session_active: bool = false

## Children of %UI hidden while "hide explorer UI" is on; restored when toggled off.
var _ui_children_hidden_for_hud_mode: Array[CanvasItem] = []

## Session-only: minimized main HUD (settings toggle); reset on each loading_started / new explorer run.
var _session_hide_main_hud: bool = false
## Session-only sub-options for hide UI.
var _session_hide_view_profile: bool = true
var _session_hide_world_interactions: bool = true
var _session_hide_player_names: bool = true
var _session_hide_scene_ui: bool = true
var _mobile_controls_hidden_for_hide_ui: bool = false
# Applies scene-driven (PBTouchScreenControls) joystick/crosshair hiding; see the class doc.
var _sdk_touch_controls: SdkTouchControlsApplier = null

## True when the debug panel was enabled from settings toggle.
var _debug_panel_from_settings: bool = false

@onready var ui_root: Control = %UI
@onready var ui_safe_area: Control = %SceneUIContainer

@onready var warning_messages = %WarningMessages
@onready var label_crosshair = %Label_Crosshair
@onready var control_pointer_tooltip = %Control_PointerTooltip

@onready var chat_panel = %ChatPanel
#@onready var url_popup = %UrlPopup
#@onready var jump_in_popup = %JumpInPopup

@onready var notifications_panel: PanelContainer = %NotificationsPanel
@onready var friends_panel: PanelContainer = %FriendsPanel
@onready var settings_panel: Control = %SettingsPanel
@onready var label_version = %Label_Version
@onready var label_fps = %Label_FPS
@onready var control_menu = %Control_Menu
@onready var mobile_ui = %MobileUI
@onready var mobile_camera_input: Control = %MobileCameraInput
@onready var safe_area_controls: MarginContainer = %SafeAreaControls
@onready var safe_area_hud: MarginContainer = %SafeAreaHud
@onready var hud_content: Control = %InteractableHUD
## Preview-mode HUD toolbar (console/scene-stats/reload). Static hidden InteractableHUD child
## so its console keeps capturing logs; shown only while _preview_panel_active().
@onready var preview_hud_panel = %PreviewHudPanel
@onready var virtual_joystick: Control = %VirtualJoystick_Left
@onready var profile_container: Control = %ProfileContainer

@onready var loading_ui = %Loading

@onready var emote_wheel = %EmoteWheel

@onready var world: Node3D = %world

@onready var timer_broadcast_position: Timer = %Timer_BroadcastPosition

@onready var navbar: Control = %Navbar
@onready var joypad: Control = %Joypad
@onready var button_show_ui: Button = %Button_ShowUI
@onready var hud_dismiss_catcher: Control = %HudDismissCatcher
@onready var interactable_area_debug: ColorRect = %InteractableAreaDebug
@onready var interactable_area_debug_label: Label = %InteractableAreaDebugLabel


func _process(_dt):
	if not Global.is_xr():
		_sdk_touch_controls.apply(_mobile_controls_hidden_for_hide_ui)

	parcel_position_real = Vector2(player.position.x * 0.0625, -player.position.z * 0.0625)

	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	if _last_parcel_position != parcel_position:
		Global.scene_fetcher.update_position(parcel_position, false)
		_last_parcel_position = parcel_position
		Global.get_config().last_parcel_position = parcel_position
		dirty_save_position = true
		Global.change_parcel.emit(parcel_position)
		Global.metrics.update_position("%d,%d" % [parcel_position.x, parcel_position.y])


func get_params_from_cmd():
	var realm_string = Global.cli.realm if not Global.cli.realm.is_empty() else null
	var location_vector = Global.cli.get_location_vector()
	if location_vector == Vector2i.MAX:
		location_vector = null

	# Preview deeplink takes priority - use it as the realm for hot reload development
	if not Global.deep_link_obj.preview.is_empty() and realm_string == null:
		realm_string = Global.deep_link_obj.preview

	if not Global.deep_link_obj.realm.is_empty() and realm_string == null:
		realm_string = Global.deep_link_obj.realm

	if Global.deep_link_obj.is_location_defined() and location_vector == null:
		location_vector = Global.deep_link_obj.location
		if realm_string == null:
			realm_string = DclUrls.main_realm()

	return [realm_string, location_vector]


func _ready():
	# Out of the lobby — restore the relaxed 10s flush cadence (the lobby drops it to 2s).
	Global.metrics.set_flush_interval(10.0)

	GraphicSettings.apply_full_processor_mode()

	Global.scene_runner.on_change_scene_id.connect(_on_change_scene_id)
	Global.change_parcel.connect(_on_change_parcel)

	label_version.set_text(DclGlobal.get_version_with_env())

	if DclGlobal.is_production():
		label_fps.visible = false

	Global.set_orientation_landscape()
	UiSounds.install_audio_recusirve(self)
	Global.music_player.stop()

	# Connect notification bell button
	Global.open_notifications_panel.connect(_show_notifications_panel)
	Global.open_discover.connect(_on_discover_open)
	Global.on_menu_open.connect(_on_menu_open)
	Global.on_menu_close.connect(_on_menu_close)

	# Connect friends button
	Global.open_friends_panel.connect(_show_friends_panel)

	# Connect settings panel button
	Global.open_settings_panel.connect(_show_settings_panel)

	# Connect debug panel signal from landscape settings panel
	var settings_node = settings_panel.get_node("MarginContainer/Settings")
	if settings_node:
		settings_node.request_debug_panel.connect(_on_control_menu_request_debug_panel)
		# Without this, the in-game "Scene Paused" toggle does nothing (menu.gd wires
		# request_pause_scenes for the pre-explorer path only).
		settings_node.request_pause_scenes.connect(_on_control_menu_request_pause_scenes)

	navbar.navbar_closed.connect(_close_all_panels)
	navbar.navbar_opened.connect(_open_friends_panel)
	profile_container.visibility_changed.connect(_on_profile_container_visibility_changed)

	# Connect to NotificationsManager queue signals
	NotificationsManager.notification_queued.connect(_on_notification_queued)

	# Connect to notification clicks to handle friend request notifications
	Global.notification_clicked.connect(_on_notification_clicked)

	# Connect on open backpack (from the navbar button or the emote wheel)
	Global.open_backpack.connect(_on_backpack_open)

	# Connect deep link router signals for path-based actions
	Global.deep_link_router.deep_link_jump.connect(_on_deep_link_jump)
	Global.deep_link_router.deep_link_open_event.connect(_on_deep_link_open_event)
	Global.deep_link_router.deep_link_open_place.connect(_on_deep_link_open_place)

	# Connect to loading state signals
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)

	Global.orientation_changed.connect(_on_orientation_changed)
	Global.chat_write_mode_changed.connect(_on_chat_write_mode_changed)

	# Keep the full-screen dismiss catcher in sync with what's open.
	notifications_panel.visibility_changed.connect(_refresh_hud_dismiss)
	friends_panel.visibility_changed.connect(_refresh_hud_dismiss)
	settings_panel.visibility_changed.connect(_refresh_hud_dismiss)
	chat_panel.chat.visibility_changed.connect(_refresh_hud_dismiss)

	# Chat focus (open) overlays the message view: hide the emote button and joypad
	# while focused, restore them when the chat closes.
	chat_panel.chat.on_open_chat.connect(_on_chat_focus_entered)
	chat_panel.chat.on_exit_chat.connect(_on_chat_focus_exited)

	player = load("res://src/logic/player/player.tscn").instantiate()

	player.set_name("Player")
	world.add_child(player)

	timer_broadcast_position.player_node = player
	if Global.is_xr():
		player.vr_screen.set_instantiate_scene(ui_root)

	emote_wheel.avatar_node = player.avatar

	loading_ui.enable_loading_screen(Global.get_config().last_realm_joined, "on_explorer_ready")
	var cmd_params = get_params_from_cmd()
	var cmd_realm = Global.FORCE_TEST_REALM if Global.FORCE_TEST else cmd_params[0]
	var cmd_location = cmd_params[1]
	if Global.FORCE_TEST and cmd_location == null:
		cmd_location = Global.FORCE_TEST_LOCATION

	# --spawn-avatars
	if Global.cli.spawn_avatars:
		var test_spawn_and_move_avatars = TestSpawnAndMoveAvatars.new()
		add_child(test_spawn_and_move_avatars)

	# --debug-panel flag acts like enabling from settings
	if Global.cli.debug_panel:
		_debug_panel_from_settings = true

	# Preview HUD toolbar (console/reload, plus scene-stats in preview/deep-link).
	# Replaces the chat panel while active; never created in a normal production run.
	_update_preview_hud()

	# multiplayer_debug deep link parameter auto-enables the multiplayer debug panel.
	# Checked via the comms flag too: a target-less deeplink is consumed while the
	# lobby/menu is active, and the Rust-side flag is the carrier that survives into
	# this later explorer boot. The signal covers deeplinks arriving while in-world.
	Global.deep_link_router.deep_link_received.connect(_check_multiplayer_debug_deeplink)
	_check_multiplayer_debug_deeplink()

	# Scene Inspector: the bridge is now dialed from app startup (Global._ready),
	# not here — so the channel is live from second 0, before login / world entry.
	# Scene Inspector file output: --scene-inspector-file or ?scene-inspector-file=true
	var scene_inspector_file: bool = (
		Global.deep_link_obj.scene_inspector_file or Global.cli.scene_inspector_file
	)
	if scene_inspector_file:
		Global.scene_inspector_dispatcher.set_file_logging(true)

	# Clear deep link after initial setup to prevent re-teleporting on first app resume
	Global.deep_link_router._clear_deep_link()

	virtual_joystick.mouse_filter = Control.MOUSE_FILTER_IGNORE
	virtual_joystick_orig_position = virtual_joystick.get_position()
	_sdk_touch_controls = SdkTouchControlsApplier.new(virtual_joystick, label_crosshair)

	if Global.is_xr():
		mobile_ui.hide()
		label_crosshair.hide()
	elif Global.is_mobile():
		mobile_ui.show()
		label_crosshair.show()
		reset_cursor_position()
		_update_virtual_controls_visibility()
	else:
		# Desktop is development-only in this build: show the on-screen controls so the
		# native joystick/joypad (and the TouchscreenInputControls / UiInputBinding features)
		# stay visible and debuggable. Keep the desktop crosshair/cursor behaviour as-is.
		mobile_ui.show()
		_update_virtual_controls_visibility()

	control_pointer_tooltip.hide()
	var start_parcel_position: Vector2i = Vector2i(Global.get_config().last_parcel_position)
	if cmd_location != null:
		start_parcel_position = cmd_location

	player.position = (
		16 * Vector3(start_parcel_position.x, 0.1, -start_parcel_position.y)
		+ Vector3(8.0, 0.0, -8.0)
	)
	player.look_at(16 * Vector3(start_parcel_position.x + 1, 0, -(start_parcel_position.y + 1)))

	Global.player_camera_node = player.camera
	Global.scene_runner.player_avatar_node = player.avatar
	Global.scene_runner.player_body_node = player
	Global.scene_runner.console = self._on_scene_console_message
	Global.scene_runner.pointer_tooltip_changed.connect(self._on_pointer_tooltip_changed)
	player.avatar.emote_triggered.connect(Global.scene_runner.on_primary_player_trigger_emote)
	player.avatar.emote_finished.connect(Global.scene_runner.on_primary_player_emote_finished)
	# Recreate base_ui before use: the previous instance is freed when the Explorer
	# scene is torn down (logout/change_scene_to_file), leaving a dangling reference.
	Global.scene_runner.recreate_base_ui()
	ui_safe_area.add_child(Global.scene_runner.base_ui)
	ui_safe_area.move_child(Global.scene_runner.base_ui, 0)

	ui_safe_area.resized.connect(self._push_scene_interactable_area)
	get_window().size_changed.connect(self._push_scene_interactable_area)
	_push_scene_interactable_area.call_deferred()

	Global.scene_fetcher.notify_pending_loading_scenes.connect(
		self._on_notify_pending_loading_scenes
	)

	# Add disconnect handler for reconnection logic
	var disconnect_handler = (
		load("res://src/ui/components/organisms/disconnect_handler/disconnect_handler.tscn")
		. instantiate()
	)
	add_child(disconnect_handler)

	Global.scene_fetcher.update_position(start_parcel_position, true)

	if cmd_realm != null:
		if Realm.is_dcl_ens(cmd_realm) and Global.deep_link_obj.preview.is_empty():
			Global.async_join_world(cmd_realm)
		else:
			Global.realm.async_set_realm(cmd_realm)
			if not Global.deep_link_obj.preview.is_empty():
				Global.scene_fetcher.set_preview_url(cmd_realm)
	else:
		if Global.get_config().last_realm_joined.is_empty():
			Global.realm.async_set_realm(
				"https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"
			)
		else:
			Global.realm.async_set_realm(Global.get_config().last_realm_joined)
	Global.scene_runner.process_mode = Node.PROCESS_MODE_INHERIT

	Global.player_identity.logout.connect(self._on_player_logout)
	Global.player_identity.profile_changed.connect(Global.avatars.update_primary_player_profile)
	Global.player_identity.profile_changed.connect(self._on_player_profile_changed)

	# Keep avatar nicknames in sync with the session "Hide UI" setting.
	# This is session-only (no config persistence) and must apply to existing + newly added avatars.
	if Global.avatars and Global.avatars.avatar_added:
		if not Global.avatars.avatar_added.is_connected(_on_avatar_added_apply_hide_ui):
			Global.avatars.avatar_added.connect(_on_avatar_added_apply_hide_ui)
	# Apply current state once at startup (in case something toggled early).
	_apply_hide_ui_to_avatar_nicks(_session_hide_main_hud)

	# Initialize social service for non-guest accounts
	if not Global.player_identity.is_guest:
		_async_initialize_social_service()

	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		Global.player_identity.profile_changed.emit(profile)

	Global.dcl_tokio_rpc.need_open_url.connect(self._on_need_open_url)
	Global.scene_runner.set_pause(false)

	if Global.testing_scene_mode:
		Global.player_identity.create_disposable_account()

	Global.metrics.update_identity(
		Global.player_identity.get_address_str(), Global.player_identity.is_guest
	)

	Global.open_profile_by_address.connect(_async_open_profile_by_address)
	Global.open_profile_by_avatar.connect(_async_open_profile_by_avatar)
	Global.open_own_profile.connect(_on_global_open_own_profile)

	ui_root.grab_focus.call_deferred()

	if OS.get_cmdline_args().has("--scene-renderer"):
		var scene_renderer_orchestor = (
			load("res://src/tool/scene_renderer/scene_orchestor.tscn").instantiate()
		)
		add_child(scene_renderer_orchestor)

	var dcl_global_camera_controller = (
		load("res://src/decentraland_components/dcl_global_camera_controller.tscn").instantiate()
	)
	add_child(dcl_global_camera_controller)

	button_show_ui.pressed.connect(_on_button_show_ui_pressed)
	_session_hide_main_hud = false
	set_visible_ui(true, true)


func _on_need_open_url(url: String, _description: String, _use_webkit: bool) -> void:
	if not Global.player_identity.get_address_str().is_empty():
		Global.open_url(url)


## Push the safe-area rect (in canvas/logical pixels) to the scene runner so
## scenes get correct UiCanvasInformation.interactable_area on every resize,
## including --emulate-ios / --emulate-android virtual margins.
## The HUD safe rectangle (in SceneUIContainer/canvas coords): device safe area + the
## symmetric 108/32/108/45 floor from SafeAreaHud, WITHOUT the chat column. This is the
## "screen inset" area — safe from device insets; the chat may still overlap it.
func get_safe_area() -> Rect2:
	var canvas: Vector2 = ui_safe_area.size
	var m_left: float = safe_area_hud.get_theme_constant("margin_left")
	var m_right: float = safe_area_hud.get_theme_constant("margin_right")
	var m_top: float = safe_area_hud.get_theme_constant("margin_top")
	var m_bottom: float = safe_area_hud.get_theme_constant("margin_bottom")
	return Rect2(
		m_left,
		m_top,
		maxf(canvas.x - m_left - m_right, 0.0),
		maxf(canvas.y - m_top - m_bottom, 0.0)
	)


## Where scenes may draw interactive UI without being covered by the client UI: the safe
## rectangle minus the left chat column. The joypad (bottom-right) may still overlap it.
func get_interactable_area() -> Rect2:
	var safe: Rect2 = get_safe_area()
	var chat: float = Global.chat_notifications_width
	return Rect2(
		safe.position.x + chat, safe.position.y, maxf(safe.size.x - chat, 0.0), safe.size.y
	)


func _push_scene_interactable_area() -> void:
	if not is_instance_valid(ui_safe_area) or not is_instance_valid(safe_area_hud):
		return
	var area: Rect2 = get_interactable_area()
	_update_interactable_area_debug(area)
	if is_instance_valid(Global.scene_runner) and area.size.x > 0 and area.size.y > 0:
		Global.scene_runner.set_interactable_area(Rect2i(area))
		Global.scene_runner.set_safe_area(Rect2i(get_safe_area()))


## Dev-only translucent overlay to verify the computed interactable area on device.
## Never shows in production; within dev, gated by `show_interactable_area`.
func _update_interactable_area_debug(area: Rect2) -> void:
	# The label is a child of the ColorRect, so toggling the rect toggles both.
	interactable_area_debug.visible = not Global.is_production() and show_interactable_area
	if not interactable_area_debug.visible:
		return
	interactable_area_debug.position = area.position
	interactable_area_debug.size = area.size

	var canvas: Vector2 = ui_safe_area.size
	var m_left: float = safe_area_hud.get_theme_constant("margin_left")
	var m_right: float = safe_area_hud.get_theme_constant("margin_right")
	var m_top: float = safe_area_hud.get_theme_constant("margin_top")
	var m_bottom: float = safe_area_hud.get_theme_constant("margin_bottom")
	var device_area: float = canvas.x * canvas.y
	var safe_area: float = (
		maxf(canvas.x - m_left - m_right, 0.0) * maxf(canvas.y - m_top - m_bottom, 0.0)
	)
	var inter_area: float = area.size.x * area.size.y
	var pct_device: int = int(round(100.0 * inter_area / device_area)) if device_area > 0.0 else 0
	var pct_safe: int = int(round(100.0 * inter_area / safe_area)) if safe_area > 0.0 else 0
	interactable_area_debug_label.text = (
		"%d%% of device\n%d%% of safe area" % [pct_device, pct_safe]
	)


## Runtime toggle for the interactable-area overlay (e.g. from Settings > Dev Tools).
func set_interactable_area_visible(value: bool) -> void:
	show_interactable_area = value
	if is_instance_valid(safe_area_hud):
		_update_interactable_area_debug(get_interactable_area())


func _on_player_logout():
	# Funnel any logout signal (e.g. session expiry) into the single canonical
	# teardown instead of quitting the app. Global.sign_out() is re-entrancy
	# guarded, so this is safe even when sign_out() is what emitted the signal.
	Global.sign_out()


## Sever this Explorer from every persistent (autoload / window / Rust singleton)
## emitter and stop its retry timers, while the node is still in the tree. Called
## by Global.sign_out() BEFORE it kills scenes / clears realm / changes orientation,
## so none of those re-emit into this about-to-be-freed Explorer. Idempotent.
func prepare_for_logout() -> void:
	# Drain any in-flight subscription retry loops on their next check.
	_session_active = false
	_subscription_reconnecting = false

	if _resubscribe_timer != null:
		_resubscribe_timer.stop()
		_resubscribe_timer.queue_free()
		_resubscribe_timer = null

	_disconnect_persistent_signals()


## Disconnect this Explorer from persistent emitters (autoloads / Rust singletons).
## Godot auto-severs any connection whose RECEIVER is freed, so the many
## Global.* -> _on_*() UI callbacks are cleaned up automatically when this node is
## freed. We only manually sever the connections Godot would NOT clean up, or that
## can fire synchronously into this node during the sign-out teardown (before the
## deferred free): the Global -> Global connection (leaks one callback per login
## otherwise) and the persistent Rust/autoload emitters used during teardown.
func _disconnect_persistent_signals() -> void:
	_safe_disconnect(Global.scene_runner.on_change_scene_id, _on_change_scene_id)
	_safe_disconnect(Global.scene_runner.pointer_tooltip_changed, _on_pointer_tooltip_changed)
	_safe_disconnect(Global.change_parcel, _on_change_parcel)
	_safe_disconnect(Global.orientation_changed, _on_orientation_changed)
	_safe_disconnect(Global.player_identity.logout, _on_player_logout)

	if Global.avatars != null:
		# Global -> Global: not auto-severed, would leak one callback per login.
		var profile_changed: Signal = Global.player_identity.profile_changed
		_safe_disconnect(profile_changed, Global.avatars.update_primary_player_profile)
		_safe_disconnect(Global.avatars.avatar_added, _on_avatar_added_apply_hide_ui)

	if Global.social_service != null:
		_safe_disconnect(Global.social_service.block_update_received, _on_block_update_received)
		_safe_disconnect(Global.social_service.subscription_dropped, _async_on_subscription_dropped)


func _safe_disconnect(sig: Signal, callable: Callable) -> void:
	if sig.is_connected(callable):
		sig.disconnect(callable)


func _on_player_profile_changed(_profile: DclUserProfile) -> void:
	# Start notifications polling when authenticated
	print("[Explorer] Player profile changed - starting notifications polling")
	NotificationsManager.start_polling()


func _async_initialize_social_service() -> void:
	# Initialize the social service with player identity
	Global.social_service.initialize_from_player_identity(Global.player_identity)

	# Connect to block update signal for real-time sync
	if not Global.social_service.block_update_received.is_connected(_on_block_update_received):
		Global.social_service.block_update_received.connect(_on_block_update_received)

	# Guests have no wallet identity and no friend graph — skip the entire
	# friendship/connectivity flow (subscriptions, retries, and the proactive timer).
	if Global.player_identity.is_guest:
		return

	# Connect subscription_dropped for auto-reconnect
	if not Global.social_service.subscription_dropped.is_connected(_async_on_subscription_dropped):
		Global.social_service.subscription_dropped.connect(_async_on_subscription_dropped)

	_session_active = true

	# Fetch blocked users from server and initialize local cache (fire-and-forget)
	_async_fetch_blocking_status()

	# Subscribe to block updates for real-time sync across devices
	Global.social_service.subscribe_to_block_updates()

	# Subscribe to friendship and connectivity updates persistently
	_async_subscribe_to_friendship_updates(true)
	_async_subscribe_to_connectivity_updates()

	# Start proactive re-subscribe timer (every 30s)
	if _resubscribe_timer == null:
		_resubscribe_timer = Timer.new()
		_resubscribe_timer.wait_time = 30.0
		_resubscribe_timer.autostart = true
		_resubscribe_timer.timeout.connect(_async_proactive_resubscribe)
		add_child(_resubscribe_timer)


func _async_fetch_blocking_status() -> void:
	var promise = Global.social_service.get_blocking_status()
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to get blocking status: ", PromiseUtils.get_error_message(promise))
		return

	var data = promise.get_data()
	if data is Dictionary:
		var blocked_users: Array = data.get("blocked_users", [])
		Global.social_blacklist.init_from_blocking_status(blocked_users)


func _on_block_update_received(address: String, is_blocked: bool) -> void:
	if is_blocked:
		Global.social_blacklist.add_blocked(address)
	else:
		Global.social_blacklist.remove_blocked(address)


## Subscribe to friendship updates with bounded exponential backoff.
## `initial_load`: on success, true triggers a full friends fetch; false a diff refresh
## (used by reconnect-after-drop, which already has data on screen).
func _async_subscribe_to_friendship_updates(initial_load: bool) -> void:
	var attempt: int = 0
	while _session_active:
		var promise = Global.social_service.subscribe_to_updates()
		await PromiseUtils.async_awaiter(promise)
		if not _session_active:
			return

		if not promise.is_rejected():
			friends_panel.set_streaming_subscription_failed(false)
			if initial_load:
				friends_panel.async_initial_friends_load()
			else:
				friends_panel.async_refresh_friends()
			return

		attempt += 1
		push_error(
			(
				"[FriendsPanel.SubscriptionState] friendship subscribe rejected (attempt %d/%d): %s"
				% [attempt, _SUBSCRIBE_RETRY_MAX_ATTEMPTS, PromiseUtils.get_error_message(promise)]
			)
		)
		friends_panel.set_streaming_subscription_failed(true)

		if attempt >= _SUBSCRIBE_RETRY_MAX_ATTEMPTS:
			return

		var delay: float = min(
			_SUBSCRIBE_RETRY_BASE_DELAY * pow(2.0, attempt - 1), _SUBSCRIBE_RETRY_MAX_DELAY
		)
		await get_tree().create_timer(delay).timeout


func _async_subscribe_to_connectivity_updates() -> void:
	var attempt: int = 0
	while _session_active:
		var promise = Global.social_service.subscribe_to_connectivity_updates()
		await PromiseUtils.async_awaiter(promise)
		if not _session_active:
			return

		if not promise.is_rejected():
			return

		attempt += 1
		push_error(
			(
				"[FriendsPanel.SubscriptionState] connectivity subscribe rejected (attempt %d/%d): %s"
				% [attempt, _SUBSCRIBE_RETRY_MAX_ATTEMPTS, PromiseUtils.get_error_message(promise)]
			)
		)

		if attempt >= _SUBSCRIBE_RETRY_MAX_ATTEMPTS:
			return

		var delay: float = min(
			_SUBSCRIBE_RETRY_BASE_DELAY * pow(2.0, attempt - 1), _SUBSCRIBE_RETRY_MAX_DELAY
		)
		await get_tree().create_timer(delay).timeout


func _async_proactive_resubscribe() -> void:
	if not _session_active:
		return
	# Re-subscribe silently (cancels old subscription, creates new one)
	var promise = Global.social_service.subscribe_to_updates()
	await PromiseUtils.async_awaiter(promise)
	if not _session_active:
		return
	if promise.is_rejected():
		return  # Silent failure — subscription_dropped will handle recovery
	# Diff-based refresh (no full rebuild)
	friends_panel.async_refresh_friends()

	# Also re-subscribe connectivity
	Global.social_service.subscribe_to_connectivity_updates()
	Global.social_service.subscribe_to_block_updates()


func _async_on_subscription_dropped() -> void:
	if not _session_active:
		return
	# Debounce: multiple streams may drop at once when connection dies
	if _subscription_reconnecting:
		return
	_subscription_reconnecting = true
	print("[FriendsPanel.SubscriptionState] subscription dropped — reconnecting in 2s")
	await get_tree().create_timer(2.0).timeout
	_subscription_reconnecting = false
	if not _session_active:
		return
	Global.social_service.subscribe_to_block_updates()
	_async_subscribe_to_friendship_updates(false)
	_async_subscribe_to_connectivity_updates()


func _on_scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	_scene_console_message.call_deferred(scene_id, level, timestamp, text)


func _scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	var title: String = Global.scene_runner.get_scene_title(scene_id)
	title += str(Global.scene_runner.get_scene_base_parcel(scene_id))
	if is_instance_valid(preview_hud_panel):
		preview_hud_panel.on_console_add(title, level, timestamp, text)


func _on_pointer_tooltip_changed():
	change_tooltips.call_deferred()


func change_tooltips():
	var tooltip_data = Global.scene_runner.pointer_tooltips.duplicate()

	# Check if there's an avatar behind the crosshair
	if _session_hide_main_hud and _session_hide_view_profile:
		_avatar_under_crosshair = null
	else:
		_avatar_under_crosshair = player.get_avatar_under_crosshair()

	# Handle outline changes through the outline system
	if _avatar_under_crosshair != _last_outlined_avatar:
		player.outline_system.set_outlined_avatar(_avatar_under_crosshair)
		_last_outlined_avatar = _avatar_under_crosshair

	# Handle the highlight (outline) for scene objects with show_highlight=true
	var highlighted_entity: Node3D = Global.scene_runner.highlighted_entity
	if not is_instance_valid(highlighted_entity):
		highlighted_entity = null
	if _session_hide_main_hud and _session_hide_world_interactions:
		highlighted_entity = null
	if highlighted_entity != _last_outlined_entity:
		player.outline_system.set_outlined_entity(highlighted_entity)
		_last_outlined_entity = highlighted_entity

	# Filter tooltips based on hide UI sub-toggles
	if _session_hide_main_hud and (_session_hide_view_profile or _session_hide_world_interactions):
		var filtered = []
		for i in tooltip_data.size():
			var entry = tooltip_data[i]
			var is_view_profile = (
				entry is Dictionary and entry.get("text_pet_down", "") == "View profile"
			)
			if is_view_profile and _session_hide_view_profile:
				continue
			if not is_view_profile and _session_hide_world_interactions:
				continue
			filtered.append(entry)
		tooltip_data = filtered

	# Tooltips now include avatar detection from scene_runner
	if not tooltip_data.is_empty():
		control_pointer_tooltip.set_pointer_data(tooltip_data)
		control_pointer_tooltip.show()
	else:
		control_pointer_tooltip.hide()


func _on_check_button_toggled(button_pressed):
	Global.scene_runner.set_pause(button_pressed)


func _unhandled_input(event):
	if not Global.is_mobile():
		if event is InputEventMouseButton and event.pressed and ui_root.has_focus():
			if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
				capture_mouse()

	if event is InputEventKey and ui_root.has_focus():
		if event.pressed and event.keycode == KEY_ESCAPE:
			if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
				release_mouse()


func _on_control_minimap_request_open_map():
	if !control_menu.visible:
		control_menu.show_map()
		release_mouse()


func _on_control_menu_jump_to(parcel: Vector2i):
	teleport_to(parcel)
	control_menu.async_close()


func _on_control_menu_hide_menu():
	control_menu.async_close()
	ui_root.grab_focus()


func _on_control_menu_toggle_fps(visibility):
	if DclGlobal.is_production():
		return
	label_fps.visible = visibility


func _on_virtual_joystick_right_stick_position(stick_position: Vector2):
	player.stick_position = stick_position


func _on_virtual_joystick_right_is_hold(hold: bool):
	player.stick_holded = hold


func _on_touch_screen_button_pressed():
	Input.action_press("ia_jump")


func _on_touch_screen_button_released():
	Input.action_release("ia_jump")


func _on_panel_chat_submit_message(message: String):
	_chat_commands.submit_message(message)


func _check_multiplayer_debug_deeplink() -> void:
	if Global.deep_link_obj.multiplayer_debug or Global.comms.get_multiplayer_debug():
		_on_control_menu_request_multiplayer_debug(true)


func _on_control_menu_request_multiplayer_debug(enabled):
	Global.comms.set_multiplayer_debug(enabled)
	if enabled:
		if not is_instance_valid(multiplayer_debug_panel):
			multiplayer_debug_panel = _MULTIPLAYER_DEBUG_PANEL_SCENE.instantiate()
			ui_root.add_child(multiplayer_debug_panel)
	elif is_instance_valid(multiplayer_debug_panel):
		ui_root.remove_child(multiplayer_debug_panel)
		multiplayer_debug_panel.queue_free()
		multiplayer_debug_panel = null


func _on_control_menu_request_pause_scenes(enabled):
	Global.scene_runner.set_pause(enabled)


## Moves the player to a specific position
##
## @param position: The 3D position to move the player to
## @param skip_loading: When true, skips showing the loading screen.
##                      This is used when teleporting inside a scene to avoid
##                      showing the loading UI for an already-loaded area.
func move_to(position: Vector3, skip_loading: bool, check_stuck: bool = true):
	if disable_move_to:
		return

	# Set grace period on avatar's emote controller to prevent emote cancellation during teleport
	if player.avatar and player.avatar.emote_controller:
		player.avatar.emote_controller.set_teleport_grace()

	player.move_to(position, check_stuck)
	# Announce the instant reposition to the Pulse transport so remote peers
	# snap to it instead of interpolating across the jump (no-op when inactive).
	Global.comms.notify_player_teleported(position)
	var cur_parcel_position = Vector2i(
		floor(player.position.x * 0.0625), -floor(player.position.z * 0.0625)
	)
	if not skip_loading:
		if not Global.scene_fetcher.is_scene_loaded(cur_parcel_position.x, cur_parcel_position.y):
			if not loading_ui.visible:
				loading_ui.enable_loading_screen("", "on_moveto")


func teleport_to(parcel: Vector2i, realm: String = ""):
	_async_teleport_to(parcel, realm)


func _async_teleport_to(parcel: Vector2i, realm: String = "") -> void:
	if not realm.is_empty() and realm != Global.realm.get_realm_string():
		var success = await Global.realm.async_set_realm(realm)
		if not success:
			return
		if not loading_ui.visible:
			loading_ui.enable_loading_screen(realm, "on_teleport")

	var move_to_position = Vector3i(parcel.x * 16 + 8, 3, -parcel.y * 16 - 8)
	move_to(move_to_position, false)

	Global.scene_fetcher.update_position(parcel, true)

	Global.get_config().add_place_to_last_places(parcel, realm)
	dirty_save_position = true


func player_look_at(look_at_position: Vector3):
	if not Global.is_xr():
		player.avatar_look_at(look_at_position)


func camera_look_at(look_at_position: Vector3):
	if not Global.is_xr():
		player.camera_look_at(look_at_position)


func avatar_look_at_independent(look_at_position: Vector3):
	if not Global.is_xr():
		player.set_avatar_rotation_independent(look_at_position)


func capture_mouse():
	if DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	if label_crosshair and ui_root:
		if not Global.touch_controls_hide_crosshair:  # respect scene-driven crosshair hide
			label_crosshair.show()
		ui_root.grab_focus.call_deferred()


func release_mouse():
	if DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	if not Global.is_mobile():
		if label_crosshair:
			label_crosshair.hide()


func set_visible_ui(value: bool, use_hud_mode: bool = false):
	if use_hud_mode:
		_set_explorer_hud_elements_visible(value)
		return

	# External callers (e.g. scene capture): if session "hide UI" is on, restoring the
	# root must reapply minimized HUD + show-UI button, not only ui_root.show().
	if value and _session_hide_main_hud:
		_set_explorer_hud_elements_visible(false)
		return

	if value == ui_root.visible:
		return

	if value:
		ui_root.show()
	else:
		ui_root.hide()

	if value:
		button_show_ui.hide()


func _is_ui_hud_mode_exception(node: Node) -> bool:
	return (
		node == ui_safe_area
		or node == control_menu
		or node == safe_area_hud
		or node == profile_container
		or node == safe_area_controls
		or node == virtual_joystick
		or node == mobile_camera_input
	)


# Whether the on-screen controls (joypad + virtual joystick) are present. They show on
# mobile and on desktop (development), but never in XR.
func _onscreen_controls_enabled() -> bool:
	return not Global.is_xr()


func _apply_mobile_controls_hide_ui(hidden: bool) -> void:
	if not _onscreen_controls_enabled():
		return
	_mobile_controls_hidden_for_hide_ui = hidden
	if hidden:
		joypad.hide()
		# Keep the joystick functional (walking) but fully transparent. A hidden Control
		# stops receiving input, so show() it and only drop its alpha — an overlay may have
		# left it hidden before Hide-UI kicked in.
		virtual_joystick.show()
		virtual_joystick.modulate.a = 0.0
	else:
		joypad.show()
		virtual_joystick.modulate.a = 1.0


func _show_joypad() -> void:
	if _mobile_controls_hidden_for_hide_ui:
		return
	joypad.show()


## Hide the on-screen movement controls (joypad + left joystick) while an overlay
## (navbar, chat focus or emote wheel) owns the screen.
func _hide_movement_controls() -> void:
	joypad.hide()
	virtual_joystick.hide()


## Restore the on-screen movement controls hidden by _hide_movement_controls().
func _show_movement_controls() -> void:
	_show_joypad()
	virtual_joystick.show()


func _set_explorer_hud_elements_visible(full_hud: bool) -> void:
	ui_root.show()
	_apply_mobile_controls_hide_ui(not full_hud)
	if full_hud:
		# SafeAreaHud stays visible (exception); we toggle its content group so the
		# Show-UI button (a sibling of hud_content) survives and children keep their state.
		hud_content.show()
		# Invariant: showing the HUD always means the bottom-left slot is restored. Hide-UI
		# is applied on navbar-close, where _close_all_panels' normal "re-show" is suppressed
		# while hidden — so it is always left hidden by the time we restore, and we must
		# re-assert it here. In preview this shows the preview HUD toolbar instead of the chat.
		# (Write-mode can't leak a hidden chatbar: writing hides the navbar, so Settings/hide-UI
		# is unreachable while writing.)
		_restore_bottom_left_hud()
		# Re-assert the emote button too: an overlay (e.g. the navbar) may have left it
		# hidden before Hide-UI was applied, and hud_content.show() won't re-show a child
		# whose own visibility is false. Portrait owns it through the orientation flow.
		if not Global.is_orientation_portrait():
			emote_wheel.show()
		for node in _ui_children_hidden_for_hud_mode:
			if is_instance_valid(node):
				node.show()
		_ui_children_hidden_for_hud_mode.clear()
		button_show_ui.hide()
		return

	hud_content.hide()
	for child in ui_root.get_children():
		if _is_ui_hud_mode_exception(child):
			continue
		if not child is CanvasItem:
			continue
		var canvas_child := child as CanvasItem
		if canvas_child.visible:
			_ui_children_hidden_for_hud_mode.append(canvas_child)
			canvas_child.hide()

	button_show_ui.show()


func _on_control_menu_request_debug_panel(enabled):
	_debug_panel_from_settings = enabled
	_update_preview_hud()


## Whether the preview HUD toolbar should exist at all: the settings Scene Logs
## toggle (or --debug-panel), a preview realm, or a `scene-stats=true` deep link.
## The chat/hide-UI restore paths use it to pick chat vs toolbar in the bottom-left slot.
func _preview_panel_active() -> bool:
	return _debug_panel_from_settings or _is_in_preview_realm() or Global.deep_link_obj.scene_stats


## Whether the scene-stats overlay (and its header button) is offered: only a preview
## realm or a `scene-stats=true` deep link — not the settings Scene Logs entry, which
## exposes console + reload only.
func _scene_stats_available() -> bool:
	return _is_in_preview_realm() or Global.deep_link_obj.scene_stats


## Keep the (static) preview HUD toolbar in sync: create/free the scene-stats overlay, point
## it at the previewed scene, gate scene logs, and show/hide the toolbar. The console/debug
## panel is always present (static) so it keeps capturing logs; only scene-stats is on demand.
func _update_preview_hud() -> void:
	if not is_instance_valid(preview_hud_panel):
		return
	var active: bool = _preview_panel_active()
	preview_hud_panel.set_scene_status_available(_scene_stats_available())
	if active:
		preview_hud_panel.set_scene(_preview_scene_id())
	Global.set_scene_log_enabled(active)
	_restore_bottom_left_hud()


## True while a navbar side panel (or the dropdown) is open — the bottom-left slot hides then.
func _bottom_left_slot_blocked() -> bool:
	return (
		navbar.is_open()
		or friends_panel.visible
		or notifications_panel.visible
		or settings_panel.visible
	)


## Restore the bottom-left slot: while a navbar panel is open it stays hidden; otherwise the
## active toolbar is shown (reset to header-only) and the chat hidden, else the chat owns the
## slot. Hide-UI still wins (nothing force-shown while the HUD is hidden).
func _restore_bottom_left_hud() -> void:
	if _bottom_left_slot_blocked():
		_hide_bottom_left_hud()
		return
	if _preview_panel_active():
		if is_instance_valid(preview_hud_panel):
			preview_hud_panel.reset()
			preview_hud_panel.show()
		chat_panel.hide()
	elif not _session_hide_main_hud:
		if is_instance_valid(preview_hud_panel):
			preview_hud_panel.hide()
		chat_panel.show()


## Hide the whole bottom-left slot (chat and the preview toolbar, header included) while the
## navbar is open with a side panel showing; the navbar-collapse path restores it.
func _hide_bottom_left_hud() -> void:
	chat_panel.hide()
	if is_instance_valid(preview_hud_panel):
		preview_hud_panel.hide()


## The single scene being previewed (one scene may span multiple parcels):
## prefer the non-global scene at the player's parcel, else the first non-global
## scene loaded; -1 if none.
func _preview_scene_id() -> int:
	if not is_instance_valid(Global.scene_runner):
		return -1
	var sid := int(Global.scene_runner.get_scene_id_by_parcel_position(parcel_position))
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode and not child.is_global() and child.get_scene_id() == sid:
			return sid
	for node in Global.scene_runner.get_children():
		if node is DclSceneNode and not node.is_global():
			return node.get_scene_id()
	return -1


func _on_timer_fps_label_timeout():
	var fps_text = "- " + str(Engine.get_frames_per_second()) + " FPS"

	# Add dynamic graphics info if enabled
	if Global.get_config().dynamic_graphics_enabled:
		var dm = Global.dynamic_graphics_manager
		if dm == null:
			label_fps.set_text(fps_text)
			return
		var profile_name = GraphicSettings.PROFILE_NAMES[dm.get_current_profile()]

		if DclGlobal.is_production():
			fps_text += " | DynGfx: %s | %s" % [dm.get_state_name(), profile_name]
		else:
			fps_text += (
				" | DynGfx: %s | R:%.2f | T:%s | %s"
				% [
					dm.get_state_name(),
					dm.get_frame_time_ratio(),
					dm.get_thermal_state_string(),
					profile_name
				]
			)

	# Show JNI timing stats on Android (debug builds only, returns 0 in release)
	if DclAndroidPlugin.is_available():
		var jni_ms = DclAndroidPlugin.take_jni_time_ms()
		var jni_calls = DclAndroidPlugin.take_jni_call_count()
		if jni_calls > 0:
			fps_text += " | JNI: %.2fms (%d)" % [jni_ms, jni_calls]

	label_fps.set_text(fps_text)
	if dirty_save_position:
		dirty_save_position = false
		Global.get_config().save_to_settings_file()


func hide_menu():
	control_menu.async_close()
	release_mouse()


func set_cursor_position(position: Vector2):
	if Global.scene_runner.raycast_use_cursor_position:
		var crosshair_position = position - (label_crosshair.size / 2) - Vector2(0, 1)
		label_crosshair.set_global_position(crosshair_position)
		control_pointer_tooltip.set_global_cursor_position(position)
		Global.scene_runner.set_cursor_position(position)


func reset_cursor_position():
	# Position crosshair at center of screen
	var viewport_size = get_tree().root.get_viewport().get_visible_rect()
	var center_position = viewport_size.size * 0.5
	var crosshair_position = center_position - (label_crosshair.size / 2) - Vector2(0, 1)
	label_crosshair.set_global_position(crosshair_position)
	control_pointer_tooltip.set_global_cursor_position(center_position)


func _on_panel_profile_open_profile():
	_open_own_profile()


func _on_button_load_scenes_pressed() -> void:
	Global.scene_fetcher._bypass_loading_check = true
	chat_panel.hide_load_scenes_button()


func _is_in_preview_realm() -> bool:
	var preview_url := Global.deep_link_obj.preview
	if not preview_url.is_empty():
		return Global.realm.realm_string == preview_url
	return Global.cli.preview_mode


func _update_preview_ui(_in_preview: bool) -> void:
	_update_preview_hud()


func _on_notify_pending_loading_scenes(pending: bool) -> void:
	if pending:
		chat_panel.show_load_scenes_button()
		if _first_time_refresh_warning:
			if loading_ui.visible:
				return
			(
				warning_messages
				. async_create_popup_warning(
					PopupWarning.WarningType.MESSAGE,
					"Load the scenes arround you",
					"[center]You have scenes pending to be loaded. To maintain a smooth experience, loading will occur only when you change scenes. If you prefer to load them immediately, please press the [b]Refresh[/b] button at the Top Left of the screen with icon [img]res://assets/ui/Reset.png[/img][/center]"
				)
			)
			_first_time_refresh_warning = false
	else:
		chat_panel.hide_load_scenes_button()


func _open_profile(dcl_user_profile: DclUserProfile):
	chat_panel.chat.exit_chat()
	profile_container.async_open(dcl_user_profile)
	release_mouse()


func _on_profile_container_visibility_changed() -> void:
	if _session_hide_main_hud:
		# Keep profile visibility controlled by its own open/close flow in Hide UI mode.
		# Avoid forcing hide/show here to prevent visibility_changed re-entrancy loops.
		return
	if not profile_container.visible:
		_show_joypad()
		# Profile page grabs keyboard focus when shown; restore it so jump works.
		Global.explorer_grab_focus()
		capture_mouse()


func _open_friends_panel() -> void:
	# Opening the navbar overlays the HUD. Fully close the chat (not just hide it) so it
	# reappears un-focused — notifications only, chatbar button un-toggled — when the navbar
	# collapses. close_chat also resets the chatbar toggle (which exit_chat alone doesn't).
	# Then close the emote overlay and hide the emote button/wheel, movement controls and
	# the whole chat. The hides run last so they win over the restores those closes emit.
	if chat_panel.is_chat_visible():
		Global.close_chat.emit()
	emote_wheel.close()
	Global.close_menu.emit()
	Global.open_friends_panel.emit()
	emote_wheel.hide()
	_hide_movement_controls()
	_hide_bottom_left_hud()


func _async_open_profile_by_address(user_address: String):
	var promise = Global.content_provider.fetch_profile(user_address)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error getting player profile: ", result.get_error())
		return

	if result != null and result is DclUserProfile:
		_open_profile(result)


func _async_open_profile_by_avatar(avatar: DclAvatar):
	if _session_hide_main_hud and _session_hide_view_profile:
		return
	# Check if it's an Avatar (GDScript class) to access avatar_id
	if avatar is Avatar:
		var avatar_instance = avatar as Avatar
		var avatar_id = avatar_instance.avatar_id
		if not avatar_id.is_empty():
			# Don't open profile for blocked users
			if Global.social_blacklist.is_blocked(avatar_id):
				return
			await _async_open_profile_by_address(avatar_id)
		else:
			printerr(
				"_async_open_profile_by_avatar: avatar_id is empty for avatar: ",
				avatar_instance.name
			)
	else:
		# Try to get avatar_id from metadata if available (fallback)
		if avatar.has_method("get") and avatar.get("avatar_id") != null:
			var avatar_id = avatar.get("avatar_id")
			if avatar_id is String and not avatar_id.is_empty():
				await _async_open_profile_by_address(avatar_id)
			else:
				printerr(
					"_async_open_profile_by_avatar: avatar is not an Avatar instance and avatar_id is not available"
				)
		else:
			printerr(
				"_async_open_profile_by_avatar: avatar is not an Avatar instance: ",
				avatar.get_class()
			)


func _on_control_menu_open_profile() -> void:
	_open_own_profile()


func _on_global_open_own_profile() -> void:
	if Global.is_orientation_portrait():
		return
	if friends_panel.visible:
		friends_panel.hide_panel()
	if notifications_panel.visible:
		notifications_panel.hide_panel()
	if settings_panel.visible:
		settings_panel.hide()
	navbar.collapse()
	_open_own_profile()


func _open_own_profile() -> void:
	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		_open_profile(profile)


func _get_viewport_scale_factors() -> Vector2:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size = get_viewport().get_visible_rect().size
	var x_factor: float = viewport_size.x / window_size.x
	var y_factor: float = viewport_size.y / window_size.y
	return Vector2(x_factor, y_factor)


func _show_friends_panel() -> void:
	if friends_panel.visible:
		return
	joypad.hide()
	friends_panel.show_panel_on_friends_tab()
	if notifications_panel.visible:
		notifications_panel.hide_panel()
	if settings_panel.visible:
		settings_panel.hide()
	_refresh_hud_dismiss()
	Global.explorer_release_focus()
	if Global.is_mobile():
		release_mouse()


func _on_friends_panel_closed() -> void:
	friends_panel.hide_panel()
	Global.explorer_grab_focus()
	capture_mouse()


func _show_settings_panel() -> void:
	if settings_panel.visible:
		return
	joypad.hide()
	settings_panel.show()
	if friends_panel.visible:
		friends_panel.hide_panel()
	if notifications_panel.visible:
		notifications_panel.hide_panel()
	_refresh_hud_dismiss()
	Global.explorer_release_focus()
	if Global.is_mobile():
		release_mouse()


func _on_settings_panel_closed() -> void:
	settings_panel.hide()
	apply_deferred_hide_ui()
	Global.explorer_grab_focus()
	capture_mouse()


func _show_notifications_panel() -> void:
	if notifications_panel.visible:
		return
	joypad.hide()
	notifications_panel.show_panel()
	if friends_panel.visible:
		friends_panel.hide_panel()
	if settings_panel.visible:
		settings_panel.hide()
	_refresh_hud_dismiss()
	Global.explorer_release_focus()
	if Global.is_mobile():
		release_mouse()


func _on_notifications_panel_closed() -> void:
	notifications_panel.hide_panel()
	Global.explorer_grab_focus()
	capture_mouse()


func _on_notification_queued(notification_d: Dictionary) -> void:
	# Only show notifications if not loading
	if not _is_loading:
		_show_notification_toast(notification_d)
	else:
		# Store the notification to show after loading finishes
		if _pending_notification_toast.is_empty():
			_pending_notification_toast = notification_d


func _show_notification_toast(notification_d: Dictionary) -> void:
	# Filter out friend request notifications from blocked users
	var notif_type = notification_d.get("type", "")
	if notif_type == "social_service_friendship_request":
		var sender_address = ""
		if "metadata" in notification_d and notification_d["metadata"] is Dictionary:
			var metadata = notification_d["metadata"]
			if "sender" in metadata and metadata["sender"] is Dictionary:
				sender_address = metadata["sender"].get("address", "")

		# Skip showing notification if sender is blocked
		if not sender_address.is_empty() and Global.social_blacklist.is_blocked(sender_address):
			# Immediately dequeue this notification and try to show next one
			NotificationsManager.dequeue_notification()
			return

	# Create and show toast notification
	var style = notification_d.get("toast_style", "default")
	var scene_path := "res://src/ui/components/organisms/notifications/notification_toast.tscn"
	if style == "alert":
		scene_path = "res://src/ui/components/organisms/notifications/alert_toast.tscn"
	var toast_scene = load(scene_path)
	var toast = toast_scene.instantiate()
	ui_root.add_child(toast)

	# Connect to toast signals
	toast.toast_closed.connect(_on_toast_closed)
	toast.mark_as_read.connect(_on_toast_mark_as_read)

	toast.async_show_notification(notification_d)


func _on_toast_closed() -> void:
	# Dequeue the current notification and check for next one
	NotificationsManager.dequeue_notification()


func _on_toast_mark_as_read(notification_d: Dictionary) -> void:
	# Mark notification as read via drag gesture
	var notification_id = notification_d.get("id", "")
	if not notification_id.is_empty():
		var ids = PackedStringArray([notification_id])
		NotificationsManager.mark_as_read(ids)


func _on_loading_started() -> void:
	_is_loading = true
	_ban_check_generation += 1
	Global.modal_manager.ban_pre_check_active = false
	_pending_notification_toast = {}  # Clear any pending notification
	_session_hide_main_hud = false
	_session_hide_view_profile = true
	_session_hide_world_interactions = true
	_session_hide_player_names = true
	_session_hide_scene_ui = true
	set_visible_ui(true, true)
	Global.session_hide_ui_toggle_sync.emit(false)
	Global.session_hide_ui_options_sync.emit(true, true, true, true)
	_apply_hide_ui_to_avatar_nicks(false)


func _on_loading_finished() -> void:
	_is_loading = false
	_update_version_label()
	# Show pending notification if there was one queued during loading
	if not _pending_notification_toast.is_empty():
		_show_notification_toast(_pending_notification_toast)
		_pending_notification_toast = {}
	if not Global.modal_manager.ban_pre_check_active:
		_async_run_ban_check()


func _async_run_ban_check() -> void:
	_ban_check_generation += 1
	var generation = _ban_check_generation

	var realm_name = Global.realm.get_realm_string()
	if realm_name.is_empty():
		return

	var scene_id: String
	if Realm.is_dcl_ens(realm_name):
		scene_id = await Global.async_resolve_world_scene_id(realm_name)
	else:
		var parcel = Global.scene_fetcher.current_position
		scene_id = await Global.async_resolve_scene_entity_id(parcel)

	if scene_id.is_empty() or generation != _ban_check_generation:
		return

	var allowed = await Global.async_check_scene_access(scene_id, realm_name)
	if not allowed and generation == _ban_check_generation:
		Global.modal_manager.ban_pre_check_active = true
		Global.modal_manager.async_show_ban_pre_check_modal()


func _on_orientation_changed(is_portrait: bool) -> void:
	if is_portrait:
		mobile_ui.hide()
		virtual_joystick.hide()
		emote_wheel.hide()
		navbar.hide()
		_set_scene_ui_visible(false)
	else:
		if _onscreen_controls_enabled():
			mobile_ui.show()
			_update_virtual_controls_visibility()
		emote_wheel.show()
		navbar._on_size_changed()
		_set_scene_ui_visible(_should_show_scene_ui())


func _on_chat_write_mode_changed(is_writing: bool) -> void:
	if Global.is_orientation_portrait():
		return
	# Emote button and movement controls are hidden for the whole chat-focus session
	# (owned by _on_chat_focus_entered/_exited). Write mode additionally hides the navbar
	# bar (focus only collapses its dropdown) and the extra HUD elements the keyboard
	# needs gone; exiting write restores them while the chat stays focused.
	# The preview HUD toolbar and the chat share the bottom-left slot and never
	# coexist (the toolbar hides the chat), so chat write mode can't overlap it —
	# no toolbar handling is needed here.
	if is_writing:
		navbar.hide()
		_set_scene_ui_visible(false)
	else:
		navbar._on_size_changed()
		_set_scene_ui_visible(_should_show_scene_ui())


func _on_chat_focus_entered() -> void:
	# Chat focus closes the navbar (collapse its dropdown/panels, keeping the bar) and
	# the emote overlay, then hides the emote button and movement controls. The hides run
	# last so they win over the restores those closes emit. Portrait and Hide-UI own
	# visibility through their own flows.
	if _session_hide_main_hud or Global.is_orientation_portrait():
		return
	emote_wheel.close()
	if navbar.is_open():
		navbar.collapse()
	emote_wheel.hide()
	_hide_movement_controls()


func _on_chat_focus_exited() -> void:
	# Restore the emote button and movement controls hidden while the chat was focused.
	if _session_hide_main_hud or Global.is_orientation_portrait():
		return
	emote_wheel.show()
	_show_movement_controls()


func _should_show_scene_ui() -> bool:
	return not (_session_hide_main_hud and _session_hide_scene_ui)


func _set_scene_ui_visible(is_visible: bool) -> void:
	var base_ui = Global.scene_runner.base_ui
	if is_instance_valid(base_ui):
		base_ui.visible = is_visible


func _update_version_label() -> void:
	var version_text = DclGlobal.get_version_with_env()
	if not DclGlobal.is_production() and Global.content_provider.get_optimized_scene_count() > 0:
		version_text += " - Opt"
	label_version.set_text(version_text)


func _on_notification_clicked(notification_d: Dictionary) -> void:
	# Handle friend request notification clicks - open friends panel on friends tab
	var notif_type = notification_d.get("type", "")

	if ["social_service_friendship_request", "social_service_friendship_accepted"].has(notif_type):
		# Open friends panel on friends tab
		if not friends_panel.visible:
			friends_panel.show_panel_on_friends_tab()
			navbar.open_navbar_silently()
			navbar.set_button_pressed(navbar.BUTTON.FRIENDS)
			if notifications_panel.visible:
				notifications_panel.hide_panel()
			if settings_panel.visible:
				settings_panel.hide()
			_refresh_hud_dismiss()
			# Release focus to prevent camera rotation while panel is open
			Global.explorer_release_focus()
			if Global.is_mobile():
				release_mouse()
			joypad.hide()
			# Keep the bottom-left slot hidden while the navbar friends panel is open.
			_hide_bottom_left_hud()


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_IN:
		# Clear badge when app comes to foreground
		NotificationsManager.clear_badge_and_delivered_notifications()

		# Resync notification queue to clean up fired notifications and reschedule next batch
		NotificationsManager.force_queue_sync()

		Global.deep_link_router.route()


func _on_deep_link_jump() -> void:
	control_menu.async_show_discover()
	if is_instance_valid(control_menu.control_discover.instance):
		# Only open the sheet when it actually has a place loaded — a deeplink with no
		# navigation target must land on Discover itself, not an empty "Scene Title" card.
		if not control_menu.control_discover.instance.jump_in.item_data.is_empty():
			control_menu.control_discover.instance.jump_in.open_panel()


func _on_deep_link_open_event(event_id: String) -> void:
	control_menu.async_show_discover()
	if is_instance_valid(control_menu.control_discover.instance):
		control_menu.control_discover.instance.async_open_event_by_id(event_id)


func _on_deep_link_open_place(place_id: String) -> void:
	control_menu.async_show_discover()
	if is_instance_valid(control_menu.control_discover.instance):
		control_menu.control_discover.instance.async_open_place_by_id(place_id)


func _on_emote_wheel_emote_wheel_closed() -> void:
	# Restore the chat messages and the movement joystick hidden while the wheel was open,
	# unless the main HUD is hidden. The joypad stays visible with the wheel, so it isn't
	# touched here. Portrait owns the joystick in its own flow.
	if _session_hide_main_hud:
		return
	chat_panel.show_messages()
	if not Global.is_orientation_portrait():
		virtual_joystick.show()


func _on_emote_wheel_emote_wheel_opened() -> void:
	# The emote wheel keeps its own button, the chatbar and the joypad visible, but takes
	# over the rest of the HUD: collapse the navbar and close its side panels, then hide the
	# chat messages and the movement joystick. The hides run last so they win over the
	# restores emitted while collapsing the navbar.
	if navbar.is_open():
		navbar.collapse()
	else:
		_close_all_panels()
	chat_panel.hide_messages()
	virtual_joystick.hide()


func _update_virtual_controls_visibility() -> void:
	if _mobile_controls_hidden_for_hide_ui:
		joypad.hide()
		# Transparent but functional, so walking keeps working while the UI is hidden.
		virtual_joystick.show()
		virtual_joystick.modulate.a = 0.0
		return
	var panel_open := (
		friends_panel.visible
		or notifications_panel.visible
		or settings_panel.visible
		or profile_container.visible
	)
	if not panel_open:
		_show_joypad()
	virtual_joystick.show()


func _on_backpack_open(_on_emotes := false) -> void:
	# Backpack is a fullscreen menu screen: collapse and hide the navbar just like
	# Discover, for both entry points (navbar button and emote wheel).
	_enter_menu_screen()


func _close_all_panels():
	control_menu.async_close()
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	_on_settings_panel_closed()
	_refresh_hud_dismiss()
	# Restore the bottom-left slot (chat, or the preview HUD toolbar in preview) and the
	# emote HUD hidden while the navbar was open, unless the main HUD is hidden. Keep the
	# emote HUD and joystick hidden in portrait, where the orientation flow owns them.
	if not _session_hide_main_hud:
		_restore_bottom_left_hud()
		if not Global.is_orientation_portrait():
			emote_wheel.show()
			virtual_joystick.show()
	_show_joypad()


func _on_discover_open():
	_enter_menu_screen()


# Shared cleanup when entering a fullscreen menu screen (Discover / Backpack):
# collapse the navbar dropdown, close the side panels and hide the navbar.
func _enter_menu_screen():
	navbar.collapse()
	_show_joypad()
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	_on_settings_panel_closed()
	_refresh_hud_dismiss()
	navbar.set_manually_hidden(true)
	release_mouse()


func _on_menu_open():
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	_on_settings_panel_closed()
	_refresh_hud_dismiss()
	release_mouse()


func _on_menu_close():
	Global.set_orientation_landscape()
	if !navbar.visible:
		navbar.set_manually_hidden(false)
		release_mouse()


func _extract_short_realm_url(full_url: String) -> String:
	var url_trimmed = full_url.trim_suffix("/")
	var parts = url_trimmed.split("/")
	if parts.size() > 0:
		return parts[parts.size() - 1]
	return full_url


func _share_place():
	var msg: String
	var url: String

	if is_genesis_city:
		var share_position = parcel_position
		# If we're in an empty parcel and there's exactly one loaded scene, use that scene's position
		var current_scene_id = Global.scene_runner.get_current_parcel_scene_id()
		if current_scene_id == -1 and Global.scene_fetcher.loaded_scenes.size() == 1:
			var scene: SceneFetcher.SceneItem = Global.scene_fetcher.loaded_scenes.values()[0]
			if scene.parcels.size() > 0:
				share_position = scene.parcels[0]

		url = (
			"https://mobile.dclexplorer.com/open?position="
			+ str(share_position[0])
			+ ","
			+ str(share_position[1])
		)
	else:
		var realm_url = Global.realm.realm_url
		var short_realm_url = _extract_short_realm_url(realm_url)
		url = "https://mobile.dclexplorer.com/open?realm=" + short_realm_url

	if scene_title.length() == 0:
		scene_title = "Decentraland"
	msg = "📍 Join Me At " + scene_title + " following this link: " + url
	#+ "\n\n If you haven't installed the app yet -> https://install-mobile.decentraland.org 📲"

	if Global.is_android():
		DclAndroidPlugin.share_text(msg)
	elif Global.is_ios():
		DclIosPlugin.share_text(msg)


func _on_change_scene_id(scene_id: int):
	is_genesis_city = Realm.is_genesis_city(Global.realm.realm_url)
	if scene_id == -1:
		scene_title = ""
		_update_preview_ui(false)
		return
	var scene = Global.scene_fetcher.get_scene_data_by_scene_id(scene_id)
	if scene != null:
		scene_title = scene.scene_entity_definition.get_title()
	else:
		scene_title = ""
	_update_preview_ui(_is_in_preview_realm())


func _on_change_parcel(_position: Vector2i):
	parcel_position = _position


func _on_hud_dismiss_catcher_gui_input(event: InputEvent) -> void:
	if (event is InputEventMouseButton or event is InputEventScreenTouch) and event.pressed:
		_close_all_panels()
		navbar.collapse()
		Global.close_chat.emit()
		capture_mouse()


## The full-screen dismiss catcher is STOP (catches empty-area taps) only while a left
## panel is open or the chat is visible; IGNORE otherwise so it never blocks gameplay.
func _refresh_hud_dismiss() -> void:
	var open: bool = (
		notifications_panel.visible
		or friends_panel.visible
		or settings_panel.visible
		or chat_panel.is_chat_visible()
	)
	hud_dismiss_catcher.mouse_filter = (
		Control.MOUSE_FILTER_STOP if open else Control.MOUSE_FILTER_IGNORE
	)


func _on_button_show_ui_pressed() -> void:
	_session_hide_main_hud = false
	_session_hide_view_profile = true
	_session_hide_world_interactions = true
	_session_hide_player_names = true
	_session_hide_scene_ui = true
	set_visible_ui(true, true)
	_set_scene_ui_visible(true)
	Global.session_hide_ui_toggle_sync.emit(false)
	Global.session_hide_ui_options_sync.emit(true, true, true, true)
	_apply_hide_ui_to_avatar_nicks(false)


func set_hide_main_hud_from_settings(minimized: bool) -> void:
	_session_hide_main_hud = minimized
	if not minimized:
		# Turning off: restore UI immediately and reset sub-options
		_session_hide_view_profile = true
		_session_hide_world_interactions = true
		_session_hide_player_names = true
		_session_hide_scene_ui = true
		set_visible_ui(true, true)
		_set_scene_ui_visible(true)
		_apply_hide_ui_to_avatar_nicks(false)
		Global.session_hide_ui_options_sync.emit(true, true, true, true)


func set_hide_view_profile(value: bool) -> void:
	_session_hide_view_profile = value


func set_hide_world_interactions(value: bool) -> void:
	_session_hide_world_interactions = value


func set_hide_player_names(value: bool) -> void:
	_session_hide_player_names = value


func set_hide_scene_ui(value: bool) -> void:
	_session_hide_scene_ui = value
	if _session_hide_main_hud:
		_set_scene_ui_visible(not value)


func is_session_hide_main_hud() -> bool:
	return _session_hide_main_hud


func is_session_hide_view_profile() -> bool:
	return _session_hide_view_profile


func is_session_hide_world_interactions() -> bool:
	return _session_hide_world_interactions


func is_session_hide_player_names() -> bool:
	return _session_hide_player_names


func is_session_hide_scene_ui() -> bool:
	return _session_hide_scene_ui


func apply_deferred_hide_ui() -> void:
	if not _session_hide_main_hud:
		return
	set_visible_ui(false, true)
	_apply_hide_ui_to_avatar_nicks(_session_hide_player_names)
	if _session_hide_scene_ui:
		_set_scene_ui_visible(false)


func _on_avatar_added_apply_hide_ui(avatar = null) -> void:
	# Called when a new avatar is spawned; ensure its nickname obeys current Hide UI state.
	if not _session_hide_main_hud or not _session_hide_player_names:
		return
	if avatar != null and avatar is Avatar:
		(avatar as Avatar).set_force_hide_name(true)
	else:
		_apply_hide_ui_to_avatar_nicks(true)


func _apply_hide_ui_to_avatar_nicks(hide: bool) -> void:
	# Remote avatars
	if Global.avatars:
		var avatars = Global.avatars.get_avatars() if "get_avatars" in Global.avatars else []
		for a in avatars:
			if a is Avatar:
				(a as Avatar).set_force_hide_name(hide)
	# Local player avatar
	if Global.scene_runner and is_instance_valid(Global.scene_runner.player_avatar_node):
		var p = Global.scene_runner.player_avatar_node
		if p is Avatar:
			(p as Avatar).set_force_hide_name(hide)
