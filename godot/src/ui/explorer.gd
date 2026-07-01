class_name Explorer
extends Node

# Friendship/connectivity subscribe retry policy: bounded exponential backoff
# 5s, 10s, 20s, 40s, 60s, 60s — caps at ~3min total before giving up.
const _SUBSCRIBE_RETRY_MAX_ATTEMPTS: int = 6
const _SUBSCRIBE_RETRY_BASE_DELAY: float = 5.0
const _SUBSCRIBE_RETRY_MAX_DELAY: float = 60.0

var is_genesis_city: bool
var player: Node3D = null
var scene_title: String
var parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0
var dirty_save_position: bool = false

var debug_panel = null
var livekit_debug_panel = null
var scene_stats_panel = null
var disable_move_to = false

var virtual_joystick_orig_position: Vector2i

var _int_regex := RegEx.create_from_string(r"^-?\d+$")
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

## True when the debug panel was enabled from settings toggle.
var _debug_panel_from_settings: bool = false

@onready var ui_root: Control = %UI
@onready var ui_safe_area: Control = %SceneUIContainer
@onready var safe_margin_container_debug: SafeMarginContainer = %SafeMarginContainerDebug

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
@onready var label_ram = %Label_RAM
@onready var control_menu = %Control_Menu
@onready var mobile_ui = %MobileUI
@onready var mobile_camera_input: Control = %MobileCameraInput
@onready var left_right_safe_container_mobile: MarginContainer = %LeftRightSafeContainerMobile
@onready var virtual_joystick: Control = %VirtualJoystick_Left
@onready var profile_container: Control = %ProfileContainer

@onready var loading_ui = %Loading

@onready var emote_wheel = %EmoteWheel

@onready var world: Node3D = %world

@onready var timer_broadcast_position: Timer = %Timer_BroadcastPosition

@onready var navbar: Control = %Navbar
@onready var joypad: Control = %Joypad
@onready var h_box_container_right_panels: HBoxContainer = %HBoxContainer_RightPanels
@onready var button_show_ui: Button = %Button_ShowUI
@onready var margin_container_show_ui: MarginContainer = %MarginContainer_ShowUI


func _process(_dt):
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
		label_ram.visible = false

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

	navbar.navbar_closed.connect(_close_all_panels)
	navbar.navbar_opened.connect(_open_friends_panel)
	profile_container.visibility_changed.connect(_on_profile_container_visibility_changed)

	# Connect to NotificationsManager queue signals
	NotificationsManager.notification_queued.connect(_on_notification_queued)

	# Connect to notification clicks to handle friend request notifications
	Global.notification_clicked.connect(_on_notification_clicked)

	# Connect on open emotes backpack
	Global.open_backpack.connect(_on_backpack_emote_opened)

	# Connect deep link router signals for path-based actions
	Global.deep_link_router.deep_link_jump.connect(_on_deep_link_jump)
	Global.deep_link_router.deep_link_open_event.connect(_on_deep_link_open_event)
	Global.deep_link_router.deep_link_open_place.connect(_on_deep_link_open_place)

	# Connect to loading state signals
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)

	Global.orientation_changed.connect(_on_orientation_changed)
	Global.chat_write_mode_changed.connect(_on_chat_write_mode_changed)

	player = load("res://src/logic/player/player.tscn").instantiate()

	player.set_name("Player")
	world.add_child(player)

	timer_broadcast_position.player_node = player
	if Global.is_xr():
		player.vr_screen.set_instantiate_scene(ui_root)

	emote_wheel.avatar_node = player.avatar

	loading_ui.enable_loading_screen()
	var cmd_params = get_params_from_cmd()
	var cmd_realm = Global.FORCE_TEST_REALM if Global.FORCE_TEST else cmd_params[0]
	var cmd_location = cmd_params[1]
	if Global.FORCE_TEST and cmd_location == null:
		cmd_location = Global.FORCE_TEST_LOCATION
	# LOADING_START metric
	var loading_data = {
		"position": str(cmd_location), "realm": str(cmd_realm), "when": "on_explorer_ready"
	}
	Global.metrics.track_screen_viewed("LOADING_START", JSON.stringify(loading_data))

	# --spawn-avatars
	if Global.cli.spawn_avatars:
		var test_spawn_and_move_avatars = TestSpawnAndMoveAvatars.new()
		add_child(test_spawn_and_move_avatars)

	# --debug-panel flag acts like enabling from settings
	if Global.cli.debug_panel:
		_debug_panel_from_settings = true

	# Show debug panel and reload button if in preview mode or --debug-panel
	_update_debug_ui()

	# Preview-only scene-stats / limits overlay (never created in production)
	_update_scene_stats_ui()

	# livekit_debug deep link parameter auto-enables the LiveKit debug panel
	if Global.deep_link_obj.livekit_debug:
		_on_control_menu_request_livekit_debug(true)

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

	if Global.is_xr():
		mobile_ui.hide()
		label_crosshair.hide()
	elif Global.is_mobile():
		mobile_ui.show()
		label_crosshair.show()
		reset_cursor_position()
		_update_virtual_controls_visibility()
	else:
		mobile_ui.hide()

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
func _push_scene_interactable_area() -> void:
	if not is_instance_valid(Global.scene_runner) or not is_instance_valid(ui_safe_area):
		return
	var canvas: Vector2 = ui_safe_area.size
	var canvas_w: int = int(canvas.x)
	var canvas_h: int = int(canvas.y)
	if canvas_w <= 0 or canvas_h <= 0:
		return

	var rect := Rect2i(0, 0, canvas_w, canvas_h)

	if Global.is_mobile() or Global.is_emulating_safe_area():
		var window_size: Vector2i = DisplayServer.window_get_size()
		if window_size.x > 0 and window_size.y > 0:
			var safe: Rect2i = Global.get_safe_area()
			var x_factor: float = canvas.x / float(window_size.x)
			var y_factor: float = canvas.y / float(window_size.y)

			var pos_x: int = clampi(roundi(safe.position.x * x_factor), 0, canvas_w)
			var pos_y: int = clampi(roundi(safe.position.y * y_factor), 0, canvas_h)
			var end_x: int = clampi(roundi(safe.end.x * x_factor), pos_x, canvas_w)
			var end_y: int = clampi(roundi(safe.end.y * y_factor), pos_y, canvas_h)
			rect = Rect2i(pos_x, pos_y, end_x - pos_x, end_y - pos_y)

	Global.scene_runner.set_interactable_area(rect)


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
	if is_instance_valid(debug_panel):
		debug_panel.on_console_add(title, level, timestamp, text)


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


