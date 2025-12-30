extends DclGlobal

signal on_menu_open
signal on_menu_close
signal loading_started
signal loading_finished
signal change_parcel(new_parcel: Vector2i)
signal open_profile_by_avatar(avatar: DclAvatar)
signal open_profile_by_address(address: String)
signal on_chat_message(address: String, message: String, timestamp: float)
signal change_virtual_keyboard(height: int)
signal notification_clicked(notification: Dictionary)
signal notification_received(notification: Dictionary)
signal deep_link_received
signal open_chat
signal open_friends_panel
signal open_notifications_panel
signal open_settings
signal open_backpack
signal open_discover
signal open_own_profile
signal close_menu
signal friends_request_size_changed(size: int)
signal close_combo
signal delete_account

enum CameraMode {
	FIRST_PERSON = 0,
	THIRD_PERSON = 1,
	CINEMATIC = 2,
}

enum FriendshipStatus {
	UNKNOWN = -1,
	REQUEST_SENT = 0,
	REQUEST_RECEIVED = 1,
	CANCELED = 2,
	ACCEPTED = 3,
	REJECTED = 4,
	DELETED = 5,
	NONE = 7
}

# Only for debugging purpose, Godot editor doesn't include a custom param debugging
const FORCE_TEST = false
const FORCE_TEST_ARG = "[[52,-52],[52,-54],[52,-56],[52,-58],[52,-60],[52,-62],[52,-64],[52,-66],[52,-68],[54,-52],[54,-54],[54,-56],[54,-58],[54,-60]]"
const FORCE_TEST_REALM = "https://decentraland.github.io/scene-explorer-tests/scene-explorer-tests"
#const FORCE_TEST_ARG = "[[52,-56]]"
# const FORCE_TEST_REALM = "http://localhost:8000"

# Increase this value for new terms and conditions
const TERMS_AND_CONDITIONS_VERSION: int = 1

# Increase this value when optimized assets change (invalidates cache)
const OPTIMIZED_ASSETS_VERSION: int = 2

## Global classes (singleton pattern)
var raycast_debugger: RaycastDebugger

var scene_fetcher: SceneFetcher
var skybox_time: SkyboxTime = null

var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader

var music_player: MusicPlayer

var preload_assets: PreloadAssets

var locations: Node

var standalone = false

var network_inspector_window: Window = null
var selected_avatar: Avatar = null

var url_popup_instance = null
var jump_in_popup_instance = null

var last_emitted_height: int = 0
var current_height: int = -1
var previous_height: int = -1
var previous_height_2: int = -1

var deep_link_obj: DclParseDeepLink = DclParseDeepLink.new()
var deep_link_url: String = ""

var player_camera_node: DclCamera3D


func set_url_popup_instance(popup_instance) -> void:
	url_popup_instance = popup_instance


func show_url_popup(url: String) -> void:
	if url_popup_instance != null:
		url_popup_instance.open(url)


func hide_url_popup() -> void:
	if url_popup_instance != null:
		url_popup_instance.close()


func set_jump_in_popup_instance(popup_instance) -> void:
	jump_in_popup_instance = popup_instance


func show_jump_in_popup(coordinates: Vector2i) -> void:
	if jump_in_popup_instance != null:
		jump_in_popup_instance.open(coordinates)


func hide_jump_in_popup() -> void:
	if jump_in_popup_instance != null:
		jump_in_popup_instance.close()


func is_xr() -> bool:
	return OS.has_feature("xr") or get_viewport().use_xr


## Vibrate handheld device
func send_haptic_feedback() -> void:
	if is_mobile():
		Input.vibrate_handheld(20)


