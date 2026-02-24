extends Control

signal open_profile
signal hide_menu
signal jump_to(Vector2i)
signal toggle_minimap
signal toggle_fps
signal toggle_ram
signal request_pause_scenes(enabled: bool)
signal request_debug_panel(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)
#signals from advanced settings

var is_in_game: bool = false  # when it is playing in the 3D Game or not
var is_open: bool = false
var buttons_quantity: int = 0
var pressed_index: int = 0

var selected_node: PlaceholderManager
var current_screen_name: String = ""
var fade_out_tween: Tween = null
var fade_in_tween: Tween = null
var _close_modulate_tween: Tween = null
var _close_hide_tween: Tween = null
var _close_node_to_free: PlaceholderManager = null

@onready var group: ButtonGroup = ButtonGroup.new()

@onready var control_discover := PlaceholderManager.new(%Control_Discover)
@onready var control_settings := PlaceholderManager.new(%Control_Settings)
@onready var control_backpack := PlaceholderManager.new(%Control_Backpack)
@onready var control_profile_settings := PlaceholderManager.new(%Control_ProfileSettings)

@onready var control_deploying_profile := %Control_DeployingProfile

@onready var portrait_button_profile: TextureButton = %Portrait_Button_Profile

@onready var account_deletion_pop_up: TextureRect = $AccountDeletionPopUp

@onready var static_button_backpack: TextureButton = %StaticButton_Backpack
@onready var static_button_discover: TextureButton = %StaticButton_Discover
@onready var static_button_settings: TextureButton = %StaticButton_Settings
@onready var control_modal: Control = %Control_Modal


func _ready():
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	static_button_backpack.button_group = btn_group
	static_button_discover.button_group = btn_group
	static_button_settings.button_group = btn_group
	portrait_button_profile.button_group = btn_group
	Global.open_discover.emit()
	static_button_discover.button_pressed = true

	account_deletion_pop_up.hide()

	is_in_game = self != get_tree().current_scene

	control_deploying_profile.hide()

	self.modulate = Color(1, 1, 1, 1)
	current_screen_name = ("DISCOVER" if Global.is_orientation_portrait() else "DISCOVER_IN_GAME")
	if !is_in_game:
		Global.metrics.track_screen_viewed(current_screen_name, '{"function": "ready"}')
		Global.metrics.flush()

	control_settings.placeholder.visible = false
	control_discover.placeholder.visible = false
	control_backpack.placeholder.visible = false
	control_profile_settings.placeholder.visible = false

	# Connect to notification clicked signal for reward notifications
	Global.notification_clicked.connect(_on_notification_clicked)

	Global.deep_link_received.connect(_on_deep_link_received)
	Global.open_settings.connect(async_show_settings)
	Global.open_backpack.connect(async_show_backpack)
	Global.open_discover.connect(async_show_discover)
	Global.open_own_profile.connect(async_show_own_profile)
	Global.close_menu.connect(close)
	Global.delete_account.connect(_on_account_delete)

	if not is_in_game:
		open.call_deferred()


func open():
	_open()


# gdlint:ignore = async-function-name
func close():
	if not is_open:
		return
	is_open = false
	GraphicSettings.apply_full_processor_mode()
	if Global.player_identity.has_changes():
		Global.player_identity.async_save_profile()
	_close_modulate_tween = create_tween()
	_close_modulate_tween.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3).set_ease(
		Tween.EASE_IN_OUT
	)
	# Capture the node to free NOW, before the tween callback fires.
	# If we reference `selected_node` directly in the lambda, it may have changed
	# by the time the callback runs (e.g. user opened Settings while close tween is running).
	_close_node_to_free = selected_node
	_close_hide_tween = create_tween()
	_close_hide_tween.tween_callback(hide).set_delay(0.3)
	_close_hide_tween.tween_callback(
		func():
			if _close_node_to_free:
				_close_node_to_free.queue_free_instance()
				_close_node_to_free = null
	)


func async_show_discover(open_menu := true):
	await control_discover._async_instantiate()
	select_discover_screen()
	if is_instance_valid(static_button_discover):
		static_button_discover.toggled.emit(true)
	if open_menu:
		_open()


func async_show_backpack(on_emotes := false):
	await control_backpack._async_instantiate()
	select_backpack_screen()
	if on_emotes:
		control_backpack.instance.show_emotes()
		control_backpack.instance.press_button_emotes()
	_open()


func async_show_settings():
	await control_settings._async_instantiate()

	if not is_instance_valid(control_settings.instance):
		return

	control_settings.instance.request_pause_scenes.connect(
		func(enabled): request_pause_scenes.emit(enabled)
	)
	control_settings.instance.request_debug_panel.connect(
		func(enabled): request_debug_panel.emit(enabled)
	)
	control_settings.instance.preview_hot_reload.connect(
		func(scene_type, scene_id): preview_hot_reload.emit(scene_type, scene_id)
	)

	select_settings_screen()
	_open()


func async_show_own_profile():
	await control_profile_settings._async_instantiate()

	select_profile_screen()
	_open()


