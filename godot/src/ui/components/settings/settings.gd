extends Control

signal request_debug_panel(enabled: bool)
signal request_pause_scenes(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)

enum SceneLogLevel {
	LOG = 1,
	SCENE_ERROR = 2,
	SYSTEM_ERROR = 3,
}

var preview_ws = WebSocketPeer.new()
var _preview_connect_to_url: String = ""
var _dirty_closed: bool = false
var _dirty_connected: bool = false

@onready
var general = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_General
@onready
var graphics = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics
@onready
var advanced = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced
@onready
var audio = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Audio

#General items:
@onready
var text_edit_cache_path = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_General/VBoxContainer_CachePath/TextEdit_CachePath

#Audio items
@onready
var h_slider_general_volume = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Audio/MasterVolume/HSlider_GeneralVolume

#Graphics items:
@onready
var h_slider_rendering_scale = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/Resolution3DScale/HSlider_Resolution3DScale
@onready
var menu_button_ui_zoom = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/UiZoom/MenuButton_UiZoom
@onready
var v_box_container_windowed = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/VBoxContainer_Windowed
@onready
var checkbox_windowed = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/VBoxContainer_Windowed/Checkbox_Windowed
@onready
var menu_button_limit_fps = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/LimitFps/MenuButton_LimitFps
@onready
var menu_button_skybox = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/Skybox/MenuButton_Skybox

#Advanced items:
@onready
var option_button_realm = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/VBoxContainer_Realm/HBoxContainer2/OptionButton_Realm
@onready
var line_edit_preview_url = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/VBoxContainer_Connection/HBoxContainer/LineEdit_PreviewUrl
@onready
var label_ws_state = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/VBoxContainer_Connection/HBoxContainer2/Label_WsState

@onready
var h_slider_process_tick_quota = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/VBoxContainer_ProcessTickQuota/HBoxContainer/HSlider_ProcessTickQuota
@onready
var label_process_tick_quota_value = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/VBoxContainer_ProcessTickQuota/HBoxContainer/Label_ProcessTickQuotaValue

@onready
var label_scene_radius_value = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_General/VBoxContainer_SceneRadius/HBoxContainer/Label_SceneRadiusValue
@onready
var h_slider_scene_radius = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_General/VBoxContainer_SceneRadius/HBoxContainer/HSlider_SceneRadius

@onready
var spin_box_gravity = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/HBoxContainer/HBoxContainer_Gravity/SpinBox_Gravity
@onready
var spin_box_jump_velocity = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/HBoxContainer/HBoxContainer_JumpVelocity/SpinBox_JumpVelocity
@onready
var spin_box_run_speed = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/HBoxContainer2/HBoxContainer_RunSpeed/SpinBox_RunSpeed
@onready
var spin_box_walk_speed = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/HBoxContainer2/HBoxContainer_WalkSpeed/SpinBox_WalkSpeed
@onready
var check_box_raycast_debugger = $ColorRect_Content/HBoxContainer/ScrollContainer/VBoxContainer/VBoxContainer_Advanced/HBoxContainer5/CheckBox_RaycastDebugger


func _ready():
	general.show()
	graphics.hide()
	advanced.hide()
	audio.hide()

	text_edit_cache_path.text = Global.config.local_content_dir

	if Global.is_mobile():
		v_box_container_windowed.hide()
		checkbox_windowed.disabled = true
	else:
		checkbox_windowed.button_pressed = Global.config.windowed

	refresh_zooms()

	h_slider_general_volume.value = Global.config.audio_general_volume
	menu_button_limit_fps.selected = Global.config.limit_fps
	menu_button_skybox.selected = Global.config.skybox

	refresh_values()


func _on_button_pressed():
	self.hide()


func _on_general_button_toggled(_button_pressed):
	general.show()
	graphics.hide()
	audio.hide()
	advanced.hide()


func _on_graphic_button_toggled(_button_pressed):
	general.hide()
	graphics.show()
	audio.hide()
	advanced.hide()


func _on_devloper_button_toggled(_button_pressed):
	general.hide()
	graphics.hide()
	audio.hide()
	advanced.show()


func _on_button_audio_pressed():
	general.hide()
	graphics.hide()
	audio.show()
	advanced.hide()


func _on_button_clear_cache_pressed():
	# Clean the content cache folder
	Global.clear_cache()


func _on_checkbox_fps_toggled(button_pressed):
	Global.config.show_fps = button_pressed


func _on_menu_button_limit_fps_item_selected(index):
	Global.config.limit_fps = index
	GraphicSettings.apply_fps_limit()
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

	if is_instance_valid(Global.raycast_debugger):
		check_box_raycast_debugger.set_pressed_no_signal(true)


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


func _on_check_box_raycast_debugger_toggled(toggled_on):
	Global.set_raycast_debugger_enable(toggled_on)


func _on_button_profile_pressed():
	pass  # Replace with function body.


func refresh_zooms():
	var selected_index: int = -1
	var i: int = 0
	var options := GraphicSettings.get_ui_zoom_available(get_window())
	menu_button_ui_zoom.clear()

	for ui_zoom_option in options.keys():
		menu_button_ui_zoom.add_item(ui_zoom_option)
		if options[ui_zoom_option] == get_window().content_scale_factor:
			selected_index = i
		i += 1
	if selected_index == -1:
		selected_index = i - 1
	menu_button_ui_zoom.selected = selected_index


func _on_checkbox_windowed_toggled(toggled_on):
	Global.config.windowed = toggled_on
	GraphicSettings.apply_window_config()
	refresh_zooms()


func _on_menu_button_ui_zoom_item_selected(index):
	var options := GraphicSettings.get_ui_zoom_available(get_window())
	var current_ui_zoom: String = menu_button_ui_zoom.get_item_text(index)
	if not options.has(current_ui_zoom):
		current_ui_zoom = "Max"
	Global.config.ui_zoom = options[current_ui_zoom]
	GraphicSettings.apply_ui_zoom(get_window())
	Global.config.save_to_settings_file()


func _on_h_slider_rendering_scale_drag_ended(_value_changed):
	Global.config.resolution_3d_scale = h_slider_rendering_scale.value
	get_window().get_viewport().scaling_3d_scale = Global.config.resolution_3d_scale
	Global.config.save_to_settings_file()


func _on_h_slider_general_volume_drag_ended(_value_changed):
	Global.config.audio_general_volume = h_slider_general_volume.value
	AudioSettings.apply_volume_settings()
