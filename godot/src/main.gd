extends Node


func _ready():
	start.call_deferred()


func start():
	var args := OS.get_cmdline_args()
#	if some.
	if args.has("--test"):
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	Global.scene_runner = SceneManager.new()
	Global.scene_runner.set_name("scene_runner")

	Global.realm = Realm.new()
	Global.realm.set_name("realm")

	Global.content_manager = ContentManager.new()
	Global.content_manager.set_name("content_manager")

	Global.comms = CommunicationManager.new()
	Global.comms.set_name("comms")

	Global.avatars = AvatarScene.new()
	Global.avatars.set_name("avatars")

	get_tree().root.add_child(Global.scene_runner)
	get_tree().root.add_child(Global.realm)
	get_tree().root.add_child(Global.comms)
	get_tree().root.add_child(Global.content_manager)
	get_tree().root.add_child(Global.avatars)

	self._start.call_deferred()
	
func _start():
	get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