func _ready():
	# Use CLI singleton for command-line args
	if cli.force_mobile:
		_set_is_mobile(true)

	# Handle fake deep link from CLI (for testing mobile deep links on desktop)
	if not cli.fake_deeplink.is_empty():
		deep_link_url = cli.fake_deeplink
		deep_link_obj = DclParseDeepLink.parse_decentraland_link(cli.fake_deeplink)

	# Connect to iOS deeplink signal
	if DclIosPlugin.is_available():
		var dcl_ios_singleton = Engine.get_singleton("DclGodotiOS")
		if dcl_ios_singleton:
			dcl_ios_singleton.deeplink_received.connect(_on_deeplink_received)

	# Setup
	nft_frame_loader = NftFrameStyleLoader.new()
	nft_fetcher = OpenSeaFetcher.new()
	music_player = MusicPlayer.new()
	preload_assets = PreloadAssets.new()

	var args = cli.get_all_args()
	if args.size() == 1 and args[0].begins_with("res://"):
		if args[0] != "res://src/main.tscn":
			self.standalone = true

	if FORCE_TEST:
		Global.testing_scene_mode = true

	# Create GDScript extensions of Rust classes
	self.config = ConfigData.new()
	config.load_from_settings_file()

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.dcl_tokio_rpc = DclTokioRpc.new()
	self.dcl_tokio_rpc.set_name("dcl_tokio_rpc")

	self.player_identity = PlayerIdentity.new()
	self.player_identity.set_name("player_identity")
	self.player_identity.profile_changed.connect(_on_player_profile_changed_sync_events)

	self.testing_tools = TestingTools.new()
	self.testing_tools.set_name("testing_tool")

	self.portable_experience_controller = PortableExperienceController.new()
	self.portable_experience_controller.set_name("portable_experience_controller")

	if cli.clear_cache_startup:
		prints("Clear cache startup!")
		Global.content_provider.clear_cache_folder()

	# Clear cache if optimized assets version changed
	if config.optimized_assets_version != Global.OPTIMIZED_ASSETS_VERSION:
		prints("Optimized assets version changed, clearing cache!")
		Global.content_provider.clear_cache_folder()
		config.optimized_assets_version = Global.OPTIMIZED_ASSETS_VERSION
		config.save_to_settings_file()

	# #[itest] only needs a godot context, not the all explorer one
	if cli.test_runner:
		print("Running godot-tests...")
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	var session_id := DclConfig.generate_uuid_v4()
	# Initialize metrics with proper user_id and session_id
	self.metrics = Metrics.create_metrics(self.config.analytics_user_id, session_id)
	self.metrics.set_name("metrics")

	var sentry_user = SentryUser.new()
	sentry_user.id = self.config.analytics_user_id
	SentrySDK.set_tag("dcl_session_id", session_id)

	# Create the GDScript-only components
	self.scene_fetcher = SceneFetcher.new()
	self.scene_fetcher.set_name("scene_fetcher")

	self.skybox_time = SkyboxTime.new()
	self.skybox_time.set_name("skybox_time")

	self.locations = load("res://src/helpers_components/locations.gd").new()
	self.locations.set_name("locations")

	get_tree().root.add_child.call_deferred(self.cli)
	get_tree().root.add_child.call_deferred(self.music_player)
	get_tree().root.add_child.call_deferred(self.scene_fetcher)
	get_tree().root.add_child.call_deferred(self.skybox_time)
	get_tree().root.add_child.call_deferred(self.locations)
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
	get_tree().root.add_child.call_deferred(self.social_blacklist)
	if "memory_debugger" in self:
		get_tree().root.add_child.call_deferred(self.memory_debugger)

	# Initialize BenchmarkReport singleton if benchmarking is enabled (requires use_memory_debugger feature)
	if cli.benchmark_report and "benchmark_report" in self:
		print("✓ BenchmarkReport initialized for full flow benchmarking")
		get_tree().root.add_child.call_deferred(self.benchmark_report)

		# Add benchmark flow controller to orchestrate the full benchmark flow
		var benchmark_flow_controller = load("res://src/tools/benchmark_flow_controller.gd").new()
		benchmark_flow_controller.set_name("BenchmarkFlowController")
		get_tree().root.add_child.call_deferred(benchmark_flow_controller)
	elif cli.benchmark_report:
		push_error(
			"BenchmarkReport requires --features use_memory_debugger to be enabled during build"
		)

	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)

	if cli.raycast_debugger:
		set_raycast_debugger_enable(true)

	if cli.network_debugger:
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
	if DclIosPlugin.is_available():
		DclIosPlugin.open_webview_url(url)
	elif DclGodotAndroidPlugin.is_available():
		DclGodotAndroidPlugin.open_custom_tab_url(url)
	else:
		OS.shell_open(url)


