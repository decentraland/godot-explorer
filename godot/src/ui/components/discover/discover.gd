class_name Discover
extends Control

@onready var jump_in = %JumpIn


func _ready():
	UiSounds.install_audio_recusirve(self)
	jump_in.hide()


func on_item_pressed(data):
	jump_in.show()
	jump_in.set_data(data)


func _on_jump_in_jump_in(parcel_position, realm):
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		explorer.teleport_to(parcel_position, realm)
		jump_in.hide()
		explorer.hide_menu()
	else:
		Global.config.last_realm_joined = realm
		Global.config.last_parcel_position = parcel_position
		Global.config.add_place_to_last_places(parcel_position, realm)
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")


func _on_visibility_changed():
	if is_node_ready() and is_inside_tree() and is_visible_in_tree():
		%LastVisitGenerator.request_last_places()
