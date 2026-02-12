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

@onready var container_gameplay: Control = %Container_Gameplay
@onready var container_graphics: Control = %VBoxContainer_Graphics
@onready var container_advanced: Control = %VBoxContainer_Advanced
@onready var container_audio: Control = %VBoxContainer_Audio
@onready var container_account: VBoxContainer = %VBoxContainer_Account

#General items:
@onready var text_edit_cache_path = %TextEdit_CachePath
@onready var label_current_cache_size = %Label_CurrentCacheSize
@onready var radio_selector_max_cache_size = %RadioSelector_MaxCacheSize

@onready var h_slider_skybox_time: HSlider = %HSlider_SkyboxTime
@onready var label_skybox_time: Label = %Label_SkyboxTime
@onready var check_button_submit_message_closes_chat: CheckButton = %CheckButton_SubmitMessageClosesChat
@onready var preview_camera_3d: Camera3D = %PreviewCamera3D
@onready var preview_viewport_container: SubViewportContainer = %PreviewViewportContainer

#Audio items
@onready var general_volume: SettingsSlider = %GeneralVolume
@onready var scene_volume: SettingsSlider = %SceneVolume
@onready var ui_volume: SettingsSlider = %UIVolume
@onready var music_volume: SettingsSlider = %MusicVolume
@onready var avatar_and_emotes_volume: SettingsSlider = %AvatarAndEmotesVolume
@onready var voice_chat_volume: SettingsSlider = %VoiceChatVolume
@onready var mic_amplification: SettingsSlider = %MicAmplification


#Graphics items:
@onready var h_slider_rendering_scale = %HSlider_Resolution3DScale
@onready var radio_selector_ui_zoom = %RadioSelector_UiZoom

@onready var v_box_container_windowed = %VBoxContainer_Windowed
@onready var radio_selector_windowed = %RadioSelector_Windowed

@onready var box_container_custom = %VBoxContainer_Custom

@onready var radio_selector_graphic_profile = %RadioSelector_GraphicProfile

# Dynamic graphics toggle
@onready var dynamic_graphics_container: HBoxContainer = %DynamicGraphics
@onready var check_button_dynamic_graphics: CheckButton = %CheckButton_DynamicGraphics

# Dynamic graphics toggle
@onready var dynamic_skybox: HBoxContainer = $ColorRect_Content/MarginContainer/MarginContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/VBoxContainer/SectionVisual/VBoxContainer/DynamicSkybox
@onready var check_button_dynamic_skybox: CheckButton = %CheckButton_DynamicSkybox

@onready var radio_selector_texture_quality = %RadioSelector_TextureQuality
@onready var radio_selector_skybox = %RadioSelector_Skybox
@onready var radio_selector_shadow = %RadioSelector_Shadow
@onready var radio_selector_bloom = %RadioSelector_Bloom
@onready var radio_selector_aa = %RadioSelector_AA

@onready var radio_selector_limit_fps = %RadioSelector_LimitFps
@onready var container_limit_fps = %LimitFps
@onready var container_resolution_3d_scale = %Resolution3DScale

#Advanced items:
@onready var option_button_realm = %OptionButton_Realm
@onready var line_edit_preview_url = %LineEdit_PreviewUrl
@onready var label_ws_state = %Label_WsState

@onready var h_slider_process_tick_quota = %HSlider_ProcessTickQuota
@onready var label_process_tick_quota_value = %Label_ProcessTickQuotaValue

@onready var check_box_raycast_debugger = %CheckBox_RaycastDebugger
@onready var button_test_notification = %Button_TestNotification

@onready var button_general: Button = %Button_General
@onready var button_graphics: Button = %Button_Graphics
@onready var button_audio: Button = %Button_Audio
#@onready var button_developer: Button = %Button_Developer
@onready var dropdown_list_graphic_profiles: DropdownList = %DropdownList_GraphicProfiles
@onready var dropdown_list_custom_skybox: DropdownList = %DropdownList_CustomSkybox


