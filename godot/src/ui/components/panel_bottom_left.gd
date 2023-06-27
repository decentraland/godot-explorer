extends Control

signal request_change_realm(realm_string: String)
signal request_change_scene_radius(new_value: int)
signal request_pause_scenes(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)

@onready var panel_console = $HFlowContainer/Panel_Console
@onready var panel_realm = $HFlowContainer/Panel_Realm
@onready var panel_preview = $HFlowContainer/Panel_Preview

@onready var button_realm = $HFlowContainer/VFlowContainer_Tabs/Button_Realm
@onready var button_console = $HFlowContainer/VFlowContainer_Tabs/Button_Console
@onready var button_collapse = $HFlowContainer/VFlowContainer_Tabs/Button_Collapse
@onready var button_preview = $HFlowContainer/VFlowContainer_Tabs/Button_Preview

@onready var h_slider_scene_radius = $HFlowContainer/Panel_Realm/HSlider_SceneRadius
@onready var label_scene_radius_value = $HFlowContainer/Panel_Realm/Label_SceneRadiusValue
@onready var option_button_realm = $HFlowContainer/Panel_Realm/OptionButton_Realm
@onready var check_button_pause = $HFlowContainer/Panel_Realm/CheckButton_Pause
@onready var rich_text_label_console = $HFlowContainer/Panel_Console/RichTextLabel_Console

const SceneLogLevel := {
	Log = 1,
	SceneError = 2,
	SystemError = 3,
}

var tabs: Dictionary = {}
var panels_collapsed = false


func _ready():
	tabs = {
		"console": {"panel": panel_console, "button": button_console},
		"realm": {"panel": panel_realm, "button": button_realm},
		"preview": {"panel": panel_preview, "button": button_preview}
	}
	button_collapse.button_pressed = false
	_on_button_collapse_pressed()

	_on_button_tab_pressed("realm")
	set_ws_state(false)
	

func _on_button_tab_pressed(tab_id: String):
	for tab in tabs.values():
		tab.panel.hide()

	if not panels_collapsed:
		tabs[tab_id].panel.show()


func _on_button_collapse_pressed():
	panels_collapsed = button_collapse.button_pressed

	for tab in tabs.values():
		tab.panel.hide()
		tab.button.visible = not panels_collapsed

	if not panels_collapsed:
		for tab in tabs.values():
			if tab.button.disabled:
				tab.panel.show()


func _on_check_button_pause_pressed():
	emit_signal("request_pause_scenes", check_button_pause.button_pressed)


func _on_option_button_realm_item_selected(index):
	emit_signal("request_change_realm", option_button_realm.get_item_text(index))


func _on_h_slider_scene_radius_drag_ended(value_changed):
	if value_changed:
		emit_signal("request_change_scene_radius", h_slider_scene_radius.value)
		label_scene_radius_value.text = str(h_slider_scene_radius.value)


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


func _on_button_clear_console_pressed():
	rich_text_label_console.clear()

@onready var label_ws_state = $HFlowContainer/Panel_Preview/Label_WsState
@onready var line_edit_preview_url = $HFlowContainer/Panel_Preview/LineEdit_PreviewUrl

var preview_ws = WebSocketPeer.new()
var _preview_connect_to_url: String = ""
var _dirty_closed: bool = false
var _dirty_connected: bool = false

func set_ws_state(connected: bool) -> void:
	if connected:
		label_ws_state.text = "Connected"
		label_ws_state.add_theme_color_override("font_color", Color.FOREST_GREEN)
	else:
		label_ws_state.text = "Disconnected"
		label_ws_state.add_theme_color_override("font_color", Color.RED)

func _process(delta):
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
						print("preview-ws > update of ", scene_type, " with id '",scene_id , "'")
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
			print("preview-ws > closed with code: %d, reason %s. Clean: %s" % [code, reason, code != -1])
			_dirty_closed = false
		
		if not _preview_connect_to_url.is_empty():
			preview_ws.connect_to_url(_preview_connect_to_url)
			print("preview-ws > connecting to ", _preview_connect_to_url)
			_preview_connect_to_url = ""
			_dirty_connected = true


func _on_button_connect_preview_pressed():
	_preview_connect_to_url = line_edit_preview_url.text.to_lower().replace("http://", "ws://").replace("https://", "wss://")
