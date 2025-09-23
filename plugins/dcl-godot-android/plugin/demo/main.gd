extends Node

const URL = "https://decentraland.org/auth/requests/aec64171-3781-4927-b874-366ff367c292?targetConfigId=alternative"
const MESSAGE = ""

var _plugin_name = "dcl-godot-android"
var _android_plugin

func _ready():
	if Engine.has_singleton(_plugin_name):
		_android_plugin = Engine.get_singleton(_plugin_name)
	else:
		printerr("Couldn't find plugin " + _plugin_name)


func _on_button_custom_tab_pressed():
	if _android_plugin:
		_android_plugin.openCustomTabUrl(URL)
		_android_plugin.showMessage(MESSAGE)


func _on_button_web_kit_pressed():
	if _android_plugin:
		_android_plugin.openWebView(URL, MESSAGE)


func _on_button_web_kit_and_destroy_pressed():
	if _android_plugin:
		_android_plugin.openWebView(URL, MESSAGE)
		await get_tree().create_timer(5.0).timeout
		_android_plugin.closeWebView()