func _is_coordinate_string(text: String) -> bool:
	var cleaned = text.strip_edges().replace("(", "").replace(")", "").replace(" ", "")
	var parts = cleaned.split(",")
	if parts.size() < 2:
		return false
	return _int_regex.search(parts[0]) != null and _int_regex.search(parts[1]) != null


func _parse_coordinates(coord_string: String) -> Vector2i:
	# Remove parentheses if present
	var cleaned = coord_string.strip_edges()
	cleaned = cleaned.replace("(", "").replace(")", "")

	# Remove all spaces
	cleaned = cleaned.replace(" ", "")

	# Split by comma
	var parts = cleaned.split(",")
	if parts.size() >= 2:
		var x_str = parts[0].strip_edges()
		var y_str = parts[1].strip_edges()

		# Validate and parse integers (including negative values)
		if _int_regex.search(x_str) != null and _int_regex.search(y_str) != null:
			return Vector2i(int(x_str), int(y_str))

	return Vector2i(0, 0)


func _on_panel_chat_submit_message(message: String):
	if message.length() == 0:
		return

	var params := message.split(" ")
	var command_str := params[0].to_lower()
	if command_str.begins_with("/"):
		if (command_str == "/go" or command_str == "/goto") and params.size() > 1:
			var arg_string = " ".join(params.slice(1)).strip_edges()
			if _is_coordinate_string(arg_string):
				var dest_vector = _parse_coordinates(arg_string)
				Global.on_chat_message.emit(
					"system",
					"[color=#ccc]🟢 Teleported to " + str(dest_vector) + "[/color]",
					Time.get_unix_time_from_system()
				)
				_on_control_menu_jump_to(dest_vector)
			elif Realm.is_dcl_ens(arg_string) or not arg_string.contains("."):
				var world_realm = (
					arg_string if arg_string.ends_with(".dcl.eth") else arg_string + ".dcl.eth"
				)
				Global.async_join_world(world_realm)
			else:
				_async_try_change_realm(arg_string, "on_goto_realm")
		elif command_str == "/changerealm" and params.size() > 1:
			var target_realm = params[1]
			if Realm.is_dcl_ens(target_realm):
				Global.async_join_world(target_realm)
			else:
				_async_try_change_realm(target_realm, "on_changerealm")

		elif command_str == "/pos":
			_emit_pos_command_message()
		elif command_str == "/clear":
			Global.realm.async_clear_realm()
		elif command_str == "/reload":
			Global.realm.async_set_realm(Global.realm.get_realm_string())
		elif command_str == "/scenecrash":
			Global.scene_runner.debug_force_crash_current_scene()
		elif command_str == "/godotcrash":
			OS.crash("User crashed on purpose")
		elif command_str == "/instantcrash":
			DclCrashGenerator.static_crash()
		elif command_str == "/delayedcrash":
			add_child(DclCrashGenerator.new())
		else:
			Global.on_chat_message.emit(
				"system", "[color=#ccc]🔴 Unknown command[/color]", Time.get_unix_time_from_system()
			)
	else:
		Global.comms.send_chat(message)
		Global.on_chat_message.emit(
			Global.player_identity.get_address_str(), message, Time.get_unix_time_from_system()
		)


