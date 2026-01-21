class_name Explorer
extends Node

var is_genesis_city: bool
var player: Node3D = null
var scene_title: String
var parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0
var dirty_save_position: bool = false

var debug_panel = null
var disable_move_to = false

var virtual_joystick_orig_position: Vector2i

var _first_time_refresh_warning = true

var _last_parcel_position: Vector2i = Vector2i.MAX
var _avatar_under_crosshair: Avatar = null
var _last_outlined_avatar: Avatar = null
var _is_loading: bool = true  # Start as loading
var _pending_notification_toast: Dictionary = {}  # Store notification waiting to be shown

@onready var ui_root: Control = %UI
@onready var ui_safe_area: Control = %SceneUIContainer
@onready var safe_margin_container_debug: SafeMarginContainer = %SafeMarginContainerDebug

@onready var warning_messages = %WarningMessages
@onready var label_crosshair = %Label_Crosshair
@onready var control_pointer_tooltip = %Control_PointerTooltip

@onready var panel_chat = %Panel_Chat
@onready var button_load_scenes: Button = %Button_LoadScenes
@onready var url_popup = %UrlPopup
@onready var jump_in_popup = %JumpInPopup

@onready var notifications_panel: PanelContainer = %NotificationsPanel
@onready var friends_panel: PanelContainer = %FriendsPanel
@onready var label_version = %Label_Version
@onready var label_fps = %Label_FPS
@onready var label_ram = %Label_RAM
@onready var control_menu = %Control_Menu
@onready var mobile_ui = %MobileUI
@onready var virtual_joystick: Control = %VirtualJoystick_Left
@onready var profile_container: Control = %ProfileContainer

@onready var loading_ui = %Loading

@onready var emote_wheel = %EmoteWheel

@onready var world: Node3D = %world

@onready var timer_broadcast_position: Timer = %Timer_BroadcastPosition
@onready var h_box_container_top_left_menu: HBoxContainer = %HBoxContainer_TopLeftMenu
@onready var control_safe_bottom_area: Control = %Control_SafeBottomArea
@onready var margin_container_chat_panel: MarginContainer = %MarginContainer_ChatPanel
@onready var v_box_container_left_side: VBoxContainer = %VBoxContainer_LeftSide
@onready var notifications: Control = %Notifications

@onready var virtual_keyboard_margin: Control = %VirtualKeyboardMargin

@onready var chat_container: Control = %ChatContainer
@onready var safe_margin_container_hud: SafeMarginContainer = %SafeMarginContainerHUD

@onready var navbar: Control = %Navbar
@onready var joypad: Control = %Joypad
@onready var chatbar: Control = %Chatbar


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
			realm_string = Realm.MAIN_REALM

	return [realm_string, location_vector]