func open_url(url: String, use_webkit: bool = false):
	if use_webkit and not Global.is_xr():
		if DclIosPlugin.is_available():
			DclIosPlugin.open_auth_url(url)
		elif DclGodotAndroidPlugin.is_available():
			if player_identity.target_config_id == "androidSocial":
				DclGodotAndroidPlugin.open_custom_tab_url(url)  # FOR SOCIAL
			else:
				DclGodotAndroidPlugin.open_webview(url, "")  # FOR WALLET CONNECT
		else:
			OS.shell_open(url)
	else:
		if DclAndroidPlugin.is_available():
			DclAndroidPlugin.show_decentraland_mobile_toast()
			DclAndroidPlugin.open_url(url)
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
	if Global.is_mobile() and !Global.is_virtual_mobile():
		DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR_LANDSCAPE)
	else:
		get_window().size = Vector2i(1280, 720)
		get_window().move_to_center()


func is_orientation_portrait():
	var window_size: Vector2i = DisplayServer.window_get_size()
	return window_size.x < window_size.y


func set_orientation_portrait():
	if Global.is_mobile() and !Global.is_virtual_mobile():
		DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR_PORTRAIT)
	else:
		get_window().size = Vector2i(720, 1280)
		get_window().move_to_center()


func set_orientation_sensor():
	DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR)


func teleport_to(parcel_position: Vector2i, new_realm: String):
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		explorer.teleport_to(parcel_position, new_realm)
		explorer.hide_menu()
		Global.on_chat_message.emit(
			"system",
			"[color=#ccc]🟢 Teleported to " + str(parcel_position) + "[/color]",
			Time.get_unix_time_from_system()
		)
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


func async_signed_fetch(url: String, method: int, _body: String = ""):
	var headers_promise = Global.player_identity.async_get_identity_headers(
		url, _body, http_method_to_string(method)
	)
	var headers_result = await PromiseUtils.async_awaiter(headers_promise)

	if headers_result is PromiseError:
		return headers_result

	var headers: Dictionary = headers_result
	if not _body.is_empty():
		headers["Content-Type"] = "application/json"

	var response_promise: Promise = Global.http_requester.request_json(url, method, _body, headers)

	return await PromiseUtils.async_awaiter(response_promise)


# Save profile without generating new snapshots (for non-visual changes)
func async_save_profile_metadata(profile: DclUserProfile):
	await ProfileService.async_deploy_profile(profile, false)


func shorten_address(address: String) -> String:
	if address.length() <= 8:
		return address
	var first_part := address.substr(0, 5)
	var last_part := address.substr(address.length() - 5, 5)
	return first_part + "..." + last_part


func get_backpack() -> Backpack:
	var explorer = Global.get_explorer()
	if explorer != null and is_instance_valid(explorer.control_menu):
		return explorer.control_menu.control_backpack
	var control_menu = get_node_or_null("/root/Menu")
	return control_menu.control_backpack


func _process(_delta: float) -> void:
	if Global.is_mobile() and !Global.is_virtual_mobile():
		var virtual_keyboard_height: int = DisplayServer.virtual_keyboard_get_height()

		# Shift the values
		previous_height_2 = previous_height
		previous_height = current_height
		current_height = virtual_keyboard_height

		# Check if stable (same for 3 frames) and different from last emitted
		if (
			current_height == previous_height
			and current_height == previous_height_2
			and current_height != last_emitted_height
		):
			last_emitted_height = current_height
			change_virtual_keyboard.emit(last_emitted_height)