func _emit_pos_command_message() -> void:
	# Coordinates: Decentraland uses X right, Y up, Z forward (north). Godot uses X right, Y up, Z backward.
	# So DCL position = (godot.x, godot.y, -godot.z). Parcels are 16m; parcel = (floor(x/16), floor(z/16)).
	var cam = get_viewport().get_camera_3d()
	if not cam:
		Global.on_chat_message.emit(
			"system", "[color=#ccc]🔴 No active camera[/color]", Time.get_unix_time_from_system()
		)
		return

	var pos_godot_player := player.global_position
	var pos_dcl_player := Vector3(pos_godot_player.x, pos_godot_player.y, -pos_godot_player.z)
	var parcel_player := Vector2i(floori(pos_dcl_player.x / 16.0), floori(pos_dcl_player.z / 16.0))

	var pos_godot_cam: Vector3 = cam.global_position
	var pos_dcl_cam := Vector3(pos_godot_cam.x, pos_godot_cam.y, -pos_godot_cam.z)
	var parcel_cam := Vector2i(floori(pos_dcl_cam.x / 16.0), floori(pos_dcl_cam.z / 16.0))

	# Relative to current parcel (origin at parcel corner, 0-16 m on XZ)
	var rel_parcel_player := Vector3(
		pos_dcl_player.x - parcel_player.x * 16.0,
		pos_dcl_player.y,
		pos_dcl_player.z - parcel_player.y * 16.0
	)
	var rel_parcel_cam := Vector3(
		pos_dcl_cam.x - parcel_cam.x * 16.0, pos_dcl_cam.y, pos_dcl_cam.z - parcel_cam.y * 16.0
	)

	# Relative to current scene base parcel
	var current_scene_id: int = Global.scene_runner.get_current_parcel_scene_id()
	var base_parcel: Vector2i = Global.scene_runner.get_scene_base_parcel(current_scene_id)
	var rel_base_player := Vector3(
		pos_dcl_player.x - base_parcel.x * 16.0,
		pos_dcl_player.y,
		pos_dcl_player.z - base_parcel.y * 16.0
	)
	var rel_base_cam := Vector3(
		pos_dcl_cam.x - base_parcel.x * 16.0, pos_dcl_cam.y, pos_dcl_cam.z - base_parcel.y * 16.0
	)

	# Camera forward in Godot is -basis.z; convert to DCL axis (Z_dcl = -Z_godot)
	var forward_godot: Vector3 = -cam.global_transform.basis.z
	var forward_dcl := Vector3(forward_godot.x, forward_godot.y, -forward_godot.z)
	if forward_dcl.length_squared() > 0.0001:
		forward_dcl = forward_dcl.normalized()

	# Realm: display name and type (main / world / preview)
	var realm_display: String = Global.realm.get_realm_string()
	if realm_display.is_empty():
		realm_display = Global.realm.realm_url
	var realm_type: String
	if Realm.is_genesis_city(Global.realm.realm_url):
		realm_type = "main"
	elif Realm.is_dcl_ens(realm_display) or realm_display.ends_with(".dcl.eth"):
		realm_type = "world"
	elif Realm.is_local_preview(Global.realm.realm_url):
		realm_type = "preview"
	else:
		realm_type = "realm"

	var msg := (
		(
			"[color=#cfc][b]Position (DCL)[/b][/color]\n"
			+ "Realm: %s  [%s]\n"
			+ "Player world: (%.2f, %.2f, %.2f)  Parcel: (%d, %d)\n"
			+ "  rel parcel: (%.2f, %.2f, %.2f)  rel base: (%.2f, %.2f, %.2f)\n"
			+ "Camera world: (%.2f, %.2f, %.2f)  Parcel: (%d, %d)\n"
			+ "  rel parcel: (%.2f, %.2f, %.2f)  rel base: (%.2f, %.2f, %.2f)\n"
			+ "Camera dir (unit): (%.4f, %.4f, %.4f)"
		)
		% [
			realm_display,
			realm_type,
			pos_dcl_player.x,
			pos_dcl_player.y,
			pos_dcl_player.z,
			parcel_player.x,
			parcel_player.y,
			rel_parcel_player.x,
			rel_parcel_player.y,
			rel_parcel_player.z,
			rel_base_player.x,
			rel_base_player.y,
			rel_base_player.z,
			pos_dcl_cam.x,
			pos_dcl_cam.y,
			pos_dcl_cam.z,
			parcel_cam.x,
			parcel_cam.y,
			rel_parcel_cam.x,
			rel_parcel_cam.y,
			rel_parcel_cam.z,
			rel_base_cam.x,
			rel_base_cam.y,
			rel_base_cam.z,
			forward_dcl.x,
			forward_dcl.y,
			forward_dcl.z
		]
	)
	Global.on_chat_message.emit("system", msg, Time.get_unix_time_from_system())


