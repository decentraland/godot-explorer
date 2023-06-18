extends Node

func _ready():
	start.call_deferred()
	
func start():
	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")
		
	var scene_runner = SceneManager.new()
	scene_runner.set_name("scene_runner")
	
	var realm = Realm.new()
	realm.set_name("realm")
	
	var content_manager = ContentManager.new()
	content_manager.set_name("content_manager")
	
	get_tree().root.add_child(scene_runner)
	get_tree().root.add_child(realm)
	get_tree().root.add_child(content_manager)
	
	get_tree().change_scene_to_file("res://src/ui/explorer.tscn")