func _ready():
	#button_developer.visible = !Global.is_production()
	button_graphics.set_pressed_no_signal(true)
	_on_button_graphics_pressed()

	if Global.get_explorer():
		preview_viewport_container.show()
	else:
		preview_viewport_container.hide()

	# general
	text_edit_cache_path.text = Global.get_config().local_content_dir
	radio_selector_max_cache_size.selected = Global.get_config().max_cache_size
	check_button_submit_message_closes_chat.button_pressed = Global.get_config().submit_message_closes_chat

	# graphic
	var i = 0
	for profile in GraphicSettings.PROFILE_NAMES:
		dropdown_list_graphic_profiles.add_item(profile, i)
		i += 1
	_setup_dynamic_graphics()
	_update_dynamic_graphics_status()
	refresh_graphic_settings()

	var j = 0
	for profile in GraphicSettings.SKYBOX_TIME_NAMES:
		dropdown_list_custom_skybox.add_item(profile.name, j)
		j += 1

	if Global.get_config().dynamic_skybox:
		check_button_dynamic_skybox.button_pressed = true
		dropdown_list_custom_skybox.select(-1)
	else:
		check_button_dynamic_skybox.button_pressed = false
		var current_skybox_time: int = Global.get_config().skybox_time
		for k in range(GraphicSettings.SKYBOX_TIME_NAMES.size()):
			if GraphicSettings.SKYBOX_TIME_NAMES[k].secs == current_skybox_time:
				dropdown_list_custom_skybox.select(k)
				break


	# volume
	general_volume.value = Global.get_config().audio_general_volume
	scene_volume.value = Global.get_config().audio_scene_volume
	voice_chat_volume.value = Global.get_config().audio_voice_chat_volume
	ui_volume.value = Global.get_config().audio_ui_volume
	music_volume.value = Global.get_config().audio_music_volume
	mic_amplification.value = Global.get_config().audio_mic_amplification

	refresh_values()


func refresh_graphic_settings():
	var graphic_profile = Global.get_config().graphic_profile
	var is_custom_profile: bool = graphic_profile == ConfigData.PROFILE_CUSTOM

	# We only show the custom settings if the graphic profile is custom
	box_container_custom.visible = is_custom_profile
	dropdown_list_graphic_profiles.select(graphic_profile)

	# Hide FPS limit and 3D resolution scale when using preset profiles
	# These are controlled by the profile, not user-configurable
	container_limit_fps.visible = is_custom_profile
	container_resolution_3d_scale.visible = is_custom_profile

	if Global.is_mobile():
		v_box_container_windowed.hide()
	else:
		radio_selector_windowed.selected = Global.get_config().window_mode

	# Maps limit_fps config to radio_selector_limit_fps index
	const INVERSE_LIMIT_FPS_MAPPING: Dictionary[int, int] = {
		ConfigData.FpsLimitMode.VSYNC: 0,
		ConfigData.FpsLimitMode.NO_LIMIT: 1,
		ConfigData.FpsLimitMode.FPS_30: 3,
		ConfigData.FpsLimitMode.FPS_60: 4,
		ConfigData.FpsLimitMode.FPS_18: 2,
	}

	radio_selector_limit_fps.selected = INVERSE_LIMIT_FPS_MAPPING.get(
		Global.get_config().limit_fps, 0
	)
	radio_selector_texture_quality.selected = Global.get_config().texture_quality
	radio_selector_skybox.selected = Global.get_config().skybox
	radio_selector_shadow.selected = Global.get_config().shadow_quality
	radio_selector_bloom.selected = Global.get_config().bloom_quality
	radio_selector_aa.selected = Global.get_config().anti_aliasing

	h_slider_rendering_scale.value = Global.get_config().resolution_3d_scale
	refresh_zooms()


func show_control(control: Control):
	container_gameplay.hide()
	container_graphics.hide()
	container_audio.hide()
	container_advanced.hide()
	container_account.hide()

	control.show()


func _on_button_pressed():
	self.hide()


