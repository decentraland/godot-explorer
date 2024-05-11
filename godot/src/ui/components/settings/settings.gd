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
@onready var h_slider_general_volume = %HSlider_GeneralVolume
@onready var h_slider_scene_volume = %HSlider_SceneVolume
@onready var h_slider_ui_volume = %HSlider_UIVolume
@onready var h_slider_music_volume = %HSlider_MusicVolume
@onready var h_slider_voice_chat_volume = %HSlider_VoiceChatVolume
@onready var h_slider_mic_amplification = %HSlider_MicAmplification

#Graphics items:
@onready var h_slider_rendering_scale = %HSlider_Resolution3DScale
@onready var radio_selector_ui_zoom = %RadioSelector_UiZoom

@onready var v_box_container_windowed = %VBoxContainer_Windowed
@onready var radio_selector_windowed = %RadioSelector_Windowed

@onready var box_container_custom = %VBoxContainer_Custom

@onready var radio_selector_graphic_profile = %RadioSelector_GraphicProfile

@onready var radio_selector_texture_quality = %RadioSelector_TextureQuality
@onready var radio_selector_skybox = %RadioSelector_Skybox
@onready var radio_selector_shadow = %RadioSelector_Shadow
@onready var radio_selector_aa = %RadioSelector_AA

@onready var radio_selector_limit_fps = %RadioSelector_LimitFps

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

	text_edit_cache_path.text = Global.get_config().local_content_dir

	refresh_graphic_settings()

	h_slider_general_volume.value = Global.get_config().audio_general_volume
	h_slider_scene_volume.value = Global.get_config().audio_scene_volume
	h_slider_voice_chat_volume.value = Global.get_config().audio_voice_chat_volume
	h_slider_ui_volume.value = Global.get_config().audio_ui_volume
	h_slider_music_volume.value = Global.get_config().audio_music_volume
	h_slider_mic_amplification.value = Global.get_config().audio_mic_amplification

	refresh_values()


func refresh_graphic_settings():
	# We only show the custom settings if the graphic profile is custom
	box_container_custom.visible = Global.get_config().graphic_profile == 3
	var graphic_profile = Global.get_config().graphic_profile
	radio_selector_graphic_profile.selected = graphic_profile

	if Global.is_mobile():
		v_box_container_windowed.hide()
	else:
		radio_selector_windowed.selected = Global.get_config().window_mode

	radio_selector_limit_fps.selected = Global.get_config().limit_fps
	radio_selector_texture_quality.selected = Global.get_config().texture_quality
	radio_selector_skybox.selected = Global.get_config().skybox
	radio_selector_shadow.selected = Global.get_config().shadow_quality
	radio_selector_aa.selected = Global.get_config().anti_aliasing

	h_slider_rendering_scale.value = Global.get_config().resolution_3d_scale
	refresh_zooms()


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
	Global.get_config().show_fps = button_pressed


func refresh_values():
	spin_box_gravity.value = Global.get_config().gravity
	spin_box_walk_speed.value = Global.get_config().walk_velocity
	spin_box_run_speed.value = Global.get_config().run_velocity
	spin_box_jump_velocity.value = Global.get_config().jump_velocity
	h_slider_process_tick_quota.set_value_no_signal(Global.get_config().process_tick_quota_ms)
	h_slider_scene_radius.set_value_no_signal(Global.get_config().scene_radius)
	label_process_tick_quota_value.text = str(Global.get_config().process_tick_quota_ms)
	label_scene_radius_value.text = str(Global.get_config().scene_radius)

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
	Global.get_config().walk_velocity = value
	Global.get_config().save_to_settings_file()


func _on_spin_box_run_speed_value_changed(value):
	Global.get_config().run_velocity = value
	Global.get_config().save_to_settings_file()


func _on_spin_box_jump_velocity_value_changed(value):
	Global.get_config().jump_velocity = value
	Global.get_config().save_to_settings_file()


func _on_spin_box_gravity_value_changed(value):
	Global.get_config().gravity = value
	Global.get_config().save_to_settings_file()


func _on_h_slider_scene_radius_value_changed(value):
	if value != Global.get_config().scene_radius:
		Global.get_config().scene_radius = h_slider_scene_radius.value
		label_scene_radius_value.text = str(h_slider_scene_radius.value)
		Global.get_config().save_to_settings_file()


func _on_button_connect_preview_pressed():
	set_preview_url(line_edit_preview_url.text)


func set_preview_url(url: String) -> void:
	_preview_connect_to_url = url.to_lower().replace("http://", "ws://").replace(
		"https://", "wss://"
	)


func _on_check_box_scene_log_toggled(toggled_on):
	request_debug_panel.emit(toggled_on)