func _ready():
	GraphicSettings.apply_full_processor_mode()

	Global.scene_runner.on_change_scene_id.connect(_on_change_scene_id)
	Global.change_parcel.connect(_on_change_parcel)

	label_version.set_text("v" + DclGlobal.get_version())
	Global.change_virtual_keyboard.connect(self._on_change_virtual_keyboard)
	Global.set_orientation_landscape()
	UiSounds.install_audio_recusirve(self)
	Global.music_player.stop()

	# Register popup instances in Global
	Global.set_url_popup_instance(url_popup)
	Global.set_jump_in_popup_instance(jump_in_popup)

	# Connect notification bell button
	Global.open_notifications_panel.connect(_show_notifications_panel)
	Global.open_chat.connect(_on_global_open_chat)
	Global.open_discover.connect(_on_discover_open)
	Global.on_menu_close.connect(_on_menu_close)

	# Connect friends button
	Global.open_friends_panel.connect(_show_friends_panel)

	navbar.close_all.connect(_close_all_panels)
	navbar.navbar_opened.connect(_open_friends_panel)
	chatbar.share_place.connect(_share_place)

	# Connect to NotificationsManager queue signals
	NotificationsManager.notification_queued.connect(_on_notification_queued)

	# Connect to notification clicks to handle friend request notifications
	Global.notification_clicked.connect(_on_notification_clicked)

	# Connect on open emotes backpack
	Global.open_backpack.connect(_on_backpack_emote_opened)

	# Connect to loading state signals
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)

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

	# --debug-panel (automatically enabled with --preview or preview deeplink)
	if Global.cli.debug_panel or not Global.deep_link_obj.preview.is_empty():
		_on_control_menu_request_debug_panel(true)

	virtual_joystick.mouse_filter = Control.MOUSE_FILTER_IGNORE
	virtual_joystick_orig_position = virtual_joystick.get_position()

	if Global.is_xr():
		mobile_ui.hide()
		label_crosshair.hide()
	elif Global.is_mobile():
		mobile_ui.show()
		label_crosshair.show()
		reset_cursor_position()
		ui_root.gui_input.connect(self._on_ui_root_gui_input)
	else:
		mobile_ui.hide()

	chat_container.hide()
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
	ui_safe_area.add_child(Global.scene_runner.base_ui)
	ui_safe_area.move_child(Global.scene_runner.base_ui, 0)

	Global.scene_fetcher.notify_pending_loading_scenes.connect(
		self._on_notify_pending_loading_scenes
	)

	# Add disconnect handler for reconnection logic
	var disconnect_handler = (
		load("res://src/ui/components/disconnect_handler/disconnect_handler.tscn").instantiate()
	)
	add_child(disconnect_handler)

	#Global.scene_fetcher.current_position = start_parcel_position
	Global.scene_fetcher.update_position(start_parcel_position, true)

	if cmd_realm != null:
		Global.realm.async_set_realm(cmd_realm)
		if control_menu.control_settings.instance != null:
			control_menu.control_settings.instance.set_preview_url(cmd_realm)
	else:
		if Global.get_config().last_realm_joined.is_empty():
			Global.realm.async_set_realm(
				"https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"
			)
		else:
			Global.realm.async_set_realm(Global.get_config().last_realm_joined)

	Global.scene_runner.process_mode = Node.PROCESS_MODE_INHERIT

	control_menu.preview_hot_reload.connect(self._on_panel_bottom_left_preview_hot_reload)

	Global.player_identity.logout.connect(self._on_player_logout)
	Global.player_identity.profile_changed.connect(Global.avatars.update_primary_player_profile)
	Global.player_identity.profile_changed.connect(self._on_player_profile_changed)

	# Initialize social service for non-guest accounts
	if not Global.player_identity.is_guest:
		_async_initialize_social_service()

	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		Global.player_identity.profile_changed.emit(profile)

	Global.dcl_tokio_rpc.need_open_url.connect(self._on_need_open_url)
	Global.scene_runner.set_pause(false)

	if Global.testing_scene_mode:
		Global.player_identity.create_guest_account()

	Global.metrics.update_identity(
		Global.player_identity.get_address_str(), Global.player_identity.is_guest
	)

	Global.open_profile_by_address.connect(_async_open_profile_by_address)
	Global.open_profile_by_avatar.connect(_async_open_profile_by_avatar)

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


func _on_need_open_url(url: String, _description: String, _use_webkit: bool) -> void:
	if not Global.player_identity.get_address_str().is_empty():
		Global.open_url(url)


func _on_player_logout():
	# Stop notifications polling
	NotificationsManager.stop_polling()

	# Clean stored session
	Global.get_config().session_account = {}
	Global.get_config().save_to_settings_file()

	# TODO: It's crashing. Logout = exit app
	#get_tree().change_scene_to_file("res://src/main.tscn")

	# TODO: Temporal solution
	get_tree().quit()


func _on_player_profile_changed(_profile: DclUserProfile) -> void:
	# Start notifications polling when authenticated
	print("[Explorer] Player profile changed - starting notifications polling")
	NotificationsManager.start_polling()


