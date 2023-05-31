extends Node

func _ready():
	start.call_deferred()
	
func start():
	var scene_runner = SceneRunner.new()
	scene_runner.set_name("scene_runner")
	
	var realm = Realm.new()
	realm.set_name("realm")
	
	get_tree().root.add_child(scene_runner)
	get_tree().root.add_child(realm)
	
	get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