func check_deep_link_teleport_to():
	if Global.is_mobile():
		var new_deep_link_url: String = ""
		if DclGodotAndroidPlugin.is_available():
			var args = DclGodotAndroidPlugin.get_deeplink_args()
			print("[DEEPLINK] Android args: ", args)
			new_deep_link_url = args.get("data", "")
		elif DclIosPlugin.is_available():
			var args = DclIosPlugin.get_deeplink_args()
			print("[DEEPLINK] iOS args: ", args)
			new_deep_link_url = args.get("data", "")

		print("[DEEPLINK] check_deep_link_teleport_to: new_deep_link_url = ", new_deep_link_url)

		if not new_deep_link_url.is_empty():
			deep_link_url = new_deep_link_url
			deep_link_obj = DclParseDeepLink.parse_decentraland_link(deep_link_url)
			print(
				"[DEEPLINK] Parsed deep_link_obj: location=",
				deep_link_obj.location,
				" realm=",
				deep_link_obj.realm,
				" preview=",
				deep_link_obj.preview
			)

		if Global.deep_link_obj.is_location_defined():
			# Use preview URL as realm if specified, otherwise use realm, otherwise main
			var realm = Global.deep_link_obj.preview
			if realm.is_empty():
				realm = Global.deep_link_obj.realm
			if realm.is_empty():
				realm = Realm.MAIN_REALM

			Global.teleport_to(Global.deep_link_obj.location, realm)
		elif not Global.deep_link_obj.preview.is_empty():
			# Preview without location - just set realm, don't teleport
			Global.teleport_to(Vector2i.ZERO, Global.deep_link_obj.preview)
		elif not Global.deep_link_obj.realm.is_empty():
			Global.teleport_to(Vector2i.ZERO, Global.deep_link_obj.realm)
		elif deep_link_url.begins_with("https://decentraland.org/events/event/?id="):
			print("Is event link")


func _on_deeplink_received(url: String) -> void:
	print("[DEEPLINK] Signal received in GDScript: ", url)
	if not url.is_empty():
		deep_link_url = url
		deep_link_obj = DclParseDeepLink.parse_decentraland_link(url)

		# Handle signin deep link for mobile auth flow
		if deep_link_obj.is_signin_request():
			_handle_signin_deep_link(deep_link_obj.signin_identity_id)
		else:
			deep_link_received.emit.call_deferred()


func _handle_signin_deep_link(identity_id: String) -> void:
	print("[DEEPLINK] Handling signin with identity_id: ", identity_id)
	if Global.player_identity.has_pending_mobile_auth():
		Global.player_identity.complete_mobile_connect_account(identity_id)
	else:
		printerr("[DEEPLINK] Received signin deep link but no pending mobile auth")


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_IN or what == NOTIFICATION_READY:
		if Global.is_mobile():
			if DclGodotAndroidPlugin.is_available():
				deep_link_url = DclGodotAndroidPlugin.get_deeplink_args().get("data", "")
			elif DclIosPlugin.is_available():
				deep_link_url = DclIosPlugin.get_deeplink_args().get("data", "")

			if not deep_link_url.is_empty():
				deep_link_obj = DclParseDeepLink.parse_decentraland_link(deep_link_url)
				# Handle signin deep link for mobile auth flow
				if deep_link_obj.is_signin_request():
					_handle_signin_deep_link(deep_link_obj.signin_identity_id)
				else:
					deep_link_received.emit.call_deferred()

			# We do not check at this instance since we'd need to check each singular state (is in lobby? is in navigating? , etc...)


func _on_player_profile_changed_sync_events(_profile: DclUserProfile) -> void:
	# Sync attended events notifications from server after authentication
	NotificationsManager.async_sync_attended_events()
