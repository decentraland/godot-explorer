extends Control

signal request_debug_panel(enabled: bool)
signal request_pause_scenes(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)

enum SceneLogLevel {
	LOG = 1,
	SCENE_ERROR = 2,
	SYSTEM_ERROR = 3,
}

var resolution_manager: ResolutionManager = ResolutionManager.new()

var preview_ws = WebSocketPeer.new()
var _preview_connect_to_url: String = ""
var _dirty_closed: bool = false
var _dirty_connected: bool = false

@onready
var general = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General
@onready
var graphics = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics
@onready
var advanced = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced

@onready
var text_edit = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_CachePath/TextEdit_CachePath
@onready
var window_size_menu_button = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics/WindowSize/MenuButton_WindowSize
@onready
var resolution_menu_button = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics/Resolution/MenuButton_Resolution
@onready
var minimap = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/Checkbox_Minimap
@onready
var label_gui_scale = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics/GuiScale/HBoxContainer/Label_GuiScale
@onready
var h_slider_gui_scale = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics/GuiScale/HBoxContainer/HSlider_GuiScale
@onready
var menu_button_limit_fps = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics/LimitFps/MenuButton_LimitFps
@onready
var menu_button_skybox = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Graphics/Skybox/MenuButton_Skybox

@onready
var option_button_realm = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_Realm/HBoxContainer2/OptionButton_Realm
@onready
var line_edit_preview_url = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_Connection/HBoxContainer/LineEdit_PreviewUrl
@onready
var label_ws_state = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_Connection/HBoxContainer2/Label_WsState
@onready
var h_slider_process_tick_quota = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_ProcessTickQuota/HBoxContainer/HSlider_ProcessTickQuota
@onready
var label_process_tick_quota_value = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_ProcessTickQuota/HBoxContainer/Label_ProcessTickQuotaValue
@onready
var h_slider_scene_radius = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_SceneRadius/HBoxContainer/HSlider_SceneRadius
@onready
var label_scene_radius_value = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/VBoxContainer_SceneRadius/HBoxContainer/Label_SceneRadiusValue
@onready
var spin_box_gravity = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/HBoxContainer/HBoxContainer_Gravity/SpinBox_Gravity
@onready
var spin_box_jump_velocity = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/HBoxContainer/HBoxContainer_JumpVelocity/SpinBox_JumpVelocity
@onready
var spin_box_run_speed = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/HBoxContainer2/HBoxContainer_RunSpeed/SpinBox_RunSpeed
@onready
var spin_box_walk_speed = $VBoxContainer/HBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_Advanced/HBoxContainer2/HBoxContainer_WalkSpeed/SpinBox_WalkSpeed


func _ready():
	if Global.is_mobile:
		window_size_menu_button.disabled = true
		resolution_menu_button.disabled = true

	general.show()
	graphics.hide()
	advanced.hide()

	text_edit.text = Global.config.local_content_dir

	resolution_manager.refresh_window_options()
	for item in resolution_manager.window_options.keys():
		window_size_menu_button.add_item(item)

	load_resolutions()
	refresh_resolution()

	menu_button_limit_fps.selected = Global.config.limit_fps
	menu_button_skybox.selected = Global.config.skybox

	refresh_values()


func _on_button_pressed():
	self.hide()


func _on_window_size_menu_button_item_selected(index):
	var current_window_size: String = window_size_menu_button.get_item_text(index)
	resolution_manager.change_window_size(get_window(), get_viewport(), current_window_size)
	load_resolutions()
	resolution_manager.center_window(get_window())
	Global.config.window_size = current_window_size
	Global.config.resolution = current_window_size
	Global.config.ui_scale = 1.0
	refresh_resolution()


func _on_resolution_menu_button_item_selected(index):
	var current_res: String = resolution_menu_button.get_item_text(index)
	resolution_manager.change_resolution(get_window(), get_viewport(), current_res)
	resolution_manager.center_window(get_window())
	resolution_manager.change_ui_scale(get_window(), 1.0)
	Global.config.resolution = current_res
	Global.config.ui_scale = 1.0
	Global.config.save_to_settings_file()
	refresh_resolution()


func refresh_resolution():
	for index in range(resolution_menu_button.item_count):
		var current_res: String = resolution_menu_button.get_item_text(index)
		if current_res == Global.config.resolution:
			resolution_menu_button.selected = index

	for index in range(window_size_menu_button.item_count):
		var current_res: String = window_size_menu_button.get_item_text(index)
		if current_res == Global.config.window_size:
			window_size_menu_button.selected = index

	h_slider_gui_scale.set_value_no_signal(Global.config.ui_scale * 100.0)
	label_gui_scale.text = str(round(100.0 * Global.config.ui_scale)) + "%"


func load_resolutions():
	resolution_menu_button.clear()
	for item in resolution_manager.resolution_options:
		resolution_menu_button.add_item(item)


func _on_general_button_toggled(_button_pressed):
	general.show()
	graphics.hide()
	advanced.hide()


func _on_graphic_button_toggled(_button_pressed):
	general.hide()
	graphics.show()
	advanced.hide()


func _on_monitoring_button_toggled(_button_pressed):
	general.hide()
	graphics.hide()
	advanced.show()