# gdlint:ignore = async-function-name
func _on_button_clear_cache_pressed():
	# Clean the content cache folder
	Global.content_provider.clear_cache_folder()
	await get_tree().process_frame
	_update_current_cache_size()


func _on_checkbox_fps_toggled(button_pressed):
	Global.get_config().show_fps = button_pressed


func refresh_values():
	h_slider_process_tick_quota.set_value_no_signal(Global.get_config().process_tick_quota_ms)
	label_process_tick_quota_value.text = str(Global.get_config().process_tick_quota_ms)

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

	var new_items: Array[String] = []
	for ui_zoom_option in options.keys():
		new_items.push_back(ui_zoom_option)
		if options[ui_zoom_option] == get_window().content_scale_factor:
			selected_index = i
		i += 1
	if selected_index == -1:
		selected_index = i - 1

	# Assign items array to trigger _refresh_list() and create children
	radio_selector_ui_zoom.items = new_items
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
	# Maps radio_selector_limit_fps index to limit_fps config
	const LIMIT_FPS_MAPPING: Dictionary[int, int] = {
		0: ConfigData.FpsLimitMode.VSYNC,
		1: ConfigData.FpsLimitMode.NO_LIMIT,
		2: ConfigData.FpsLimitMode.FPS_18,
		3: ConfigData.FpsLimitMode.FPS_30,
		4: ConfigData.FpsLimitMode.FPS_60
	}

	Global.get_config().limit_fps = LIMIT_FPS_MAPPING[index]
	Global.get_config().save_to_settings_file()


func _on_radio_selector_skybox_select_item(index, _item):
	Global.get_config().skybox = index
	Global.get_config().save_to_settings_file()


func _on_radio_selector_shadow_select_item(index, _item):
	Global.get_config().shadow_quality = index
	Global.get_config().save_to_settings_file()


func _on_radio_selector_bloom_select_item(index, _item):
	Global.get_config().bloom_quality = index
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
	# Use centralized profile application (handles all parameters)
	# 0: Very Low, 1: Low, 2: Medium, 3: High, 4: Custom
	if index < ConfigData.PROFILE_CUSTOM:
		GraphicSettings.apply_graphic_profile(index)
	else:
		Global.get_config().graphic_profile = index  # Custom - keep current settings

	refresh_graphic_settings()
	Global.get_config().save_to_settings_file()

	# Notify dynamic graphics manager of manual profile change
	Global.dynamic_graphics_manager.on_manual_profile_change(index)


func _on_h_slider_music_volume_value_changed(value):
	Global.get_config().audio_music_volume = value
	AudioSettings.apply_music_volume_settings()
	Global.get_config().save_to_settings_file()


func _on_radio_selector_texture_quality_select_item(index, _item):
	Global.get_config().texture_quality = index
	Global.get_config().save_to_settings_file()


func _on_radio_selector_max_cache_size_select_item(index, _item):
	Global.get_config().max_cache_size = index
	GeneralSettings.apply_max_cache_size()
	Global.get_config().save_to_settings_file()


func _update_current_cache_size():
	var current_size_mb = roundf(
		float(Global.content_provider.get_cache_folder_total_size()) / 1000.0 / 1000.0
	)
	label_current_cache_size.text = "(current size: %dmb)" % int(current_size_mb)


func _on_container_general_visibility_changed():
	_update_current_cache_size()


func _on_check_button_dynamic_skybox_toggled(toggled_on: bool) -> void:
	dropdown_list_custom_skybox.disabled = toggled_on
	if toggled_on:
		dropdown_list_custom_skybox.select(-1)
	if Global.get_config().dynamic_skybox != toggled_on:
		Global.get_config().dynamic_skybox = toggled_on
		Global.get_config().save_to_settings_file()


func _on_check_button_submit_message_closes_chat_toggled(toggled_on: bool) -> void:
	if Global.get_config().submit_message_closes_chat != toggled_on:
		Global.get_config().submit_message_closes_chat = toggled_on
		Global.get_config().save_to_settings_file()


