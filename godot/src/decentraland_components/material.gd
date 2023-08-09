extends Node

@export var dcl_scene_id: int = -1

var waiting_albedo_texture = false
var waiting_alpha_texture = false
var waiting_emissive_texture = false
var waiting_bump_texture = false

var connected_to_content_loading = false


func connect_if_needed():
	var needed = (
		waiting_albedo_texture
		or waiting_alpha_texture
		or waiting_bump_texture
		or waiting_emissive_texture
	)
	if needed and not connected_to_content_loading:
		Global.content_manager.content_loading_finished.connect(self._on_content_loading_finished)
		connected_to_content_loading = true
	elif not needed and connected_to_content_loading:
		Global.content_manager.content_loading_finished.disconnect(
			self._on_content_loading_finished
		)
		connected_to_content_loading = false


func _on_content_loading_finished(hash):
	pass


func _set_albedo_texture(file_path: String):
	pass
