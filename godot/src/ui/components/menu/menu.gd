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
var buttons_quantity: int = 0
var pressed_index: int = 0

var selected_node: Control
var current_screen_name: String = ""
var fade_out_tween: Tween = null

@onready var group: ButtonGroup = ButtonGroup.new()

@onready var control_discover = %Control_Discover
@onready var control_settings = %Control_Settings
@onready var control_backpack: Backpack = %Control_Backpack
@onready var control_profile_settings: ProfileSettings = %Control_ProfileSettings

@onready var control_deploying_profile = %Control_DeployingProfile

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
	control_settings.request_pause_scenes.connect(func(enabled): request_pause_scenes.emit(enabled))
	control_settings.request_debug_panel.connect(func(enabled): request_debug_panel.emit(enabled))
	control_settings.preview_hot_reload.connect(
		func(scene_type, scene_id): preview_hot_reload.emit(scene_type, scene_id)
	)

	self.modulate = Color(1, 1, 1, 1)
	current_screen_name = ("DISCOVER" if Global.is_orientation_portrait() else "DISCOVER_IN_GAME")
	if !is_in_game:
		Global.metrics.track_screen_viewed(current_screen_name, '{"function": "ready"}')
		Global.metrics.flush()

	selected_node = control_discover
	control_settings.hide()
	control_discover.show()
	control_backpack.hide()
	control_profile_settings.hide()

	# Connect to notification clicked signal for reward notifications
	Global.notification_clicked.connect(_on_notification_clicked)

	Global.deep_link_received.connect(_on_deep_link_received)
	Global.open_settings.connect(show_settings)
	Global.open_backpack.connect(show_backpack)
	Global.open_discover.connect(show_discover)
	Global.open_own_profile.connect(show_own_profile)
	Global.close_menu.connect(close)
	Global.delete_account.connect(_on_account_delete)


func _on_button_close_pressed():
	_async_request_hide_menu()


# gdlint:ignore = async-function-name
func close():
	# Wait for profile deploy if backpack has changes before closing
	if selected_node == control_backpack:
		await _async_deploy_if_has_changes()

	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3).set_ease(Tween.EASE_IN_OUT)
	var tween_h = create_tween()
	tween_h.tween_callback(hide).set_delay(0.3)


func show_discover():
	select_discover_screen()
	hud_button_discover.toggled.emit(true)
	_open()


func show_backpack(on_emotes := false):
	select_backpack_screen()
	if on_emotes:
		control_backpack.show_emotes()
		control_backpack.press_button_emotes()
	_open()


func show_settings():
	select_settings_screen()
	_open()


func show_own_profile():
	select_profile_screen()
	_open()


func _open():
	if not visible:
		show()
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.25).set_ease(Tween.EASE_IN_OUT)


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


func select_node(node: Node, play_sfx: bool = true):
	if selected_node != node:
		fade_out(selected_node)
		fade_in(node)

		if play_sfx:
			UiSounds.play_sound("generic_button_press")


func fade_in(node: Control):
	selected_node = node
	node.show()
	var tween = create_tween()
	tween.tween_property(node, "modulate", Color(1, 1, 1), 0.3)


func fade_out(node: Control):
	if is_instance_valid(fade_out_tween):
		if fade_out_tween.is_running():
			selected_node.hide()
			fade_out_tween.stop()

	fade_out_tween = create_tween().set_parallel(true)
	fade_out_tween.tween_property(node, "modulate", Color(1, 1, 1, 0), 0.3)
	fade_out_tween.tween_callback(node.hide).set_delay(0.3)


func _on_visibility_changed():
	if is_visible_in_tree():
		Global.on_menu_open.emit()
		UiSounds.play_sound("mainmenu_widget_open")
		grab_focus()
		Global.explorer_release_focus()
	else:
		UiSounds.play_sound("mainmenu_widget_close")
		Global.on_menu_close.emit()


func _async_deploy_if_has_changes():
	if control_backpack.has_changes():
		control_deploying_profile.show()
		await control_backpack.async_save_profile()
		control_deploying_profile.hide()


func _async_request_hide_menu():
	if control_deploying_profile.visible:  # loading...
		return

	await _async_deploy_if_has_changes()

	hide_menu.emit()


func _on_button_backpack_toggled(toggled_on):
	if !toggled_on:
		_async_deploy_if_has_changes()


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


func _on_notification_clicked(notification: Dictionary) -> void:
	# Handle notification clicks - open backpack for reward notifications
	var notif_type = notification.get("type", "")

	# Check if this is a reward notification
	if notif_type in ["reward_assignment", "reward_in_progress"]:
		# Open the backpack to show the reward
		show_backpack()
		Global.open_navbar_silently.emit()


func _on_deep_link_received() -> void:
	Global.check_deep_link_teleport_to()


func _on_portrait_button_discover_pressed() -> void:
	show_discover()


func _on_portrait_button_backpack_pressed() -> void:
	show_backpack()


func _on_portrait_button_settings_pressed() -> void:
	show_settings()


func _on_account_delete() -> void:
	if account_deletion_pop_up:
		account_deletion_pop_up.async_start_flow()