func _async_initialize_social_service() -> void:
	# Initialize the social service with player identity
	# Note: Subscriptions are now handled by FriendsPanel when it opens/closes
	Global.social_service.initialize_from_player_identity(Global.player_identity)


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
	_avatar_under_crosshair = player.get_avatar_under_crosshair()
	Global.selected_avatar = _avatar_under_crosshair

	# Handle outline changes through the outline system
	if _avatar_under_crosshair != _last_outlined_avatar:
		player.outline_system.set_outlined_avatar(_avatar_under_crosshair)
		_last_outlined_avatar = _avatar_under_crosshair

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
	control_menu.close()


func _on_control_menu_hide_menu():
	control_menu.close()
	ui_root.grab_focus()


func _on_control_menu_toggle_fps(visibility):
	label_fps.visible = visibility


func _on_panel_bottom_left_preview_hot_reload(_scene_type, scene_id):
	Global.scene_fetcher.reload_scene(scene_id)


func _on_virtual_joystick_right_stick_position(stick_position: Vector2):
	player.stick_position = stick_position


func _on_virtual_joystick_right_is_hold(hold: bool):
	player.stick_holded = hold


func _on_touch_screen_button_pressed():
	Input.action_press("ia_jump")


func _on_touch_screen_button_released():
	Input.action_release("ia_jump")


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
		var int_regex = RegEx.new()
		int_regex.compile(r"^-?\d+$")

		if int_regex.search(x_str) != null and int_regex.search(y_str) != null:
			return Vector2i(int(x_str), int(y_str))

	return Vector2i(0, 0)


func _on_panel_chat_submit_message(message: String):
	if message.length() == 0:
		return

	var params := message.split(" ")
	var command_str := params[0].to_lower()
	if command_str.begins_with("/"):
		if command_str == "/go" or command_str == "/goto" and params.size() > 1:
			# Join all params after the command to handle spaces properly
			var coord_string = ""
			if params.size() > 1:
				coord_string = " ".join(params.slice(1))

			var dest_vector = _parse_coordinates(coord_string)

			Global.on_chat_message.emit(
				"system",
				"[color=#ccc]🟢 Teleported to " + str(dest_vector) + "[/color]",
				Time.get_unix_time_from_system()
			)
			_on_control_menu_jump_to(dest_vector)
		elif command_str == "/changerealm" and params.size() > 1:
			Global.on_chat_message.emit(
				"system",
				"[color=#ccc]Trying to change to realm " + params[1] + "[/color]",
				Time.get_unix_time_from_system()
			)
			Global.realm.async_set_realm(params[1], true)
			loading_ui.enable_loading_screen()
			# LOADING_START metric
			var loading_data = {
				"position": str(Global.scene_fetcher.current_position),
				"realm": params[1],
				"when": "on_changerealm"
			}
			Global.metrics.track_screen_viewed("LOADING_START", JSON.stringify(loading_data))

		elif command_str == "/world" and params.size() > 1:
			var world_realm = params[1] + ".dcl.eth"
			Global.on_chat_message.emit(
				"system",
				"[color=#ccc]Trying to change to world " + world_realm + "[/color]",
				Time.get_unix_time_from_system()
			)
			Global.realm.async_set_realm(world_realm, true)
			loading_ui.enable_loading_screen()
			# LOADING_START metric
			var loading_data = {
				"position": str(Global.scene_fetcher.current_position),
				"realm": world_realm,
				"when": "on_world"
			}
			Global.metrics.track_screen_viewed("LOADING_START", JSON.stringify(loading_data))

		elif command_str == "/clear":
			Global.realm.async_clear_realm()
		elif command_str == "/reload":
			Global.realm.async_set_realm(Global.realm.get_realm_string())
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


func _on_control_menu_request_pause_scenes(enabled):
	Global.scene_runner.set_pause(enabled)


