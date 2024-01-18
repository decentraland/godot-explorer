extends Node

var sign_in_resource = preload("res://src/ui/components/auth/sign_in.tscn")

var parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0
var dirty_save_position: bool = false

var last_position_sent: Vector3 = Vector3.ZERO
var counter: int = 0

var debug_panel = null

var virtual_joystick_orig_position: Vector2i

var last_index_scene_ui_root: int = -1
var _last_parcel_position: Vector2i

@onready var ui_root: Control = $UI

@onready var label_crosshair = $UI/Label_Crosshair
@onready var control_pointer_tooltip = $UI/Control_PointerTooltip

@onready var panel_chat = $UI/SafeMarginContainer/InteractableHUD/Panel_Chat

@onready var label_fps = %Label_FPS
@onready var label_ram = %Label_RAM
@onready var control_menu = $UI/Control_Menu
@onready var control_minimap = $UI/Control_Minimap
@onready var player := $world/Player
@onready var mobile_ui = $UI/SafeMarginContainer/InteractableHUD/MobileUI
@onready
var virtual_joystick: Control = $UI/SafeMarginContainer/InteractableHUD/MobileUI/JoystickMoveArea/VirtualJoystick_Left
@onready
var joystick_move_area: Control = $UI/SafeMarginContainer/InteractableHUD/MobileUI/JoystickMoveArea


func _process(_dt):
	parcel_position_real = Vector2(player.position.x * 0.0625, -player.position.z * 0.0625)
	control_minimap.set_center_position(parcel_position_real)

	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	if _last_parcel_position != parcel_position:
		Global.scene_fetcher.update_position(parcel_position)
		_last_parcel_position = parcel_position
		Global.config.last_parcel_position = parcel_position
		dirty_save_position = true


func _on_parcels_procesed(parcels, empty):
	control_minimap.control_map_shader.set_used_parcels(parcels, empty)
	control_menu.control_map.control_map_shader.set_used_parcels(parcels, empty)


# TODO: this can be a command line parser and get some helpers like get_string("--realm"), etc
func get_params_from_cmd():
	var args := OS.get_cmdline_args()
	var realm_string = null
	var location_vector = null
	var realm_in_place := args.find("--realm")
	var location_in_place := args.find("--location")

	if realm_in_place != -1 and args.size() > realm_in_place + 1:
		realm_string = args[realm_in_place + 1]

	if location_in_place != -1 and args.size() > location_in_place + 1:
		location_vector = args[location_in_place + 1]
		location_vector = location_vector.split(",")
		if location_vector.size() == 2:
			location_vector = Vector2i(int(location_vector[0]), int(location_vector[1]))
		else:
			location_vector = null
	return [realm_string, location_vector]


func _ready():
	var cmd_params = get_params_from_cmd()
	var cmd_realm = Global.FORCE_TEST_REALM if Global.FORCE_TEST else cmd_params[0]
	var cmd_location = cmd_params[1]

	virtual_joystick.mouse_filter = Control.MOUSE_FILTER_IGNORE
	virtual_joystick_orig_position = virtual_joystick.get_position()
	panel_chat.hide()

	if Global.is_mobile:
		mobile_ui.show()
		label_crosshair.show()
	else:
		mobile_ui.hide()

	var sky = null
	if Global.is_mobile:
		sky = load("res://assets/sky/sky_basic_without_glow.tscn").instantiate()
		add_child(sky)
	elif Global.testing_scene_mode:
		sky = load("res://assets/sky/sky_test.tscn").instantiate()
		add_child(sky)
	else:
		match Global.config.skybox:
			0:
				sky = load("res://assets/sky/sky_basic.tscn").instantiate()
			1:
				sky = load("res://assets/sky/krzmig/world_environment.tscn").instantiate()
				sky.day_time = 14.9859

		add_child(sky)
		if Global.config.skybox == 1:
			sky.day_time = 10

	control_pointer_tooltip.hide()
	var start_parcel_position: Vector2i = Vector2i(Global.config.last_parcel_position)
	if cmd_location != null:
		start_parcel_position = cmd_location

	player.position = 16 * Vector3(start_parcel_position.x, 0.1, -start_parcel_position.y)
	player.look_at(16 * Vector3(start_parcel_position.x + 1, 0, -(start_parcel_position.y + 1)))

	Global.scene_runner.camera_node = player.camera
	Global.scene_runner.player_node = player
	Global.scene_runner.console = self._on_scene_console_message
	Global.scene_runner.pointer_tooltip_changed.connect(self._on_pointer_tooltip_changed)
	ui_root.add_child(Global.scene_runner.base_ui)
	ui_root.move_child(Global.scene_runner.base_ui, label_crosshair.get_index() + 1)

	Global.scene_fetcher.connect("parcels_processed", self._on_parcels_procesed)

	if cmd_realm != null:
		Global.realm.async_set_realm(cmd_realm)
	else:
		if Global.config.last_realm_joined.is_empty():
			Global.realm.async_set_realm(
				"https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main"
			)
		else:
			Global.realm.async_set_realm(Global.config.last_realm_joined)

	Global.scene_runner.process_mode = Node.PROCESS_MODE_INHERIT

	control_menu.preview_hot_reload.connect(self._on_panel_bottom_left_preview_hot_reload)

	Global.player_identity.logout.connect(self._on_player_logout)
	Global.player_identity.profile_changed.connect(Global.avatars.update_primary_player_profile)
	Global.player_identity.need_open_url.connect(self._on_need_open_url)

	if Global.testing_scene_mode:
		Global.player_identity.create_guest_account()
	elif not Global.player_identity.try_recover_account(Global.config.session_account):
		Global.scene_runner.set_pause(true)
		ui_root.add_child(sign_in_resource.instantiate())

	# last
	ui_root.grab_focus.call_deferred()


