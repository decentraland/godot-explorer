extends Control

signal request_debug_panel(enabled: bool)
signal request_pause_scenes(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)

enum SceneLogLevel {
	LOG = 1,
	SCENE_ERROR = 2,
	SYSTEM_ERROR = 3,
}

const CACHE_SIZE_MB: Array[int] = [1024, 2048, 4096]

var preview_ws = WebSocketPeer.new()
var _preview_connect_to_url: String = ""
var _dirty_closed: bool = false
var _dirty_connected: bool = false

@onready var container_gameplay: VBoxContainer = %VBoxContainer_Gameplay
@onready var container_graphics: VBoxContainer = %VBoxContainer_Graphics
@onready var container_advanced: VBoxContainer = %VBoxContainer_Advanced
@onready var container_audio: VBoxContainer = %VBoxContainer_Audio
@onready var container_account: VBoxContainer = %VBoxContainer_Account
@onready var container_storage: VBoxContainer = %VBoxContainer_Storage
@onready var v_box_container_sections: VBoxContainer = %VBoxContainer_Sections
@onready var button_back_to_explorer: Button = %Button_BackToExplorer

#Storage items:
@onready var dropdown_list_max_cache_size: DropdownList = %DropdownList_MaxCacheSize
@onready var label_current_cache_value: Label = %Label_CurrentCacheValue
@onready var progress_bar_current_cache_size: ProgressBar = %ProgressBar_CurrentCacheSize
@onready var button_clear_cache: Button = %Button_ClearCache

@onready var h_slider_skybox_time: HSlider = %HSlider_SkyboxTime
@onready var label_skybox_time: Label = %Label_SkyboxTime
@onready
var check_button_submit_message_closes_chat: CheckButton = %CheckButton_SubmitMessageClosesChat
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
@onready
var dynamic_skybox: HBoxContainer = $ColorRect_Content/MarginContainer/MarginContainer/ScrollContainer/VBoxContainer/VBoxContainer_Graphics/VBoxContainer/SectionVisual/VBoxContainer/DynamicSkybox
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
@onready var content_scroll_container: ScrollContainer = %ContentScrollContainer
@onready var line_edit_custom_preview_url: LineEditCustom = %LineEditCustom_WebSocket
@onready var process_tick_quota: SettingsSlider = %ProcessTickQuota
@onready var check_button_raycast_debugger: CheckButton = %CheckButton_RaycastDebugger
@onready var dropdown_list_realm: DropdownList = %DropdownList_Realm

@onready var button_graphics: Button = %Button_Graphics
@onready var button_audio: Button = %Button_Audio
@onready var button_gameplay: Button = %Button_Gameplay
@onready var button_account: Button = %Button_Account
@onready var button_storage: Button = %Button_Storage
@onready var button_developer: Button = %Button_Developer

@onready var tabs_scroll_container: ScrollContainer = %TabsScrollContainer
@onready var dropdown_list_graphic_profiles: DropdownList = %DropdownList_GraphicProfiles
@onready var dropdown_list_custom_skybox: DropdownList = %DropdownList_CustomSkybox


func _ready():
	button_back_to_explorer.hide()
	#button_developer.visible = !Global.is_production()
	button_graphics.set_pressed_no_signal(true)
	_on_button_graphics_pressed()

	# Preview URL: release focus when clicking outside, keep visible when keyboard opens, connect button
	line_edit_custom_preview_url.custom_focus_entered.connect(
		_on_line_edit_preview_url_focus_entered
	)
	line_edit_custom_preview_url.button_pressed.connect(_on_button_connect_preview_pressed)

	if Global.get_explorer():
		preview_viewport_container.show()
	else:
		preview_viewport_container.hide()

	# general
	check_button_submit_message_closes_chat.button_pressed = (
		Global.get_config().submit_message_closes_chat
	)

	dropdown_list_max_cache_size.add_item("1 GB", 0)
	dropdown_list_max_cache_size.add_item("2 GB", 1)
	dropdown_list_max_cache_size.add_item("4 GB", 2)
	var cache_index := clampi(Global.get_config().max_cache_size, 0, CACHE_SIZE_MB.size() - 1)
	dropdown_list_max_cache_size.select(cache_index)
	progress_bar_current_cache_size.max_value = CACHE_SIZE_MB[cache_index]
	dropdown_list_max_cache_size.item_selected.connect(
		_on_dropdown_list_max_cache_size_item_selected
	)

	# graphic
	var i = 0
	for profile in GraphicSettings.PROFILE_NAMES:
		if profile != "Custom":
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
	avatar_and_emotes_volume.value = Global.get_config().audio_avatar_and_emotes_volume
	mic_amplification.value = Global.get_config().audio_mic_amplification

	refresh_values()

	# Dev Tools
	dropdown_list_realm.add_item("mannakia.dcl.eth", 0)
	dropdown_list_realm.add_item("http://127.0.0.1:8000", 1)
	dropdown_list_realm.add_item("https://sdk-test-scenes.decentraland.org", 2)
	dropdown_list_realm.add_item(
		"https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest", 3
	)
	dropdown_list_realm.add_item("https://peer.decentraland.org", 4)
	dropdown_list_realm.add_item(
		"https://sdk-team-cdn.decentraland.org/ipfs/streaming-world-main", 5
	)
	dropdown_list_realm.add_item("https://peer.decentraland.org", 6)
	dropdown_list_realm.add_item("shibu.dcl.eth", 7)
	dropdown_list_realm.add_item(
		"https://leanmendoza.github.io/mannakia-dcl-scene/mannakia-dcl-scene", 8
	)
	dropdown_list_realm.add_item("https://sdilauro.github.io/dae-unit-tests/dae-unit-tests", 9)
	dropdown_list_realm.add_item("https://realm-provider.decentraland.org/main", 10)


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
	for child in v_box_container_sections.get_children():
		child.hide()
	control.show()