## Moves the player to a specific position
##
## @param position: The 3D position to move the player to
## @param skip_loading: When true, skips showing the loading screen.
##                      This is used when teleporting inside a scene to avoid
##                      showing the loading UI for an already-loaded area.
func move_to(position: Vector3, skip_loading: bool):
	if disable_move_to:
		return

	# Set grace period on avatar's emote controller to prevent emote cancellation during teleport
	if player.avatar and player.avatar.emote_controller:
		player.avatar.emote_controller.set_teleport_grace()

	player.move_to(position)
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


func teleport_to(parcel: Vector2i, realm: String = ""):
	var move_to_position = Vector3i(parcel.x * 16 + 8, 3, -parcel.y * 16 - 8)
	move_to(move_to_position, false)

	if not realm.is_empty() && realm != Global.realm.get_realm_string():
		Global.realm.async_set_realm(realm)

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
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	if label_crosshair and ui_root:
		label_crosshair.show()
		ui_root.grab_focus.call_deferred()


func release_mouse():
	Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	if not Global.is_mobile():
		if label_crosshair:
			label_crosshair.hide()


func set_visible_ui(value: bool):
	if value == ui_root.visible:
		return

	if value:
		ui_root.show()
	else:
		ui_root.hide()


func _on_control_menu_request_debug_panel(enabled):
	if enabled:
		if not is_instance_valid(debug_panel):
			debug_panel = load("res://src/ui/components/debug_panel/debug_panel.tscn").instantiate()
			safe_margin_container_debug.add_child(debug_panel)
	else:
		if is_instance_valid(debug_panel):
			safe_margin_container_debug.remove_child(debug_panel)
			debug_panel.queue_free()
			debug_panel = null

	Global.set_scene_log_enabled(enabled)


func _on_timer_fps_label_timeout():
	var fps_text = "- " + str(Engine.get_frames_per_second()) + " FPS"

	# Add dynamic graphics info if enabled
	if Global.get_config().dynamic_graphics_enabled:
		var dm = Global.dynamic_graphics_manager
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

	label_fps.set_text(fps_text)
	if dirty_save_position:
		dirty_save_position = false
		Global.get_config().save_to_settings_file()


func hide_menu():
	control_menu.close()
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


func _on_ui_root_gui_input(event: InputEvent):
	if event is InputEventScreenTouch:
		if event.pressed:
			set_cursor_position(event.position)
		# On mobile in PointerUnlocked mode (VirtualCamera active), trigger ia_pointer on touch
		if Global.is_mobile() and Global.scene_runner.raycast_use_cursor_position:
			if event.pressed:
				Input.action_press("ia_pointer")
			else:
				Input.action_release("ia_pointer")


func _on_panel_profile_open_profile():
	_open_own_profile()


func _on_button_load_scenes_pressed() -> void:
	Global.scene_fetcher._bypass_loading_check = true
	button_load_scenes.hide()


func _on_notify_pending_loading_scenes(pending: bool) -> void:
	if pending:
		button_load_scenes.show()
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
		button_load_scenes.hide()


func _open_profile(dcl_user_profile: DclUserProfile):
	panel_chat.exit_chat()
	profile_container.open(dcl_user_profile)
	release_mouse()


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
	# Check if it's an Avatar (GDScript class) to access avatar_id
	if avatar is Avatar:
		var avatar_instance = avatar as Avatar
		var avatar_id = avatar_instance.avatar_id
		if not avatar_id.is_empty():
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


func _open_own_profile() -> void:
	control_menu.async_show_own_profile()
	release_mouse()


func _get_viewport_scale_factors() -> Vector2:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size = get_viewport().get_visible_rect().size
	var x_factor: float = viewport_size.x / window_size.x
	var y_factor: float = viewport_size.y / window_size.y
	return Vector2(x_factor, y_factor)


func _on_global_open_chat() -> void:
	# When coming from Global.open_chat, start chat and handle UI
	safe_margin_container_hud.hide()
	chat_container.show()
	panel_chat.async_start_chat()
	release_mouse()


func _on_panel_chat_on_open_chat() -> void:
	# When coming from on_open_chat from panel_chat, only handle the UI
	# DO NOT call async_start_chat() because it's already running (avoids recursion)
	safe_margin_container_hud.hide()
	chat_container.show()
	# Hide navbar when chat opens to prevent it from showing when virtual keyboard appears
	if Global.is_mobile():
		navbar.set_manually_hidden(true)