func _on_button_developer_pressed() -> void:
	show_control(container_advanced)


func _on_button_graphics_pressed() -> void:
	show_control(container_graphics)


func _on_button_gameplay_pressed() -> void:
	show_control(container_gameplay)


func _on_button_audio_pressed():
	show_control(container_audio)


func _on_button_account_pressed() -> void:
	show_control(container_account)


func _on_button_delete_account_pressed() -> void:
	Global.delete_account.emit()


func _on_button_test_notification_pressed() -> void:
	# Test notification with emojis and accents in both title and body
	# This will test if iOS can display both correctly
	var test_title = "🎉 Notificación de Prueba 🎉"
	var test_body = "Esta es una notificación de prueba con emojis 🚀 y acentos: áéíóú ÁÉÍÓÚ ñ Ñ"
	var notification_id = "test_notification_" + str(Time.get_unix_time_from_system())
	var delay_seconds = 5  # Show notification in 5 seconds

	if NotificationsManager.schedule_local_notification(
		notification_id, test_title, test_body, delay_seconds
	):
		print(
			(
				"Test notification scheduled: id=%s, title=%s, body=%s"
				% [notification_id, test_title, test_body]
			)
		)
		print(
			"Expected: Emojis and accents should be preserved. If they show as symbols, enable sanitization."
		)
	else:
		printerr("Failed to schedule test notification")


func _on_button_report_bug_pressed() -> void:
	var form_id = "1FAIpQLScWjnb3Ya7yV8xFn0R-yf_SMejzBGDiDTZbHaddOFEmJwAM6g"
	var base_url = "https://docs.google.com/forms/d/e/" + form_id + "/viewform"

	var params = []
	var platform = "desktop"
	var device_brand = ""
	var device_model = ""
	var os_version = OS.get_name()
	var app_version = DclGlobal.get_version()
	var environment = ""
	if DclAndroidPlugin.is_available():
		var android_singleton = Engine.get_singleton("dcl-godot-android")
		if android_singleton:
			var device_info = android_singleton.getMobileDeviceInfo()
			device_brand = device_info.get("device_brand", "")
			device_model = device_info.get("device_model", "")
			os_version = device_info.get("os_version", OS.get_name())
		platform = "mobile"
	elif DclIosPlugin.is_available():
		var ios_singleton = Engine.get_singleton("DclGodotiOS")
		if ios_singleton:
			var device_info = ios_singleton.get_mobile_device_info()
			device_brand = device_info.get("device_brand", "")
			device_model = device_info.get("device_model", "")
			os_version = device_info.get("os_version", OS.get_name())
		platform = "mobile"

	params.append("entry.908487542=" + os_version.uri_encode())
	params.append("entry.1825988508=" + app_version.uri_encode())
	params.append("entry.902053507=" + platform.uri_encode())
	params.append("entry.983493489=" + Global.player_identity.get_address_str().uri_encode())
	params.append("entry.519686692=" + RenderingServer.get_video_adapter_name().uri_encode())
	params.append("entry.69678037=" + Global.session_id.uri_encode())

	if "dev" in app_version:
		environment = "develop"
	else:
		environment = "production"

	params.append("entry.1045647501=" + environment.uri_encode())

	if device_brand != "":
		params.append("entry.942533991=" + device_brand.uri_encode())

	if device_model != "":
		params.append("entry.264855991=" + device_model.uri_encode())

	var url = base_url
	if params.size() > 0:
		url += "?" + "&".join(params)

	Global.open_url(url)


func _on_button_open_user_data_pressed() -> void:
	var user_data_path = OS.get_user_data_dir()
	print("Opening user data folder: ", user_data_path)

	# On mobile, we can't open the file explorer directly, so we copy the path to clipboard
	if Global.is_mobile():
		DisplayServer.clipboard_set(user_data_path)
		print("User data path copied to clipboard: ", user_data_path)
	else:
		# On desktop (Windows, macOS, Linux), open the file explorer
		OS.shell_open(user_data_path)