func _async_scroll_to_tab_button(button: Button) -> void:
	await get_tree().process_frame
	var scroll := tabs_scroll_container.scroll_horizontal
	var view_width := tabs_scroll_container.size.x
	var btn_left := button.position.x
	var btn_right := button.position.x + button.size.x
	var visible_left := float(scroll)
	var visible_right := float(scroll) + view_width
	var fully_visible := btn_left >= visible_left and btn_right <= visible_right
	if fully_visible:
		return
	var separation := 48.0
	var target_x := 0.0
	var h_bar := tabs_scroll_container.get_h_scroll_bar()
	var max_scroll := float(maxi(0, int(h_bar.max_value)) if h_bar else 0)
	var cut_left := btn_left < visible_left
	var cut_right := btn_right > visible_right
	if cut_left:
		target_x = btn_left - separation
	elif cut_right:
		target_x = btn_right - view_width + separation
	target_x = clamp(target_x, 0.0, max_scroll)
	var tween := create_tween()
	tween.tween_property(tabs_scroll_container, "scroll_horizontal", int(target_x), 0.2)


func _on_button_pressed():
	self.hide()


func _unhandled_input(event: InputEvent) -> void:
	# Release LineEdit focus when tapping outside so virtual keyboard closes
	if not is_visible_in_tree():
		return
	if not line_edit_custom_preview_url.has_focus():
		return
	var pos: Vector2
	if event is InputEventMouseButton and event.pressed:
		pos = event.global_position
	elif event is InputEventScreenTouch and event.pressed:
		pos = event.position
	else:
		return
	if not line_edit_custom_preview_url.get_global_rect().has_point(pos):
		line_edit_custom_preview_url.release_focus()


# gdlint:ignore = async-function-name
func _on_line_edit_preview_url_focus_entered() -> void:
	# After a short delay (keyboard opening), scroll so the LineEdit stays visible
	await get_tree().create_timer(0.35).timeout
	if (
		not is_instance_valid(line_edit_custom_preview_url)
		or not line_edit_custom_preview_url.has_focus()
	):
		return
	var content_node: Control = content_scroll_container.get_child(0)
	var scroll_y: float = content_scroll_container.scroll_vertical
	var view_h: float = content_scroll_container.size.y
	var line_edit_global_top: float = line_edit_custom_preview_url.global_position.y
	var content_global_top: float = content_node.global_position.y
	var line_edit_y_in_content: float = line_edit_global_top - content_global_top + scroll_y
	var line_edit_h: float = line_edit_custom_preview_url.size.y
	var padding: float = 20.0
	if line_edit_y_in_content < scroll_y + padding:
		content_scroll_container.scroll_vertical = maxf(0, line_edit_y_in_content - padding)
	elif line_edit_y_in_content + line_edit_h > scroll_y + view_h - padding:
		var v_bar = content_scroll_container.get_v_scroll_bar()
		var max_scroll: float = v_bar.max_value if v_bar else 0.0
		content_scroll_container.scroll_vertical = minf(
			max_scroll, line_edit_y_in_content + line_edit_h - view_h + padding
		)


# gdlint:ignore = async-function-name
func _on_button_clear_cache_pressed():
	# Clean the content cache folder
	Global.content_provider.clear_cache_folder()
	await get_tree().process_frame
	_update_current_cache_size()


func _on_checkbox_fps_toggled(button_pressed):
	Global.get_config().show_fps = button_pressed


func refresh_values():
	process_tick_quota.value = Global.get_config().process_tick_quota_ms
	if is_instance_valid(Global.raycast_debugger):
		check_button_raycast_debugger.set_pressed_no_signal(true)


func set_ws_state(connected: bool) -> void:
	if connected:
		line_edit_custom_preview_url.set_description_text_and_color("Connected", Color.FOREST_GREEN)
	else:
		line_edit_custom_preview_url.set_description_text_and_color("Disconnected", Color.RED)


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
	set_preview_url(line_edit_custom_preview_url.get_text())


func set_preview_url(url: String) -> void:
	_preview_connect_to_url = url.to_lower().replace("http://", "ws://").replace(
		"https://", "wss://"
	)


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