func _on_panel_chat_on_exit_chat() -> void:
	safe_margin_container_hud.show()
	chat_container.hide()
	if Global.is_mobile():
		mobile_ui.show()
		# Restore navbar visibility when chat closes
		navbar.set_manually_hidden(false)


func _on_change_virtual_keyboard(virtual_keyboard_height: int):
	if virtual_keyboard_height != 0:
		var window_size: Vector2i = DisplayServer.window_get_size()
		var viewport_size = get_viewport().get_visible_rect().size

		var y_factor: float = viewport_size.y / window_size.y
		virtual_keyboard_margin.custom_minimum_size.y = virtual_keyboard_height * y_factor
	elif virtual_keyboard_height == 0:
		panel_chat.exit_chat()


func _show_friends_panel() -> void:
	if friends_panel.visible:
		return
	joypad.hide()
	friends_panel.show_panel_on_friends_tab()
	if notifications_panel.visible:
		notifications_panel.hide_panel()
	Global.explorer_release_focus()
	if Global.is_mobile():
		release_mouse()


func _on_friends_panel_closed() -> void:
	friends_panel.hide_panel()
	Global.explorer_grab_focus()
	capture_mouse()


func _show_notifications_panel() -> void:
	if notifications_panel.visible:
		return
	joypad.hide()
	notifications_panel.show_panel()
	if friends_panel.visible:
		friends_panel.hide_panel()
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
	var toast_scene = load("res://src/ui/components/notifications/notification_toast.tscn")
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
	_pending_notification_toast = {}  # Clear any pending notification


func _on_loading_finished() -> void:
	_is_loading = false
	# Show pending notification if there was one queued during loading
	if not _pending_notification_toast.is_empty():
		_show_notification_toast(_pending_notification_toast)
		_pending_notification_toast = {}


func _on_notification_clicked(notification_d: Dictionary) -> void:
	# Handle friend request notification clicks - open friends panel on friends tab
	var notif_type = notification_d.get("type", "")

	if ["social_service_friendship_request", "social_service_friendship_accepted"].has(notif_type):
		# Open friends panel on friends tab
		if not friends_panel.visible:
			friends_panel.show_panel_on_friends_tab()
			navbar.open_navbar_silently()
			navbar.set_button_pressed(navbar.BUTTON.FRIENDS)
			# Close notifications panel if open
			if notifications_panel.visible:
				notifications_panel.hide_panel()
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

		Global.check_deep_link_teleport_to()


func _on_emote_wheel_emote_wheel_closed() -> void:
	virtual_joystick.show()


func _on_emote_wheel_emote_wheel_opened() -> void:
	virtual_joystick.hide()


func _on_backpack_emote_opened(on_emotes := false) -> void:
	if not on_emotes:
		return
	navbar.open_navbar_silently()
	navbar.set_button_pressed(navbar.BUTTON.BACKPACK)


func _close_all_panels():
	control_menu.close()
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	joypad.show()


func _on_discover_open():
	navbar.close_from_discover_button()
	joypad.show()
	_on_friends_panel_closed()
	_on_notifications_panel_closed()
	navbar.set_manually_hidden(true)
	release_mouse()


func _on_menu_close():
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
		url = (
			"decentraland://open?position="
			+ str(parcel_position[0])
			+ ","
			+ str(parcel_position[1])
		)
	else:
		var realm_url = Global.realm.realm_url
		var short_realm_url = _extract_short_realm_url(realm_url)
		url = "decentraland://open?realm=" + short_realm_url

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
		return
	var scene = Global.scene_fetcher.get_scene_data_by_scene_id(scene_id)
	if scene != null:
		scene_title = scene.scene_entity_definition.get_title()
	else:
		scene_title = ""


func _on_change_parcel(_position: Vector2i):
	parcel_position = _position
