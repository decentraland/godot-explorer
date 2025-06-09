extends DclGlobal

signal on_menu_open
signal on_menu_close
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

# Increase this value for new terms and conditions
const TERMS_AND_CONDITIONS_VERSION: int = 1

## Global classes (singleton pattern)
var raycast_debugger: RaycastDebugger

var scene_fetcher: SceneFetcher
var skybox_time: SkyboxTime = null

var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader

var music_player: MusicPlayer

var standalone = false
var dcl_android_plugin
var webkit_android_plugin
var webkit_ios_plugin

var network_inspector_window: Window = null


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
	if args.has("--test-runner"):
		print("Running godot-tests...")
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	if Engine.has_singleton("DclAndroidPlugin"):
		dcl_android_plugin = Engine.get_singleton("DclAndroidPlugin")

	if Engine.has_singleton("webview-godot-android"):
		webkit_android_plugin = Engine.get_singleton("webview-godot-android")

	if Engine.has_singleton("WebKit"):
		webkit_ios_plugin = Engine.get_singleton("WebKit")

	self.metrics = Metrics.create_metrics(
		self.config.analytics_user_id, DclConfig.generate_uuid_v4()
	)
	self.metrics.set_name("metrics")

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.dcl_tokio_rpc = DclTokioRpc.new()
	self.dcl_tokio_rpc.set_name("dcl_tokio_rpc")

	self.player_identity = PlayerIdentity.new()
	self.player_identity.set_name("player_identity")

	self.testing_tools = TestingTools.new()
	self.testing_tools.set_name("testing_tool")

	self.content_provider = ContentProvider.new()
	self.content_provider.set_name("content_provider")

	self.scene_fetcher = SceneFetcher.new()
	self.scene_fetcher.set_name("scene_fetcher")

	self.skybox_time = SkyboxTime.new()
	self.skybox_time.set_name("skybox_time")

	self.portable_experience_controller = PortableExperienceController.new()
	self.portable_experience_controller.set_name("portable_experience_controller")

	self.avatars = AvatarScene.new()
	self.avatars.set_name("avatar_scene")

	get_tree().root.add_child.call_deferred(self.music_player)
	get_tree().root.add_child.call_deferred(self.scene_fetcher)
	get_tree().root.add_child.call_deferred(self.skybox_time)
	get_tree().root.add_child.call_deferred(self.content_provider)
	get_tree().root.add_child.call_deferred(self.scene_runner)
	get_tree().root.add_child.call_deferred(self.realm)
	get_tree().root.add_child.call_deferred(self.dcl_tokio_rpc)
	get_tree().root.add_child.call_deferred(self.player_identity)
	get_tree().root.add_child.call_deferred(self.comms)
	get_tree().root.add_child.call_deferred(self.avatars)
	get_tree().root.add_child.call_deferred(self.portable_experience_controller)
	get_tree().root.add_child.call_deferred(self.testing_tools)
	get_tree().root.add_child.call_deferred(self.metrics)
	get_tree().root.add_child.call_deferred(self.network_inspector)

	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)

	if args.has("--raycast-debugger"):
		set_raycast_debugger_enable(true)

	if args.has("--network-debugger"):
		self.network_inspector.set_is_active(true)
		open_network_inspector_ui()
	else:
		self.network_inspector.set_is_active(false)

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


func open_webview_url(url):
	if webkit_ios_plugin != null:
		webkit_ios_plugin.open_webview_url(url)
	elif webkit_android_plugin != null:
		webkit_android_plugin.openCustomTabUrl(url)  # FOR SOCIAL
	else:
		OS.shell_open(url)


func open_url(url: String, use_webkit: bool = false):
	if use_webkit:
		if webkit_ios_plugin != null:
			webkit_ios_plugin.open_auth_url(url)
		elif webkit_android_plugin != null:
			if player_identity.target_config_id == "androidSocial":
				webkit_android_plugin.openCustomTabUrl(url)  # FOR SOCIAL
			else:
				webkit_android_plugin.openWebView(url, "")  # FOR WALLET CONNECT
		else:
			#printerr("No webkit plugin found")
			OS.shell_open(url)
	else:
		if Global.dcl_android_plugin != null:
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


