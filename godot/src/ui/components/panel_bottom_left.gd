extends Control

signal request_change_realm(realm_string: String)
signal request_change_scene_radius(new_value: int)
signal request_pause_scenes(enabled: bool)

@onready var panel_console = $HFlowContainer/Panel_Console
@onready var panel_settings = $HFlowContainer/Panel_Settings

@onready var button_settings = $HFlowContainer/VFlowContainer_Tabs/Button_Settings
@onready var button_console = $HFlowContainer/VFlowContainer_Tabs/Button_Console
@onready var button_collapse = $HFlowContainer/VFlowContainer_Tabs/Button_Collapse

@onready var h_slider_scene_radius = $HFlowContainer/Panel_Settings/HSlider_SceneRadius
@onready var label_scene_radius_value = $HFlowContainer/Panel_Settings/Label_SceneRadiusValue
@onready var option_button_realm = $HFlowContainer/Panel_Settings/OptionButton_Realm
@onready var check_button_pause = $HFlowContainer/Panel_Settings/CheckButton_Pause
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
		"settings": {"panel": panel_settings, "button": button_settings}
	}
	button_collapse.button_pressed = false
	_on_button_collapse_pressed()

	_on_button_tab_pressed("settings")


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


func _on_console_add(scene_id: int, level: int, timestamp: float, text: String) -> void:
	var color := Color.BLACK
	match level:
		SceneLogLevel.Log:
			color = Color.DARK_SLATE_BLUE
		SceneLogLevel.SceneError:
			color = Color.DARK_RED
		SceneLogLevel.SystemError:
			color = Color.RED

	timestamp = round(timestamp * 100.0) / 100.0
	var msg = "(" + str(timestamp) + ") Scene " + str(scene_id) + " > " + text
	rich_text_label_console.push_color(color)
	rich_text_label_console.add_text(msg)
	rich_text_label_console.pop()
	rich_text_label_console.newline()
