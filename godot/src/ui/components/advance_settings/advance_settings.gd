extends Control

@onready
var check_button_pause = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_Realm/HBoxContainer/CheckButton_Pause
@onready
var option_button_realm = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_Realm/HBoxContainer2/OptionButton_Realm
@onready
var line_edit_preview_url = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_Connection/HBoxContainer/LineEdit_PreviewUrl
@onready
var label_ws_state = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_Connection/HBoxContainer2/Label_WsState
@onready
var h_slider_process_tick_quota = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_ProcessTickQuota/HBoxContainer/HSlider_ProcessTickQuota
@onready
var label_process_tick_quota_value = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_ProcessTickQuota/HBoxContainer/Label_ProcessTickQuotaValue
@onready
var h_slider_scene_radius = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_SceneRadius/HBoxContainer/HSlider_SceneRadius
@onready
var label_scene_radius_value = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/VBoxContainer_SceneRadius/HBoxContainer/Label_SceneRadiusValue
@onready
var rich_text_label_console = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General2/Panel_Console/RichTextLabel_Console
@onready
var spin_box_gravity = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/HBoxContainer/HBoxContainer_Gravity/SpinBox_Gravity
@onready
var spin_box_jump_velocity = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/HBoxContainer/HBoxContainer_JumpVelocity/SpinBox_JumpVelocity
@onready
var spin_box_run_speed = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/HBoxContainer2/HBoxContainer_RunSpeed/SpinBox_RunSpeed
@onready
var spin_box_walk_speed = $VBoxContainer/ColorRect_Background/HBoxContainer/VBoxContainer_General/HBoxContainer2/HBoxContainer_WalkSpeed/SpinBox_WalkSpeed

var preview_ws = WebSocketPeer.new()
var _preview_connect_to_url: String = ""
var _dirty_closed: bool = false
var _dirty_connected: bool = false

signal request_pause_scenes(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)

const SceneLogLevel := {
	Log = 1,
	SceneError = 2,
	SystemError = 3,
}


func _ready():
	refresh_values()


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
	Global.realm.set_realm(option_button_realm.get_item_text(index))

func _on_h_slider_scene_radius_drag_ended(value_changed):
	if value_changed:
		Global.config.scene_radius = h_slider_scene_radius.value
		label_scene_radius_value.text = str(h_slider_scene_radius.value)
		Global.config.save_to_settings_file()


func _on_check_button_pause_toggled(button_pressed):
	emit_signal("request_pause_scenes", check_button_pause.button_pressed)


func _on_button_clear_console_pressed():
	rich_text_label_console.clear()


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
	_preview_connect_to_url = (
		line_edit_preview_url
		. text
		. to_lower()
		. replace("http://", "ws://")
		. replace("https://", "wss://")
	)


func _on_console_add(scene_title: String, level: int, timestamp: float, text: String) -> void:
	var color := Color.BLACK
	match level:
		SceneLogLevel.Log:
			color = Color.DARK_SLATE_BLUE
		SceneLogLevel.SceneError:
			color = Color.DARK_RED
		SceneLogLevel.SystemError:
			color = Color.RED

	timestamp = round(timestamp * 100.0) / 100.0
	var msg = "(" + str(timestamp) + ") " + scene_title + " > " + text
	rich_text_label_console.push_color(color)
	rich_text_label_console.add_text(msg)
	rich_text_label_console.pop()
	rich_text_label_console.newline()


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


func _on_button_copy_console_content_pressed():
	rich_text_label_console.select_all()
	var text = rich_text_label_console.get_parsed_text()
	DisplayServer.clipboard_set(text)
