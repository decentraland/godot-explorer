extends Node

signal loading_show_requested

@export var loading_screen: Control

var scenes_metadata_loaded: bool = false
var waiting_new_scene_load_report = true
var waiting_for_scenes = false
var wait_for: float = 0.0
var empty_timeout: float = 0.0
var pending_to_load: int = 0


func _ready():
	Global.scene_fetcher.report_scene_load.connect(_report_scene_load)


func _report_scene_load(done: bool, is_new_loading: bool, pending: int):
	scenes_metadata_loaded = done
	waiting_for_scenes = done
	if done == false and is_new_loading:  # start
		enable_loading_screen()

	waiting_new_scene_load_report = false
	empty_timeout = 2.0
	pending_to_load = pending


func enable_loading_screen():
	Global.content_provider.set_max_concurrent_downloads(32)
	loading_screen.show()
	set_physics_process(true)
	scenes_metadata_loaded = false
	loading_screen.set_progress(0.0)
	waiting_new_scene_load_report = true
	loading_show_requested.emit()
	wait_for = 1.0


func hide_loading_screen():
	Global.content_provider.set_max_concurrent_downloads(6)
	set_physics_process(false)
	loading_screen.async_hide_loading_screen_effect()


func _physics_process(delta):
	if wait_for > 0.0:
		wait_for -= delta
		return
	if waiting_new_scene_load_report:
		return

	if scenes_metadata_loaded == false:
		# We fake 20% for the metadata loading in 2 seconds
		var new_progress = minf(loading_screen.progress + delta / 4.0 * 20.0, 20.0)
		loading_screen.set_progress(new_progress)
	elif waiting_for_scenes:
		var scenes_loaded_count: int = Global.scene_runner.get_child_count()
		if scenes_loaded_count == 0 and pending_to_load == 0:
			empty_timeout -= delta
			if empty_timeout < 0.0:
				loading_screen.set_progress(100)
				hide_loading_screen()
			return

		# 20% to 100% is waiting for all scene runners hit frame 4 (all gltf are loaded)
		var scene_progress: float = 0
		for child in Global.scene_runner.get_children():
			var this_scene_progress: float = 0.0
			if child is DclSceneNode:
				var tick_number = mini(child.get_last_tick_number(), 4)
				if tick_number < 4:
					this_scene_progress = 0.2 * (float(tick_number) / 3.0)
					this_scene_progress += (
						0.8 * child.get_gltf_loading_progress() * (float(tick_number) / 3.0)
					)
				else:
					this_scene_progress = 1.0
			scene_progress += this_scene_progress

		var current_progress: int = int(scene_progress / float(scenes_loaded_count) * 80.0) + 20
		loading_screen.set_progress(current_progress)

		if current_progress == 100:
			hide_loading_screen()
