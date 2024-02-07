class_name Discover
extends Control

@onready var jump_in = %JumpIn

func _ready():
	jump_in.hide()

func on_item_pressed(data):
	jump_in.show()
	jump_in.set_data(data)


func _on_jump_in_jump_in(position, realm):
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		Global.realm.async_set_realm(realm)
		explorer.teleport_to(position)
		explorer.loading_ui.enable_loading_screen()
		jump_in.hide()
		explorer.hide_menu()
	else:
		Global.config.last_realm_joined = realm
		Global.config.last_parcel_position = position
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