func _on_check_box_scene_pause_toggled(toggled_on):
	emit_signal("request_pause_scenes", toggled_on)


func _on_check_box_raycast_debugger_toggled(toggled_on):
	Global.set_raycast_debugger_enable(toggled_on)


func refresh_zooms():
	var selected_index: int = -1
	var i: int = 0
	var options := GraphicSettings.get_ui_zoom_available(get_window())
	radio_selector_ui_zoom.clear()

	for ui_zoom_option in options.keys():
		radio_selector_ui_zoom.add_item(ui_zoom_option)
		if options[ui_zoom_option] == get_window().content_scale_factor:
			selected_index = i
		i += 1
	if selected_index == -1:
		selected_index = i - 1
	radio_selector_ui_zoom.selected = selected_index


func _on_h_slider_rendering_scale_drag_ended(_value_changed):
	Global.get_config().resolution_3d_scale = h_slider_rendering_scale.value
	get_window().get_viewport().scaling_3d_scale = Global.get_config().resolution_3d_scale
	Global.get_config().save_to_settings_file()


func _on_h_slider_mic_amplification_value_changed(value):
	Global.get_config().audio_mic_amplification = value
	AudioSettings.apply_mic_amplification_settings()
	Global.get_config().save_to_settings_file()


func _on_h_slider_ui_volume_value_changed(value):
	Global.get_config().audio_ui_volume = value
	AudioSettings.apply_ui_volume_settings()
	Global.get_config().save_to_settings_file()


func _on_h_slider_voice_chat_volume_value_changed(value):
	Global.get_config().audio_voice_chat_volume = value
	AudioSettings.apply_voice_chat_volume_settings()
	Global.get_config().save_to_settings_file()


func _on_h_slider_scene_volume_value_changed(value):
	Global.get_config().audio_scene_volume = value
	AudioSettings.apply_scene_volume_settings()
	Global.get_config().save_to_settings_file()


func _on_h_slider_general_volume_value_changed(value):
	Global.get_config().audio_general_volume = value
	AudioSettings.apply_general_volume_settings()
	Global.get_config().save_to_settings_file()


func _on_radio_selector_ui_zoom_select_item(_index, item):
	var options := GraphicSettings.get_ui_zoom_available(get_window())
	var current_ui_zoom: String = item
	if not options.has(current_ui_zoom):
		current_ui_zoom = "Max"
	Global.get_config().ui_zoom = options[current_ui_zoom]
	GraphicSettings.apply_ui_zoom(get_window())
	Global.get_config().save_to_settings_file()


func _on_radio_selector_select_item(index, _item):
	Global.get_config().limit_fps = index
	GraphicSettings.apply_fps_limit()
	Global.get_config().save_to_settings_file()


func _on_radio_selector_skybox_select_item(index, _item):
	Global.get_config().skybox = index
	Global.get_config().save_to_settings_file()


func _on_radio_selector_shadow_select_item(index, _item):
	Global.get_config().shadow_quality = index
	Global.get_config().save_to_settings_file()


# gdlint:ignore = async-function-name
func _on_radio_selector_windowed_select_item(index, _item):
	Global.get_config().window_mode = index
	GraphicSettings.apply_window_config()
	await get_tree().process_frame
	refresh_zooms()


func _on_radio_selector_aa_select_item(index, _item):
	Global.get_config().anti_aliasing = index
	Global.get_config().save_to_settings_file()


func _on_radio_selector_graphic_profile_select_item(index, _item):
	Global.get_config().graphic_profile = index

	match index:
		0:  # Performance
			Global.get_config().anti_aliasing = 0  # off
			Global.get_config().shadow_quality = 0  # disabled
			Global.get_config().skybox = 0  # low
			Global.get_config().texture_quality = 0  # low
		1:  # Balanced
			Global.get_config().anti_aliasing = 1  # x2
			Global.get_config().shadow_quality = 1  # normal
			Global.get_config().skybox = 1  # medium
			Global.get_config().texture_quality = 1  # medium
		2:  # Quality
			Global.get_config().anti_aliasing = 3  # x8
			Global.get_config().shadow_quality = 2  # high quality
			Global.get_config().skybox = 2  # high
			Global.get_config().texture_quality = 2  # high
		3:  # Custom
			pass

	refresh_graphic_settings()
	Global.get_config().save_to_settings_file()


func _on_h_slider_music_volume_value_changed(value):
	Global.get_config().audio_music_volume = value
	AudioSettings.apply_music_volume_settings()
	Global.get_config().save_to_settings_file()


func _on_radio_selector_texture_quality_select_item(index, item):
	Global.get_config().texture_quality = index
	Global.get_config().save_to_settings_file()
