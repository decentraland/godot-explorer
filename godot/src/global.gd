extends DclGlobal

signal config_changed
signal loading_finished

enum CameraMode {
	FIRST_PERSON = 0,
	THIRD_PERSON = 1,
	CINEMATIC = 2,
}

# Only for debugging purpose, Godot editor doesn't include a custom param debugging
const FORCE_TEST = false
const FORCE_TEST_ARG = "[[52,-52],[52,-54],[52,-56],[52,-58],[52,-60],[52,-62],[52,-64],[52,-66],[52,-68],[54,-52],[54,-54],[54,-56],[54,-58],[54,-60]]"
const FORCE_TEST_REALM = "https://decentraland.github.io/scene-explorer-tests/scene-explorer-tests"
#const FORCE_TEST_ARG = "[[52,-56]]"
# const FORCE_TEST_REALM = "http://localhost:8000"

## Global classes (singleton pattern)
var config: ConfigData

var raycast_debugger: RaycastDebugger
var animation_importer: AnimationImporter

var scene_fetcher: SceneFetcher
var http_requester: RustHttpQueueRequester

var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader

var standalone = false
var dcl_android_plugin


func _ready():
	var args := OS.get_cmdline_args()
	# _set_is_mobile(true) # Test

	if args.has("--force-mobile"):
		_set_is_mobile(true)

	# Setup
	http_requester = RustHttpQueueRequester.new()
	animation_importer = AnimationImporter.new()
	nft_frame_loader = NftFrameStyleLoader.new()
	nft_fetcher = OpenSeaFetcher.new()

	if args.size() == 1 and args[0].begins_with("res://"):
		if args[0] != "res://src/main.tscn":
			self.standalone = true

	if FORCE_TEST:
		Global.testing_scene_mode = true

	self.config = ConfigData.new()
	config.load_from_settings_file()

	if args.has("--clear-cache-startup"):
		prints("Clear cache startup!")
		Global.clear_cache()

	# #[itest] only needs a godot context, not the all explorer one
	if args.has("--test"):
		print("Running godot-tests...")
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	if Engine.has_singleton("DclAndroidPlugin"):
		dcl_android_plugin = Engine.get_singleton("DclAndroidPlugin")

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.player_identity = PlayerIdentity.new()
	self.player_identity.set_name("player_identity")

	self.testing_tools = TestingTools.new()
	self.testing_tools.set_name("testing_tool")

	self.content_provider = ContentProvider.new()
	self.content_provider.set_name("content_provider")

	self.scene_fetcher = SceneFetcher.new()
	self.scene_fetcher.set_name("scene_fetcher")

	self.portable_experience_controller = PortableExperienceController.new()
	self.portable_experience_controller.set_name("portable_experience_controller")

	self.avatars = AvatarScene.new()
	self.avatars.set_name("avatar_scene")

	get_tree().root.add_child.call_deferred(self.scene_fetcher)
	get_tree().root.add_child.call_deferred(self.content_provider)
	get_tree().root.add_child.call_deferred(self.scene_runner)
	get_tree().root.add_child.call_deferred(self.realm)
	get_tree().root.add_child.call_deferred(self.player_identity)
	get_tree().root.add_child.call_deferred(self.comms)
	get_tree().root.add_child.call_deferred(self.avatars)
	get_tree().root.add_child.call_deferred(self.portable_experience_controller)
	get_tree().root.add_child.call_deferred(self.testing_tools)

	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)

	if args.has("--raycast-debugger"):
		set_raycast_debugger_enable(true)

	DclMeshRenderer.init_primitive_shapes()


func set_raycast_debugger_enable(enable: bool):
	var current_enabled = is_instance_valid(raycast_debugger)
	if current_enabled == enable:
		return

	if enable:
		raycast_debugger = RaycastDebugger.new()
		add_child(raycast_debugger)
	else:
		remove_child(raycast_debugger)
		raycast_debugger = null


func add_raycast(id: int, time: float, from: Vector3, to: Vector3) -> void:
	if is_instance_valid(raycast_debugger):
		raycast_debugger.add_raycast(id, time, from, to)


func print_node_tree(node: Node, prefix = ""):
	print(prefix + node.name)
	for child in node.get_children():
		if child is Node:
			print_node_tree(child, prefix + node.name + "/")


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


func open_url(url: String):
	if Global.dcl_android_plugin != null:
		Global.dcl_android_plugin.showDecentralandMobileToast()
		Global.dcl_android_plugin.openUrl(url)
	else:
		OS.shell_open(url)


func clear_cache():
	# Clean the content cache folder
	if DirAccess.dir_exists_absolute(Global.config.local_content_dir):
		for file in DirAccess.get_files_at(Global.config.local_content_dir):
			DirAccess.remove_absolute(Global.config.local_content_dir + file)
		DirAccess.remove_absolute(Global.config.local_content_dir)

	if not DirAccess.dir_exists_absolute(Global.config.local_content_dir):
		DirAccess.make_dir_absolute(Global.config.local_content_dir)
