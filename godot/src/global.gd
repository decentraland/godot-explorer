extends DclGlobal

signal config_changed

enum CameraMode {
	FIRST_PERSON = 0,
	THIRD_PERSON = 1,
	CINEMATIC = 2,
}

# Only for debugging purpose, Godot editor doesn't include a custom param debugging
const FORCE_TEST = false
const FORCE_TEST_ARG = '["52,-52"]'
const FORCE_TEST_REALM = "https://sdilauro.github.io/dae-unit-tests/dae-unit-tests"

## Global classes (singleton pattern)
var content_manager: ContentManager
var config: ConfigData

var raycast_debugger = load("res://src/tool/raycast_debugger/raycast_debugger.gd").new()
var animation_importer: AnimationImporter = AnimationImporter.new()

var scene_fetcher: SceneFetcher = null
var http_requester: RustHttpRequesterWrapper = RustHttpRequesterWrapper.new()

var nft_fetcher: OpenSeaFetcher = OpenSeaFetcher.new()
var nft_frame_loader: NftFrameStyleLoader = NftFrameStyleLoader.new()

var standalone = false

@onready var is_mobile = OS.has_feature("mobile")
#@onready var is_mobile = true


func _ready():
	var args := OS.get_cmdline_args()
	if args.size() == 1 and args[0].begins_with("res://"):
		if args[0] != "res://src/main.tscn":
			self.standalone = true

	if FORCE_TEST:
		Global.testing_scene_mode = true

	self.config = ConfigData.new()
	config.load_from_settings_file()

	if args.has("--test"):
		print("Running godot-tests...")
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.testing_tools = TestingTools.new()
	self.testing_tools.set_name("testing_tool")

	self.content_manager = ContentManager.new()
	self.content_manager.set_name("content_manager")

	self.scene_fetcher = SceneFetcher.new()
	self.scene_fetcher.set_name("scene_fetcher")

	self.portable_experience_controller = PortableExperienceController.new()
	self.portable_experience_controller.set_name("portable_experience_controller")

	self.avatars = CustomAvatarScene.new()
	self.avatars.set_name("avatar_scene")

	get_tree().root.add_child.call_deferred(self.scene_fetcher)
	get_tree().root.add_child.call_deferred(self.content_manager)
	get_tree().root.add_child.call_deferred(self.scene_runner)
	get_tree().root.add_child.call_deferred(self.realm)
	get_tree().root.add_child.call_deferred(self.comms)
	get_tree().root.add_child.call_deferred(self.avatars)
	get_tree().root.add_child.call_deferred(self.portable_experience_controller)
	get_tree().root.add_child.call_deferred(self.testing_tools)

	# TODO: enable raycast debugger
	add_child(raycast_debugger)

	DclMeshRenderer.init_primitive_shapes()


func add_raycast(_id: int, _time: float, _from: Vector3, _to: Vector3) -> void:
	# raycast_debugger.add_raycast(id, time, from, to)
	pass


func get_tls_client():
	return TLSOptions.client_unsafe()


func print_node_tree(node: Node, prefix = ""):
	print(prefix + node.name)
	for child in node.get_children():
		if child is Node:
			print_node_tree(child, prefix + node.name + "/")


func _process(_dt: float):
	http_requester.poll()


func get_explorer():
	var explorer = get_node_or_null("/root/explorer")
	if is_instance_valid(explorer):
		return explorer
	return null


func explorer_has_focus() -> bool:
	var explorer = get_explorer()
	if explorer == null:
		return false

	return explorer.ui_root.has_focus()


func explorer_grab_focus() -> void:
	var explorer = get_explorer()
	if explorer == null:
		return

	explorer.ui_root.grab_focus.call_deferred()


func capture_mouse():
	var explorer = get_node_or_null("/root/explorer")
	if is_instance_valid(explorer):
		explorer.capture_mouse()
	else:
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func release_mouse():
	var explorer = get_node_or_null("/root/explorer")
	if is_instance_valid(explorer):
		explorer.release_mouse()
	else:
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