func open_network_inspector_ui():
	if is_instance_valid(network_inspector_window):
		network_inspector_window.show()
		return

	get_viewport().set_embedding_subwindows(false)
	network_inspector_window = Window.new()
	network_inspector_window.size = Vector2i(1280, 720)
	network_inspector_window.initial_position = (
		Window.WINDOW_INITIAL_POSITION_CENTER_MAIN_WINDOW_SCREEN
	)

	const NETWORK_INSPECTOR_UI = preload(
		"res://src/ui/components/debug_panel/network_inspector/network_inspector_ui.tscn"
	)
	network_inspector_window.add_child(NETWORK_INSPECTOR_UI.instantiate())

	add_child(network_inspector_window)
	network_inspector_window.show()


func async_load_threaded(resource_path: String, promise: Promise) -> void:
	prints("loading async", resource_path)

	var main_tree = get_tree()
	var err = ResourceLoader.load_threaded_request(resource_path)
	if err != OK:
		promise.reject("async_load_threaded error load_threaded_request not ok")
		return

	var status = ResourceLoader.load_threaded_get_status(resource_path)
	while status == 1:
		await main_tree.process_frame
		status = ResourceLoader.load_threaded_get_status(resource_path)

	var resource = ResourceLoader.load_threaded_get(resource_path)
	if resource == null:
		promise.reject("async_load_threaded error load_threaded_request result null")
		return

	promise.resolve_with_data(resource)


func set_orientation_landscape():
	DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR_LANDSCAPE)


func is_orientation_portrait():
	var window_size: Vector2i = DisplayServer.window_get_size()
	return window_size.x < window_size.y


func set_orientation_portrait():
	DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR_PORTRAIT)


func set_orientation_sensor():
	DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR)


func teleport_to(parcel_position: Vector2i, new_realm: String):
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		explorer.teleport_to(parcel_position, new_realm)
		explorer.hide_menu()
	else:
		Global.get_config().last_realm_joined = new_realm
		Global.get_config().last_parcel_position = parcel_position
		Global.get_config().add_place_to_last_places(parcel_position, new_realm)
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")


func http_method_to_string(method: int) -> String:
	match method:
		HTTPClient.METHOD_GET:
			return "GET"
		HTTPClient.METHOD_POST:
			return "POST"
		HTTPClient.METHOD_PUT:
			return "PUT"
		HTTPClient.METHOD_DELETE:
			return "DELETE"
		HTTPClient.METHOD_HEAD:
			return "HEAD"
		HTTPClient.METHOD_OPTIONS:
			return "OPTIONS"
		HTTPClient.METHOD_PATCH:
			return "PATCH"
		HTTPClient.METHOD_CONNECT:
			return "CONNECT"
		HTTPClient.METHOD_TRACE:
			return "TRACE"
		_:
			return "GET"  # Default fallback


func async_signed_fetch(url:String, method:int, _body:String=""):
	var headers_promise = Global.player_identity.async_get_identity_headers(url, _body, http_method_to_string(method))
	var headers_result = await PromiseUtils.async_awaiter(headers_promise)
	
	if headers_result is PromiseError:
		printerr("Error getting identity headers: ", headers_result.get_error())
		return
		
	var headers: Dictionary = headers_result
	
	if not _body.is_empty():
		headers["Content-Type"] = "application/json"
	
	var response_promise: Promise = Global.http_requester.request_json(url, method, _body, headers)
	var response_result = await PromiseUtils.async_awaiter(response_promise)
	
	if response_result is PromiseError:
		printerr("Error making HTTP request: ", response_result.get_error())
		return
	
	var json: Dictionary = response_result.get_string_response_as_json()
	return json