func _open():
	if is_open:
		return
	# Kill any pending close tweens so the old close() doesn't hide us again.
	# But still free the node that close() intended to free (avoid memory leak).
	if is_instance_valid(_close_modulate_tween) and _close_modulate_tween.is_running():
		_close_modulate_tween.kill()
	if is_instance_valid(_close_hide_tween) and _close_hide_tween.is_running():
		_close_hide_tween.kill()
		# The tween callback won't fire, so free the node manually
		if _close_node_to_free:
			_close_node_to_free.queue_free_instance()
			_close_node_to_free = null
	if selected_node and not selected_node.instance:
		selected_node = null
	if not selected_node:
		async_show_discover(false)
	if not visible:
		show()
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.25).set_ease(Tween.EASE_IN_OUT)
	tween.tween_callback(
		func():
			is_open = true
			GraphicSettings.apply_low_processor_mode()
	)


func _on_control_settings_toggle_fps_visibility(visibility):
	emit_signal("toggle_fps", visibility)


func _on_control_settings_toggle_ram_usage_visibility(visibility):
	emit_signal("toggle_ram", visibility)


func select_settings_screen(play_sfx: bool = true):
	current_screen_name = ("SETTINGS" if Global.is_orientation_portrait() else "SETTINGS_IN_GAME")
	Global.metrics.track_screen_viewed(current_screen_name, "")
	select_node(control_settings, play_sfx)


func select_discover_screen(play_sfx: bool = true):
	current_screen_name = ("DISCOVER" if Global.is_orientation_portrait() else "DISCOVER_IN_GAME")
	Global.metrics.track_screen_viewed(current_screen_name, "")
	select_node(control_discover, play_sfx)


func select_backpack_screen(play_sfx: bool = true):
	current_screen_name = ("BACKPACK" if Global.is_orientation_portrait() else "BACKPACK_IN_GAME")
	Global.metrics.track_screen_viewed(current_screen_name, "")
	select_node(control_backpack, play_sfx)


func select_profile_screen(play_sfx: bool = true):
	current_screen_name = ("PROFILE" if Global.is_orientation_portrait() else "PROFILE_IN_GAME")
	Global.metrics.track_screen_viewed(current_screen_name, "")
	select_node(control_profile_settings, play_sfx)


func select_node(node: PlaceholderManager, play_sfx: bool = true):
	if selected_node and not selected_node.instance:
		selected_node = null
	if selected_node != node:
		if selected_node and selected_node.instance:
			fade_out(selected_node)
		fade_in(node)

		if play_sfx:
			UiSounds.play_sound("generic_button_press")
	if selected_node == node:
		if selected_node.instance:
			selected_node.instance.show()


func fade_in(node: PlaceholderManager):
	if not is_instance_valid(node.instance):
		return
	selected_node = node
	node.instance.show()
	if is_instance_valid(fade_in_tween):
		if fade_in_tween.is_running():
			fade_out_tween.custom_step(100.0)
			fade_in_tween.kill()
	node.instance.modulate.a = 0.0
	fade_in_tween = create_tween()
	fade_in_tween.tween_property(node.instance, "modulate", Color(1, 1, 1), 0.3)


func fade_out(node: PlaceholderManager):
	if not is_instance_valid(node.instance):
		return
	if is_instance_valid(fade_out_tween):
		if fade_out_tween.is_running():
			fade_out_tween.custom_step(100.0)
			fade_out_tween.kill()

	node.instance.modulate.a = 1.0
	fade_out_tween = create_tween()
	fade_out_tween.tween_property(node.instance, "modulate", Color(1, 1, 1, 0), 0.3)
	fade_out_tween.tween_callback(node.instance.hide)
	fade_out_tween.tween_callback(node.async_put_to_sleep)


func _on_visibility_changed():
	if is_visible_in_tree():
		Global.on_menu_open.emit()
		UiSounds.play_sound("mainmenu_widget_open")
		grab_focus()
		Global.explorer_release_focus()
	else:
		UiSounds.play_sound("mainmenu_widget_close")
		Global.on_menu_close.emit()


func _async_request_hide_menu():
	await Global.player_identity.async_save_profile()
	hide_menu.emit()


func _on_button_backpack_toggled(toggled_on):
	if !toggled_on:
		Global.player_identity.async_save_profile()


func _on_notification_clicked(notification_dict: Dictionary) -> void:
	# Handle notification clicks - open backpack for reward notifications
	var notif_type = notification_dict.get("type", "")

	# Check if this is a reward notification
	if notif_type in ["reward_assignment", "reward_in_progress"]:
		# Open the backpack to show the reward
		async_show_backpack()
		Global.open_navbar_silently.emit()


func _on_deep_link_received() -> void:
	Global.check_deep_link_teleport_to()


func _on_portrait_button_discover_pressed() -> void:
	async_show_discover()


func _on_portrait_button_backpack_pressed() -> void:
	async_show_backpack()


func _on_portrait_button_settings_pressed() -> void:
	async_show_settings()


func _on_account_delete() -> void:
	if account_deletion_pop_up:
		account_deletion_pop_up.async_start_flow()