func _on_control_menu_request_livekit_debug(enabled):
	Global.comms.set_livekit_debug(enabled)
	if enabled:
		if not is_instance_valid(livekit_debug_panel):
			livekit_debug_panel = (
				load("res://src/ui/components/organisms/livekit_debug/livekit_debug_panel.tscn")
				. instantiate()
			)
			ui_root.add_child(livekit_debug_panel)
	else:
		if is_instance_valid(livekit_debug_panel):
			ui_root.remove_child(livekit_debug_panel)
			livekit_debug_panel.queue_free()
			livekit_debug_panel = null


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
	var cur_parcel_position = Vector2i(
		floor(player.position.x * 0.0625), -floor(player.position.z * 0.0625)
	)
	if not skip_loading:
		if not Global.scene_fetcher.is_scene_loaded(cur_parcel_position.x, cur_parcel_position.y):
			loading_ui.enable_loading_screen()
			# LOADING_START metric
			var loading_data = {
				"position": str(position),
				"realm": Global.realm.get_realm_string(),
				"when": "on_moveto"
			}
			Global.metrics.track_screen_viewed("LOADING_START", JSON.stringify(loading_data))


func _async_try_change_realm(realm_string: String, when: String) -> void:
	Global.on_chat_message.emit(
		"system",
		"[color=#ccc]Trying to change to realm " + realm_string + "[/color]",
		Time.get_unix_time_from_system()
	)
	var success = await Global.realm.async_set_realm(realm_string, true)
	if success:
		loading_ui.enable_loading_screen()
		var loading_data = {
			"position": str(Global.scene_fetcher.current_position),
			"realm": realm_string,
			"when": when
		}
		Global.metrics.track_screen_viewed("LOADING_START", JSON.stringify(loading_data))


func teleport_to(parcel: Vector2i, realm: String = ""):
	_async_teleport_to(parcel, realm)


func _async_teleport_to(parcel: Vector2i, realm: String = "") -> void:
	if not realm.is_empty() and realm != Global.realm.get_realm_string():
		var success = await Global.realm.async_set_realm(realm)
		if not success:
			return
		loading_ui.enable_loading_screen()

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
		margin_container_show_ui.hide()


func _is_ui_hud_mode_exception(node: Node) -> bool:
	return (
		node == ui_safe_area
		or node == control_menu
		or node == margin_container_show_ui
		or node == profile_container
		or node == left_right_safe_container_mobile
		or node == mobile_camera_input
	)


func _apply_mobile_controls_hide_ui(hidden: bool) -> void:
	if not Global.is_mobile():
		return
	_mobile_controls_hidden_for_hide_ui = hidden
	if hidden:
		joypad.hide()
		virtual_joystick.modulate.a = 0.0
	else:
		joypad.show()
		virtual_joystick.modulate.a = 1.0


func _show_joypad() -> void:
	if _mobile_controls_hidden_for_hide_ui:
		return
	joypad.show()


func _set_explorer_hud_elements_visible(full_hud: bool) -> void:
	ui_root.show()
	_apply_mobile_controls_hide_ui(not full_hud)
	if full_hud:
		for node in _ui_children_hidden_for_hud_mode:
			if is_instance_valid(node):
				node.show()
		_ui_children_hidden_for_hud_mode.clear()
		margin_container_show_ui.hide()
		return

	for child in ui_root.get_children():
		if _is_ui_hud_mode_exception(child):
			continue
		if not child is CanvasItem:
			continue
		var canvas_child := child as CanvasItem
		if canvas_child.visible:
			_ui_children_hidden_for_hud_mode.append(canvas_child)
			canvas_child.hide()

	margin_container_show_ui.show()