func _on_h_slider_drag_ended(value_changed):
	if value_changed:
		resolution_manager.change_ui_scale(get_window(), h_slider_gui_scale.value / 100.0)
		Global.config.ui_scale = h_slider_gui_scale.value / 100.0
		label_gui_scale.text = str(round(100.0 * Global.config.ui_scale)) + "%"
		Global.config.save_to_settings_file()


func _on_button_clear_cache_pressed():
	# Clean the content cache folder
	if DirAccess.dir_exists_absolute(Global.config.local_content_dir):
		for file in DirAccess.get_files_at(Global.config.local_content_dir):
			DirAccess.remove_absolute(Global.config.local_content_dir + file)
		DirAccess.remove_absolute(Global.config.local_content_dir)

	if not DirAccess.dir_exists_absolute(Global.config.local_content_dir):
		DirAccess.make_dir_absolute(Global.config.local_content_dir)


func _on_checkbox_fps_toggled(button_pressed):
	Global.config.show_fps = button_pressed


func _on_menu_button_limit_fps_item_selected(index):
	Global.config.limit_fps = index
	resolution_manager.apply_fps_limit()
	Global.config.save_to_settings_file()


func _on_menu_button_skybox_item_selected(index):
	Global.config.skybox = index
	Global.config.save_to_settings_file()


func refresh_values():
	spin_box_gravity.value = Global.config.gravity
	spin_box_walk_speed.value = Global.config.walk_velocity
	spin_box_run_speed.value = Global.config.run_velocity
	spin_box_jump_velocity.value = Global.config.jump_velocity
	h_slider_process_tick_quota.set_value_no_signal(Global.config.process_tick_quota_ms)
	h_slider_scene_radius.set_value_no_signal(Global.config.scene_radius)
	label_process_tick_quota_value.text = str(Global.config.process_tick_quota_ms)
	label_scene_radius_value.text = str(Global.config.scene_radius)


func _on_h_slider_process_tick_quota_value_changed(value):
	label_process_tick_quota_value.text = str(value)


func _on_option_button_realm_item_selected(index):
	Global.realm.async_set_realm(option_button_realm.get_item_text(index))


func set_ws_state(connected: bool) -> void:
	if connected:
		label_ws_state.text = "Connected"
		label_ws_state.add_theme_color_override("font_color", Color.FOREST_GREEN)
	else:
		label_ws_state.text = "Disconnected"
		label_ws_state.add_theme_color_override("font_color", Color.RED)


func _process(_delta):
	preview_ws.poll()

	var state = preview_ws.get_ready_state()
	if state == WebSocketPeer.STATE_OPEN:
		if not _preview_connect_to_url.is_empty():
			preview_ws.close()

		if _dirty_connected:
			_dirty_connected = false
			_dirty_closed = true
			set_ws_state(true)

		while preview_ws.get_available_packet_count():
			var packet = preview_ws.get_packet().get_string_from_utf8()
			var json = JSON.parse_string(packet)
			if json != null and json is Dictionary:
				var msg_type = json.get("type", "")
				match msg_type:
					"SCENE_UPDATE":
						var scene_id = json.get("payload", {}).get("sceneId", "unknown")
						var scene_type = json.get("payload", {}).get("sceneType", "scene")
						print("preview-ws > update of ", scene_type, " with id '", scene_id, "'")
						preview_hot_reload.emit(scene_type, scene_id)
					_:
						printerr("preview-ws > unknown message type ", msg_type)

	elif state == WebSocketPeer.STATE_CLOSING:
		_dirty_closed = true
	elif state == WebSocketPeer.STATE_CLOSED:
		if _dirty_closed:
			set_ws_state(false)

			var code = preview_ws.get_close_code()
			var reason = preview_ws.get_close_reason()
			print(
				(
					"preview-ws > closed with code: %d, reason %s. Clean: %s"
					% [code, reason, code != -1]
				)
			)
			_dirty_closed = false

		if not _preview_connect_to_url.is_empty():
			preview_ws.connect_to_url(_preview_connect_to_url)
			print("preview-ws > connecting to ", _preview_connect_to_url)
			_preview_connect_to_url = ""
			_dirty_connected = true


func _on_spin_box_walk_speed_value_changed(value):
	Global.config.walk_velocity = value
	Global.config.save_to_settings_file()


func _on_spin_box_run_speed_value_changed(value):
	Global.config.run_velocity = value
	Global.config.save_to_settings_file()


func _on_spin_box_jump_velocity_value_changed(value):
	Global.config.jump_velocity = value
	Global.config.save_to_settings_file()


func _on_spin_box_gravity_value_changed(value):
	Global.config.gravity = value
	Global.config.save_to_settings_file()


func _on_h_slider_scene_radius_value_changed(value):
	if value != Global.config.scene_radius:
		Global.config.scene_radius = h_slider_scene_radius.value
		label_scene_radius_value.text = str(h_slider_scene_radius.value)
		Global.config.save_to_settings_file()


func _on_button_connect_preview_pressed():
	_preview_connect_to_url = (
		line_edit_preview_url
		. text
		. to_lower()
		. replace("http://", "ws://")
		. replace("https://", "wss://")
	)


func _on_check_box_scene_log_toggled(toggled_on):
	request_debug_panel.emit(toggled_on)


func _on_check_box_scene_pause_toggled(toggled_on):
	emit_signal("request_pause_scenes", toggled_on)