func _on_need_open_url(url: String, _description: String) -> void:
	if not Global.player_identity.get_address_str().is_empty():
		if Global.dcl_android_plugin != null:
			Global.dcl_android_plugin.showDecentralandMobileToast()
			Global.dcl_android_plugin.openUrl(url)
		else:
			OS.shell_open(url)


func _on_player_logout():
	# TODO: clean all UI ?
	control_menu.close()

	# Clean stored session
	Global.config.session_account = {}
	Global.config.save_to_settings_file()

	ui_root.add_child(sign_in_resource.instantiate())


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
	if not Global.scene_runner.pointer_tooltips.is_empty():
		control_pointer_tooltip.set_pointer_data(Global.scene_runner.pointer_tooltips)
		control_pointer_tooltip.show()
	else:
		control_pointer_tooltip.hide()


func _on_check_button_toggled(button_pressed):
	Global.scene_runner.set_pause(button_pressed)


func _unhandled_input(event):
	if not Global.is_mobile:
		if event is InputEventMouseButton and event.pressed:
			if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
				capture_mouse()

		if event is InputEventKey and ui_root.has_focus():
			if event.pressed and event.keycode == KEY_TAB:
				if not control_menu.visible:
					control_menu.show_last()
					release_mouse()

			if event.pressed and event.keycode == KEY_M:
				if control_menu.visible:
					pass
				else:
					control_menu.show_map()
					release_mouse()

			if event.pressed and event.keycode == KEY_ESCAPE:
				if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
					release_mouse()

			if event.pressed and event.keycode == KEY_ENTER:
				panel_chat.show()


func _toggle_ram_usage(visibility: bool):
	if visibility:
		label_ram.show()
	else:
		label_ram.hide()


func _on_control_minimap_request_open_map():
	if !control_menu.visible:
		control_menu.show_map()
		release_mouse()


func _on_control_menu_jump_to(parcel: Vector2i):
	player.set_position(Vector3i(parcel.x * 16, 3, -parcel.y * 16))
	control_menu.close()


func _on_control_menu_hide_menu():
	control_menu.close()
	control_menu.control_map.clear()
	ui_root.grab_focus()


func _on_control_menu_toggle_ram(visibility):
	label_ram.visible = visibility


func _on_control_menu_toggle_fps(visibility):
	label_fps.visible = visibility


func _on_control_menu_toggle_minimap(visibility):
	control_minimap.visible = visibility


func _on_panel_bottom_left_preview_hot_reload(_scene_type, scene_id):
	Global.scene_fetcher.reload_scene(scene_id)


func _on_timer_broadcast_position_timeout():
	var transform: Transform3D = player.avatar.global_transform
	var position = transform.origin
	var rotation = transform.basis.get_rotation_quaternion()

	if last_position_sent.is_equal_approx(position):
		counter += 1
		if counter < 10:
			return

	Global.comms.broadcast_position_and_rotation(position, rotation)
	last_position_sent = position
	counter = 0


func _on_virtual_joystick_right_stick_position(stick_position: Vector2):
	player.stick_position = stick_position


