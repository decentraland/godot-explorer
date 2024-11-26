extends Control

signal hide_menu
signal jump_to(Vector2i)
signal toggle_minimap
signal toggle_fps
signal toggle_ram
signal request_pause_scenes(enabled: bool)
signal request_debug_panel(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)
#signals from advanced settings

const BACKPACK_OFF = preload("res://assets/ui/nav-bar-icons/backpack-off.svg")
const BACKPACK_ON = preload("res://assets/ui/nav-bar-icons/backpack-on.svg")
const EXPLORER_OFF = preload("res://assets/ui/nav-bar-icons/explorer-off.svg")
const EXPLORER_ON = preload("res://assets/ui/nav-bar-icons/explorer-on.svg")
const MAP_OFF = preload("res://assets/ui/nav-bar-icons/map-off.svg")
const MAP_ON = preload("res://assets/ui/nav-bar-icons/map-on.svg")
const SETTINGS_OFF = preload("res://assets/ui/nav-bar-icons/settings-off.svg")
const SETTINGS_ON = preload("res://assets/ui/nav-bar-icons/settings-on.svg")

var buttons_quantity: int = 0
var pressed_index: int = 0

var selected_node: Control

var fade_out_tween: Tween = null

@onready var group: ButtonGroup = ButtonGroup.new()
@onready var color_rect_header = %ColorRect_Header

@onready var control_discover = %Control_Discover
@onready var control_settings = %Control_Settings
@onready var control_map = %Control_Map
@onready var control_backpack: Backpack = %Control_Backpack
@onready var control_profile_settings: ProfileSettings = %Control_ProfileSettings

@onready var button_discover = %Button_Discover
@onready var button_map = %Button_Map
@onready var button_backpack = %Button_Backpack
@onready var button_settings = %Button_Settings
@onready var control_deploying_profile = %Control_DeployingProfile

@onready var button_magic_wallet = %Button_MagicWallet


func _ready():
	control_deploying_profile.hide()
	control_settings.request_pause_scenes.connect(func(enabled): request_pause_scenes.emit(enabled))
	control_settings.request_debug_panel.connect(func(enabled): request_debug_panel.emit(enabled))
	control_settings.preview_hot_reload.connect(
		func(scene_type, scene_id): preview_hot_reload.emit(scene_type, scene_id)
	)

	self.modulate = Color(1, 1, 1, 0)
	button_settings.set_pressed(true)
	selected_node = control_settings
	control_map.hide()
	control_settings.show()
	control_discover.hide()
	control_backpack.hide()
	control_profile_settings.hide()
	control_map.jump_to.connect(_jump_to)

	# Leave it, because we can open a browser with the Magic Wallet
	button_magic_wallet.visible = false


func _unhandled_input(event):
	if event is InputEventKey and visible:
		if event.pressed and event.keycode == KEY_TAB:
			pressed_index = group.get_pressed_button().get_index()
			buttons_quantity = group.get_buttons().size() - 1
			control_map.clear()

			if pressed_index < buttons_quantity:
				group.get_buttons()[pressed_index + 1].set_pressed(true)
				group.get_buttons()[pressed_index + 1].emit_signal("pressed")
			else:
				#change index to 0 to include "Control Discover"
				group.get_buttons()[1].set_pressed(true)
				group.get_buttons()[1].emit_signal("pressed")
		if event.pressed and event.keycode == KEY_ESCAPE:
			_async_request_hide_menu()
		if event.pressed and event.keycode == KEY_M:
			if selected_node == control_map:
				_async_request_hide_menu()
			else:
				show_map()
		if event.pressed and event.keycode == KEY_P:
			if selected_node == control_settings:
				_async_request_hide_menu()
			else:
				_on_button_settings_pressed()


func _on_button_close_pressed():
	_async_request_hide_menu()


func _jump_to(parcel: Vector2i):
	jump_to.emit(parcel)


func close():
	color_rect_header.hide()
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3).set_ease(Tween.EASE_IN_OUT)
	var tween_h = create_tween()
	tween_h.tween_callback(hide).set_delay(0.3)


func show_last():
	self.show()
	self.grab_focus()
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.3).set_ease(Tween.EASE_IN_OUT)
	color_rect_header.show()


func show_map():
	select_node(control_map, false)
	button_map.set_pressed(true)
	_open()


func show_backpack():
	select_node(control_backpack, false)
	button_backpack.set_pressed(true)
	_open()


func show_settings():
	select_node(control_settings, false)
	button_settings.set_pressed(true)
	_open()


func _open():
	if not visible:
		show()
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.25).set_ease(Tween.EASE_IN_OUT)
	color_rect_header.show()


func _on_control_settings_toggle_fps_visibility(visibility):
	emit_signal("toggle_fps", visibility)


func _on_control_settings_toggle_map_visibility(visibility):
	emit_signal("toggle_minimap", visibility)


func _on_control_settings_toggle_ram_usage_visibility(visibility):
	emit_signal("toggle_ram", visibility)


func select_node(node: Node, play_sfx: bool = true):
	if selected_node != node:
		fade_out(selected_node)
		fade_in(node)

		if play_sfx:
			UiSounds.play_sound("mainmenu_tab_switch")


func _on_button_settings_pressed():
	select_node(control_settings)


func _on_button_map_pressed():
	select_node(control_map)


func _on_button_discover_pressed():
	select_node(control_discover)


func _on_button_backpack_pressed():
	select_node(control_backpack)


func _on_menu_profile_button_open_menu_profile():
	select_node(control_profile_settings)


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
		UiSounds.play_sound("mainmenu_widget_open")
		grab_focus()
		Global.explorer_release_focus()
	else:
		UiSounds.play_sound("mainmenu_widget_close")


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


func _on_button_discover_toggled(toggled_on):
	button_discover.icon = EXPLORER_ON if toggled_on else EXPLORER_OFF


func _on_button_map_toggled(toggled_on):
	button_map.icon = MAP_ON if toggled_on else MAP_OFF


func _on_button_backpack_toggled(toggled_on):
	button_backpack.icon = BACKPACK_ON if toggled_on else BACKPACK_OFF
	if !toggled_on:
		_async_deploy_if_has_changes()


func _on_button_settings_toggled(toggled_on):
	button_settings.icon = SETTINGS_ON if toggled_on else SETTINGS_OFF


func _on_button_magic_wallet_pressed():
	pass
	# On future we can open the magic wallet in a WebKit / WebView