func _on_button_report_content_pressed() -> void:
	var form_id = "1FAIpQLSdD31D0GKROyxmrvM-KVStqdhyqF430crjaTtpemEiAqCHQbg"
	var base_url = "https://docs.google.com/forms/d/e/" + form_id + "/viewform"

	var params = []

	var scene_name = ""
	if Global.scene_runner != null:
		var current_scene_id = Global.scene_runner.get_current_parcel_scene_id()
		if current_scene_id >= 0:
			scene_name = Global.scene_runner.get_scene_title(current_scene_id)

	var current_position = Global.get_config().last_parcel_position
	var scene_info = "%s (%d, %d)" % [scene_name, current_position.x, current_position.y]

	var wallet_id = Global.player_identity.get_address_str()

	params.append("entry.60289947=" + scene_info.uri_encode())
	params.append("entry.927432836=" + wallet_id.uri_encode())

	var url = base_url
	if params.size() > 0:
		url += "?" + "&".join(params)

	Global.open_url(url)


func _setup_dynamic_graphics() -> void:
	# Only show on mobile platforms
	dynamic_graphics_container.visible = Global.is_mobile()

	if not Global.is_mobile():
		return

	# Initialize checkbox state
	var is_enabled: bool = Global.get_config().dynamic_graphics_enabled
	check_button_dynamic_graphics.set_pressed_no_signal(is_enabled)
	dropdown_list_graphic_profiles.disabled = is_enabled
	# Update UI state
	_update_dynamic_graphics_status()

	# Connect to manager signal to update UI when profile changes dynamically
	Global.dynamic_graphics_manager.profile_change_requested.connect(
		func(_profile: int):
			refresh_graphic_settings()
			_update_dynamic_graphics_status()
	)


func _on_check_button_dynamic_graphics_toggled(toggled_on: bool) -> void:
	dropdown_list_graphic_profiles.disabled = toggled_on
	Global.get_config().dynamic_graphics_enabled = toggled_on
	Global.get_config().save_to_settings_file()

	# Enable/disable the dynamic graphics manager
	Global.dynamic_graphics_manager.set_enabled(toggled_on)

	# Update UI state
	_update_dynamic_graphics_status()


func _update_dynamic_graphics_status() -> void:
	if not Global.is_mobile():
		return

	var manager = Global.dynamic_graphics_manager
	if manager == null or not manager.is_enabled():
		#label_dynamic_graphics_status.text = ""
		return

	var current_profile: int = manager.get_current_profile()
	dropdown_list_graphic_profiles.select(current_profile)
	var state_name: String = manager.get_state_name()
	var profile_name: String = GraphicSettings.PROFILE_NAMES[current_profile]
	
	print(profile_name, state_name)
	
	match state_name:
		"Disabled":
			print("")
		"WarmingUp":
			var remaining := int(manager.get_warmup_remaining())
			print("Warming up... (%ds)" % remaining)
		"Monitoring":
			print( "Active - Current: %s" % profile_name)
		"Cooldown":
			var remaining := int(manager.get_cooldown_remaining())
			print((
				"Cooldown (%ds) - Current: %s" % [remaining, profile_name]
			))


func _on_dropdown_list_graphic_profiles_item_selected(index: int) -> void:
	# Use centralized profile application (handles all parameters)
	# 0: Very Low, 1: Low, 2: Medium, 3: High, 4: Custom
	if index < ConfigData.PROFILE_CUSTOM:
		GraphicSettings.apply_graphic_profile(index)
	else:
		Global.get_config().graphic_profile = index  # Custom - keep current settings

	refresh_graphic_settings()
	Global.get_config().save_to_settings_file()

	# Notify dynamic graphics manager of manual profile change
	Global.dynamic_graphics_manager.on_manual_profile_change(index)


func _on_dropdown_list_custom_skybox_item_selected(index: int) -> void:
	var time: int = GraphicSettings.SKYBOX_TIME_NAMES[index].secs
	if Global.get_config().skybox_time != time:
		Global.get_config().skybox_time = time