func _on_control_menu_request_debug_panel(enabled):
	_debug_panel_from_settings = enabled
	_update_debug_ui()


func _update_debug_ui():
	var should_show = _debug_panel_from_settings or _is_in_preview_realm()

	if should_show:
		if not is_instance_valid(debug_panel):
			debug_panel = (
				load("res://src/ui/components/organisms/debug_panel/debug_panel.tscn").instantiate()
			)
			safe_margin_container_debug.add_child(debug_panel)
	else:
		if is_instance_valid(debug_panel):
			safe_margin_container_debug.remove_child(debug_panel)
			debug_panel.queue_free()
			debug_panel = null

	if is_instance_valid(debug_panel):
		debug_panel.set_reload_scene_visible(should_show)

	Global.set_scene_log_enabled(should_show)


## Scene-stats overlay. Instantiated in preview, or in any realm when the
## `scene-stats=true` deep link forces it on, but NEVER in production — so a
## normal production run still pays zero cost, mirroring _update_debug_ui.
func _update_scene_stats_ui() -> void:
	var enabled := _is_in_preview_realm() or Global.deep_link_obj.scene_stats
	var should_show := enabled and not DclGlobal.is_production()
	if should_show:
		if not is_instance_valid(scene_stats_panel):
			scene_stats_panel = (
				load("res://src/ui/components/organisms/scene_stats_panel/scene_stats_panel.tscn")
				. instantiate()
			)
			# Host inside the safe-area HUD layer so it respects device safe
			# insets (notch) like the other top-right HUD elements.
			var host: Node = ui_root.get_node_or_null("LeftRightSafeContainer/InteractableHUD")
			if host == null:
				host = ui_root
			host.add_child(scene_stats_panel)
		scene_stats_panel.set_scene(_preview_scene_id())
	else:
		if is_instance_valid(scene_stats_panel):
			scene_stats_panel.queue_free()
			scene_stats_panel = null


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
	_update_debug_ui()
	_update_scene_stats_ui()


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


func _open_friends_panel() -> void:
	Global.close_menu.emit()
	Global.open_friends_panel.emit()


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
	h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_STOP
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
	h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_STOP
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
	h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_STOP
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
		emote_wheel.hide()
		navbar.hide()
		_set_scene_ui_visible(false)
	else:
		if Global.is_mobile():
			mobile_ui.show()
			_update_virtual_controls_visibility()
		emote_wheel.show()
		navbar._on_size_changed()
		_set_scene_ui_visible(_should_show_scene_ui())


func _on_chat_write_mode_changed(is_writing: bool) -> void:
	if Global.is_orientation_portrait():
		return
	if is_writing:
		mobile_ui.hide()
		emote_wheel.hide()
		navbar.hide()
		_set_scene_ui_visible(false)
	else:
		if Global.is_mobile():
			mobile_ui.show()
			_update_virtual_controls_visibility()
		emote_wheel.show()
		navbar._on_size_changed()
		_set_scene_ui_visible(_should_show_scene_ui())


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
			h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_STOP
			# Release focus to prevent camera rotation while panel is open
			Global.explorer_release_focus()
			if Global.is_mobile():
				release_mouse()
			joypad.hide()


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
	virtual_joystick.show()


func _on_emote_wheel_emote_wheel_opened() -> void:
	virtual_joystick.hide()


func _update_virtual_controls_visibility() -> void:
	if _mobile_controls_hidden_for_hide_ui:
		joypad.hide()
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


func _on_backpack_emote_opened(on_emotes := false) -> void:
	if not on_emotes:
		return
	navbar.open_navbar_silently()
	navbar.set_button_pressed(navbar.BUTTON.BACKPACK)


func _close_all_panels():
	control_menu.async_close()
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	_on_settings_panel_closed()
	h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_show_joypad()


func _on_discover_open():
	navbar.collapse()
	_show_joypad()
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	_on_settings_panel_closed()
	h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_IGNORE
	navbar.set_manually_hidden(true)
	release_mouse()


func _on_menu_open():
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	_on_settings_panel_closed()
	h_box_container_right_panels.mouse_filter = Control.MOUSE_FILTER_IGNORE
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


func _on_h_box_container_right_panels_gui_input(event: InputEvent) -> void:
	if (event is InputEventMouseButton or event is InputEventScreenTouch) and event.pressed:
		_close_all_panels()
		navbar.collapse()
		capture_mouse()


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
