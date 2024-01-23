extends Node

signal loading_show_requested

@export var loading_screen: Control

var scenes_metadata_loaded: bool = false
var waiting_new_scene_load_report = true
var waiting_for_scenes = false


func _ready():
	Global.scene_fetcher.report_new_load.connect(_report_scene_new_load)


func _report_scene_new_load(done: bool):
	scenes_metadata_loaded = done
	waiting_for_scenes = done
	if done == false:  # start
		enable_loading_screen()

	waiting_new_scene_load_report = false


func enable_loading_screen():
	loading_screen.show()
	set_physics_process(true)
	scenes_metadata_loaded = false
	loading_screen.set_progress(0.0)
	waiting_new_scene_load_report = true
	loading_show_requested.emit()


func hide_loading_screen():
	set_physics_process(false)
	loading_screen.async_hide_loading_screen_effect()


func _physics_process(delta):
	if waiting_new_scene_load_report:
		return

	if scenes_metadata_loaded == false:
		# We fake 20% for the metadata loading in 2 seconds
		var new_progress = minf(loading_screen.progress + delta / 4.0 * 20.0, 20.0)
		loading_screen.set_progress(new_progress)
	elif waiting_for_scenes:
		# 20% to 100% is waiting for all scene runners hit frame 4 (all gltf are loaded)
		var scene_progress: int = 0
		for child in Global.scene_runner.get_children():
			if child is DclSceneNode:
				scene_progress += mini(child.get_last_tick_number(), 4)

		var current_progress: int = (
			int(float(scene_progress) / float(Global.scene_runner.get_child_count() * 4) * 80.0)
			+ 20
		)
		loading_screen.set_progress(current_progress)

		if current_progress == 100:
			hide_loading_screen()
