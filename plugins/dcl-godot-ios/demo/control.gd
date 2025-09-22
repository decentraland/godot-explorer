extends Control

var dcl_godot_ios_plugin

func _ready():
	if Engine.has_singleton("DclGodotIOS"):
		print("iOS DclGodotIOS plugin found!")
		dcl_godot_ios_plugin = Engine.get_singleton("DclGodotIOS")
		dcl_godot_ios_plugin.print_version()
	else:
		print("iOS DclGodotIOS plugin is not available on this platform.")

func _on_button_pressed():
	dcl_godot_ios_plugin.print_version()


func _on_button_2_pressed():
	dcl_godot_ios_plugin.open_auth_url("https://decentraland.org/auth/")