func _on_dropdown_list_max_cache_size_item_selected(index: int) -> void:
	Global.get_config().max_cache_size = index
	GeneralSettings.apply_max_cache_size()
	progress_bar_current_cache_size.max_value = CACHE_SIZE_MB[index]
	_update_current_cache_size()
	Global.get_config().save_to_settings_file()


func _update_current_cache_size():
	var current_size_mb = roundf(
		float(Global.content_provider.get_cache_folder_total_size()) / 1000.0 / 1000.0
	)
	if current_size_mb >= 1024.0:
		label_current_cache_value.text = "%.1f GB" % (current_size_mb / 1024.0)
	elif current_size_mb > 0.0:
		label_current_cache_value.text = "%.1f MB" % current_size_mb
	else:
		label_current_cache_value.text = "0 MB"
	progress_bar_current_cache_size.value = current_size_mb
	button_clear_cache.disabled = current_size_mb == 0


func _on_container_storage_visibility_changed():
	_update_current_cache_size()


func _on_check_button_dynamic_skybox_toggled(toggled_on: bool) -> void:
	dropdown_list_custom_skybox.disabled = toggled_on
	if toggled_on:
		dropdown_list_custom_skybox.select(-1)
	else:
		dropdown_list_custom_skybox.select(3)
	if Global.get_config().dynamic_skybox != toggled_on:
		Global.get_config().dynamic_skybox = toggled_on
		Global.get_config().save_to_settings_file()


func _on_check_button_submit_message_closes_chat_toggled(toggled_on: bool) -> void:
	if Global.get_config().submit_message_closes_chat != toggled_on:
		Global.get_config().submit_message_closes_chat = toggled_on
		Global.get_config().save_to_settings_file()


func _on_button_developer_pressed() -> void:
	show_control(container_advanced)
	_async_scroll_to_tab_button(button_developer)


func _on_button_graphics_pressed() -> void:
	show_control(container_graphics)
	_async_scroll_to_tab_button(button_graphics)


func _on_button_gameplay_pressed() -> void:
	show_control(container_gameplay)
	_async_scroll_to_tab_button(button_gameplay)


func _on_button_audio_pressed():
	show_control(container_audio)
	_async_scroll_to_tab_button(button_audio)


func _on_button_account_pressed() -> void:
	show_control(container_account)
	_async_scroll_to_tab_button(button_account)


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
			print("Active - Current: %s" % profile_name)
		"Cooldown":
			var remaining := int(manager.get_cooldown_remaining())
			print("Cooldown (%ds) - Current: %s" % [remaining, profile_name])


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


func _on_button_privacy_policy_pressed() -> void:
	const PRIVACY_POLICY_URL = "https://decentraland.org/privacy/"
	Global.open_url(PRIVACY_POLICY_URL)


func _on_button_content_policy_pressed() -> void:
	const CONTENT_POLICY_URL = "https://decentraland.org/content/"
	Global.open_url(CONTENT_POLICY_URL)


func _on_button_terms_of_service_pressed() -> void:
	const TERMS_OF_USE_URL = "https://decentraland.org/terms/"
	Global.open_url(TERMS_OF_USE_URL)


func _on_button_discord_pressed() -> void:
	const DISCORD_URL = "https://discord.com/channels/417796904760639509/1446513533893218465"
	Global.open_url(DISCORD_URL)


func _on_button_storage_pressed() -> void:
	show_control(container_storage)
	_async_scroll_to_tab_button(%Button_Storage)


func _on_button_back_to_explorer_pressed() -> void:
	if Global.get_explorer():
		Global.close_menu.emit()
		Global.set_orientation_landscape()


func _on_visibility_changed() -> void:
	if is_node_ready() and is_inside_tree() and is_visible_in_tree():
		Global.set_orientation_portrait()
		if Global.get_explorer():
			if button_back_to_explorer:
				button_back_to_explorer.show()


func _on_check_button_scene_processing_paused_toggled(toggled_on: bool) -> void:
	emit_signal("request_pause_scenes", toggled_on)


func _on_check_button_raycast_debugger_toggled(toggled_on: bool) -> void:
	Global.set_raycast_debugger_enable(toggled_on)


func _on_check_button_scene_logs_enabled_toggled(toggled_on: bool) -> void:
	request_debug_panel.emit(toggled_on)


func _on_dropdown_list_realm_item_selected(index: int) -> void:
	var realm_text := dropdown_list_realm.get_item_text(index)
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		Global.realm.async_set_realm(realm_text)
		explorer.hide_menu()
		Global.close_menu.emit()
		Global.set_orientation_landscape()
	else:
		Global.close_menu.emit()
		Global.get_config().last_realm_joined = realm_text
		Global.get_config().last_parcel_position = Vector2i.ZERO
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")


func _on_avatar_and_emotes_volume_value_changed(value: float) -> void:
	Global.get_config().audio_avatar_and_emotes_volume = value
	AudioSettings.apply_avatar_and_emotes_volume_settings()
	Global.get_config().save_to_settings_file()