func _on_virtual_joystick_right_is_hold(hold: bool):
	player.stick_holded = hold


func _on_touch_screen_button_pressed():
	Input.action_press("ia_jump")


func _on_touch_screen_button_released():
	Input.action_release("ia_jump")


func _on_panel_chat_submit_message(message: String):
	if message.length() == 0:
		return

	var params := message.split(" ")
	var command_str := params[0].to_lower()
	if command_str.begins_with("/"):
		if command_str == "/go" or command_str == "/goto" and params.size() > 1:
			var comma_params = params[1].split(",")
			var dest_vector = Vector2i(0, 0)
			if comma_params.size() > 1:
				dest_vector = Vector2i(int(comma_params[0]), int(comma_params[1]))
			elif params.size() > 2:
				dest_vector = Vector2i(int(params[1]), int(params[2]))

			panel_chat.add_chat_message(
				"[color=#ccc]> Teleport to " + str(dest_vector) + "[/color]"
			)
			_on_control_menu_jump_to(dest_vector)
		elif command_str == "/changerealm" and params.size() > 1:
			panel_chat.add_chat_message(
				"[color=#ccc]> Trying to change to realm " + params[1] + "[/color]"
			)
			Global.realm.async_set_realm(params[1])
		elif command_str == "/reload":
			Global.realm.async_set_realm(Global.realm.get_realm_string())
		else:
			pass
			# TODO: unknown command
	else:
		Global.comms.send_chat(message)
		panel_chat.on_chats_arrived(
			[[player.avatar.avatar_id, player.avatar.avatar_name, 0, message]]
		)


func _on_control_menu_request_pause_scenes(enabled):
	Global.scene_runner.set_pause(enabled)


func move_to(position: Vector3):
	player.set_position(position)


func teleport_to(parcel: Vector2i):
	player.set_position(Vector3i(parcel.x * 16, 3, -parcel.y * 16))


func player_look_at(look_at_position: Vector3):
	player.avatar_look_at(look_at_position)


func capture_mouse():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	label_crosshair.show()
	ui_root.grab_focus.call_deferred()


func release_mouse():
	Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	label_crosshair.hide()


func set_visible_ui(value: bool):
	if value == ui_root.visible:
		return

	if value:
		ui_root.show()
		var ui_node = ui_root.get_parent().get_node("scenes_ui")
		ui_node.reparent(ui_root)
	else:
		ui_root.hide()
		var ui_node = ui_root.get_node("scenes_ui")
		ui_node.reparent(ui_root.get_parent())


func _on_control_menu_request_debug_panel(enabled):
	if enabled:
		if not is_instance_valid(debug_panel):
			debug_panel = load("res://src/ui/components/debug_panel/debug_panel.tscn").instantiate()
			ui_root.add_child(debug_panel)
			ui_root.move_child(debug_panel, control_menu.get_index() - 1)
	else:
		if is_instance_valid(debug_panel):
			ui_root.remove_child(debug_panel)
			debug_panel.queue_free()
			debug_panel = null

	Global.set_scene_log_enabled(enabled)


func _on_timer_fps_label_timeout():
	label_ram.set_text("RAM Usage: " + str(OS.get_static_memory_usage() / 1024.0 / 1024.0) + " MB")
	label_fps.set_text("ALPHA - " + str(Engine.get_frames_per_second()) + " FPS")
	if dirty_save_position:
		dirty_save_position = false
		Global.config.save_to_settings_file()


func _on_mini_map_pressed():
	control_menu.show_map()
	release_mouse()


func _on_profile_pressed():
	control_menu.show_backpack()
	release_mouse()


func _on_button_jump_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			Input.action_press("ia_jump")
		else:
			Input.action_release("ia_jump")


func _on_joystick_move_area_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			virtual_joystick.set_position(event.position - virtual_joystick.get_size() / 2)

			# Convert to global position and forward the event
			virtual_joystick.mouse_filter = Control.MOUSE_FILTER_PASS
			event.position = joystick_move_area.get_global_rect().position + event.position
			virtual_joystick.custom_input(event)


func _on_virtual_joystick_left_is_holded(holded: bool):
	if not holded:
		virtual_joystick.set_position(virtual_joystick_orig_position)
		virtual_joystick.mouse_filter = Control.MOUSE_FILTER_IGNORE


func _on_button_open_chat_pressed():
	panel_chat.visible = not panel_chat.visible
