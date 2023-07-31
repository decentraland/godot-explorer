extends Node

signal config_changed

@onready var is_mobile = OS.get_name() == "Android"
#@onready var is_mobile = true

## Global classes (singleton pattern)

var scene_runner: SceneManager
var realm: Realm
var content_manager: ContentManager
var comms: CommunicationManager
var avatars: AvatarScene
var config: Config
var config_dictionary: Dictionary = {
	"gravity": 55.0,
	"jump_velocity": 12.0,
	"walk_velocity": 12.0,
	"run_velocity": 20.0,
	"process_tick_quota": 1,
	"scene_radius": 1,
}

var raycast_debugger = load("res://src/tool/raycast_debugger/raycast_debugger.gd").new()


func _ready():
	var args := OS.get_cmdline_args()
#	if some.
	if args.has("--test"):
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	self.scene_runner = SceneManager.new()
	self.scene_runner.set_name("scene_runner")
	self.scene_runner.process_mode = Node.PROCESS_MODE_DISABLED

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.content_manager = ContentManager.new()
	self.content_manager.set_name("content_manager")

	self.comms = CommunicationManager.new()
	self.comms.set_name("comms")

	self.avatars = AvatarScene.new()
	self.avatars.set_name("avatars")
	
	self.config = Config.new()
	self.config.set_name("config")
	config.init()

	get_tree().root.add_child.call_deferred(self.scene_runner)
	get_tree().root.add_child.call_deferred(self.realm)
	get_tree().root.add_child.call_deferred(self.comms)
	get_tree().root.add_child.call_deferred(self.content_manager)
	get_tree().root.add_child.call_deferred(self.avatars)

	# TODO: enable raycast debugger
	add_child(raycast_debugger)


func add_raycast(_id: int, _time: float, _from: Vector3, _to: Vector3) -> void:
	pass
	#raycast_debugger.add_raycast(id, time, from, to)


# TODO: move this to another class?
# Configuration section


func _load():
	pass


func _save():
	emit_signal("config_changed")
	pass


func _default():
	pass


func get_resolution():
	return (
		config_dictionary
		. get(
			"resolution",
		)
	)


func get_gravity():
	return config_dictionary.get("gravity", 55.0)


func get_jump_velocity():
	return config_dictionary.get("jump_velocity", 12.0)


func get_walk_velocity():
	return config_dictionary.get("walk_velocity", 12.0)


func get_run_velocity():
	return config_dictionary.get("run_velocity", 20.0)


func get_process_tick_quota():
	return config_dictionary.get("process_tick_quota", 1)


func get_scene_radius():
	return config_dictionary.get("scene_radius", 1)


func get_tls_client():
	return TLSOptions.client_unsafe()


func print_node_tree(node: Node, prefix = ""):
	print(prefix + node.name)
	for child in node.get_children():
		if child is Node:
			print_node_tree(child, prefix + node.name + "/")
