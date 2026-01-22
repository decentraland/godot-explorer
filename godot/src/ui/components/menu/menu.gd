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

@onready var group: ButtonGroup = ButtonGroup.new()

@onready var control_discover := PlaceholderManager.new(%Control_Discover)
@onready var control_settings := PlaceholderManager.new(%Control_Settings)
@onready var control_backpack := PlaceholderManager.new(%Control_Backpack)
@onready var control_profile_settings := PlaceholderManager.new(%Control_ProfileSettings)

@onready var control_deploying_profile := %Control_DeployingProfile

@onready var portrait_button_profile: Button = %Portrait_Button_Profile

@onready var color_rect_portrait_top_safe_area: ColorRect = %ColorRect_Portrait_Top_SafeArea
@onready var color_rect_portrait_bottom_safe_area: ColorRect = %ColorRect_Portrait_Bottom_SafeArea
@onready var account_deletion_pop_up: TextureRect = $AccountDeletionPopUp

@onready var hud_button_backpack: Button = %HudButton_Backpack
@onready var hud_button_discover: Button = %HudButton_Discover
@onready var hud_button_settings: Button = %HudButton_Settings


func _ready():
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	hud_button_backpack.button_group = btn_group
	hud_button_discover.button_group = btn_group
	hud_button_settings.button_group = btn_group
	portrait_button_profile.button_group = btn_group
	Global.open_discover.emit()
	hud_button_discover.button_pressed = true

	account_deletion_pop_up.hide()

	is_in_game = self != get_tree().current_scene
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()

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
	if Global.player_identity.has_changes():
		Global.player_identity.async_save_profile()
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3).set_ease(Tween.EASE_IN_OUT)
	var tween_h = create_tween()
	tween_h.tween_callback(hide).set_delay(0.3)
	tween_h.tween_callback(
		func():
			if selected_node:
				selected_node.queue_free_instance()
	)


func async_show_discover():
	await control_discover._async_instantiate()
	select_discover_screen()
	if is_instance_valid(hud_button_discover):
		hud_button_discover.toggled.emit(true)
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
	if selected_node and not selected_node.instance:
		selected_node = null
	if not selected_node:
		async_show_discover()
	if not visible:
		show()
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.25).set_ease(Tween.EASE_IN_OUT)
	tween.tween_callback(func(): is_open = true)


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


func _on_size_changed() -> void:
	var safe_area: Rect2i = Global.get_safe_area()
	var window_size: Vector2i = DisplayServer.window_get_size()

	var top: int = 0
	var bottom: int = 0

	if window_size.x >= safe_area.size.x and window_size.y >= safe_area.size.y:
		var y_factor: float = size.y / window_size.y

		top = max(top, safe_area.position.y * y_factor)
		bottom = max(bottom, abs(safe_area.end.y - window_size.y) * y_factor)

	if (
		is_instance_valid(color_rect_portrait_top_safe_area)
		and is_instance_valid(color_rect_portrait_bottom_safe_area)
	):
		color_rect_portrait_top_safe_area.custom_minimum_size.y = top
		color_rect_portrait_bottom_safe_area.custom_minimum_size.y = bottom


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
