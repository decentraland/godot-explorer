extends DclGlobal

signal config_changed
signal loading_started
signal loading_finished
signal change_parcel(new_parcel: Vector2i)

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
var raycast_debugger: RaycastDebugger

var scene_fetcher: SceneFetcher

var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader

var music_player: MusicPlayer

var standalone = false
var dcl_android_plugin
var webkit_android_plugin


func is_xr() -> bool:
	return OS.has_feature("xr") or get_viewport().use_xr


func _ready():
	var args := OS.get_cmdline_args()
	# _set_is_mobile(true) # Test

	if args.has("--force-mobile"):
		_set_is_mobile(true)

	# Setup
	http_requester = RustHttpQueueRequester.new()
	nft_frame_loader = NftFrameStyleLoader.new()
	nft_fetcher = OpenSeaFetcher.new()
	music_player = MusicPlayer.new()

	if args.size() == 1 and args[0].begins_with("res://"):
		if args[0] != "res://src/main.tscn":
			self.standalone = true

	if FORCE_TEST:
		Global.testing_scene_mode = true

	self.config = ConfigData.new()
	config.load_from_settings_file()

	if args.has("--clear-cache-startup"):
		prints("Clear cache startup!")
		Global.content_provider.clear_cache_folder()

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
	
	if Engine.has_singleton("webkit-godot-android"):
		webkit_android_plugin = Engine.get_singleton("webkit-godot-android")

	self.metrics = Metrics.create_metrics(
		self.config.analytics_user_id, DclConfig.generate_uuid_v4()
	)
	self.metrics.set_name("metrics")

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.dcl_tokio_rpc = DclTokioRpc.new()
	self.dcl_tokio_rpc.set_name("dcl_tokio_rpc")

	self.magic_link = MagicLink.new()
	self.magic_link.set_name("magic_link")

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

	get_tree().root.add_child.call_deferred(self.music_player)
	get_tree().root.add_child.call_deferred(self.scene_fetcher)
	get_tree().root.add_child.call_deferred(self.content_provider)
	get_tree().root.add_child.call_deferred(self.scene_runner)
	get_tree().root.add_child.call_deferred(self.realm)
	get_tree().root.add_child.call_deferred(self.dcl_tokio_rpc)
	get_tree().root.add_child.call_deferred(self.magic_link)
	get_tree().root.add_child.call_deferred(self.player_identity)
	get_tree().root.add_child.call_deferred(self.comms)
	get_tree().root.add_child.call_deferred(self.avatars)
	get_tree().root.add_child.call_deferred(self.portable_experience_controller)
	get_tree().root.add_child.call_deferred(self.testing_tools)
	get_tree().root.add_child.call_deferred(self.metrics)

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
		raycast_debugger.queue_free()
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


func explorer_release_focus() -> void:
	var explorer = get_explorer()
	if explorer == null:
		return

	explorer.ui_root.release_focus.call_deferred()


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
	if webkit_android_plugin != null:
		webkit_android_plugin.openCustomTabUrl(url) # FOR SOCIAL
		#webkit_android_plugin.openWebView(url, "") # FOR WALLET CONNECT

	elif Global.dcl_android_plugin != null:
		Global.dcl_android_plugin.showDecentralandMobileToast()
		Global.dcl_android_plugin.openUrl(url)
	else:
		OS.shell_open(url)


func async_create_popup_warning(
	warning_type: PopupWarning.WarningType, title: String, description: String
):
	var explorer = get_explorer()
	if is_instance_valid(explorer):
		await explorer.warning_messages.async_create_popup_warning(warning_type, title, description)


func async_get_texture_size(content_mapping, src, sender) -> void:
	var texture_hash: String = content_mapping.get_hash(src)
	if texture_hash.is_empty():
		texture_hash = src

	var promise = Global.content_provider.fetch_texture_by_hash(texture_hash, content_mapping)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr(src, "couldn't get the size", result.get_error())
		sender.send(Vector2(2048.0, 2048))
		return

	sender.send(Vector2(result.original_size))
