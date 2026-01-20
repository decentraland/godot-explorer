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
signal open_navbar_silently
signal close_menu
signal close_navbar
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
const FORCE_TEST_REALM = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"
const FORCE_TEST_LOCATION = Vector2i(54, -55)
#const FORCE_TEST_ARG = "[[52,-56]]"
# const FORCE_TEST_REALM = "http://localhost:8000"

# Increase this value for new terms and conditions
const TERMS_AND_CONDITIONS_VERSION: int = 1

# Increase this value when local assets cache format changes (invalidates cache)
const LOCAL_ASSETS_CACHE_VERSION: int = 3

## Global classes (singleton pattern)

var raycast_debugger: RaycastDebugger

var scene_fetcher: SceneFetcher
var skybox_time: SkyboxTime = null

var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader

var snapshot: Snapshot

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
var session_id: String

var _is_portrait: bool = true

# Cached reference to SafeAreaPresets (loaded dynamically to avoid export issues)
var _safe_area_presets: GDScript = null


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


func is_emulating_safe_area() -> bool:
	return cli.emulate_ios or cli.emulate_android


func _get_safe_area_presets() -> GDScript:
	if _safe_area_presets == null:
		_safe_area_presets = load("res://assets/no-export/safe_area_presets.gd")
	return _safe_area_presets


func get_safe_area() -> Rect2i:
	if cli.emulate_ios:
		var presets := _get_safe_area_presets()
		return presets.get_ios_safe_area(is_orientation_portrait(), get_window().size)
	if cli.emulate_android:
		var presets := _get_safe_area_presets()
		return presets.get_android_safe_area(is_orientation_portrait(), get_window().size)
	return DisplayServer.get_display_safe_area()


func _instantiate_phone_frame_overlay() -> void:
	var overlay_scene = load("res://assets/no-export/phone_frame_overlay.tscn")
	if overlay_scene:
		var overlay = overlay_scene.instantiate()
		add_child(overlay)


## Vibrate handheld device
func send_haptic_feedback() -> void:
	if is_mobile():
		Input.vibrate_handheld(20)


# gdlint: ignore=async-function-name
func _ready():
	# Use CLI singleton for command-line args
	if cli.force_mobile:
		_set_is_mobile(true)

	# Handle safe area emulation (enables mobile mode and resizes window)
	if cli.emulate_ios:
		_set_is_mobile(true)
		var presets := _get_safe_area_presets()
		var target_size: Vector2i = presets.get_ios_window_size(is_orientation_portrait())
		get_window().size = target_size
		get_window().move_to_center()
		_instantiate_phone_frame_overlay()
	elif cli.emulate_android:
		_set_is_mobile(true)
		var presets := _get_safe_area_presets()
		var target_size: Vector2i = presets.get_android_window_size(is_orientation_portrait())
		get_window().size = target_size
		get_window().move_to_center()
		_instantiate_phone_frame_overlay()

	# Handle fake deep link from CLI (for testing mobile deep links on desktop)
	if not cli.fake_deeplink.is_empty():
		deep_link_url = cli.fake_deeplink
		deep_link_obj = DclParseDeepLink.parse_decentraland_link(cli.fake_deeplink)
		print(
			"[DEEPLINK] Parsed fake deep_link_obj: location=",
			deep_link_obj.location,
			" realm=",
			deep_link_obj.realm,
			" preview=",
			deep_link_obj.preview
		)

	# Connect to iOS deeplink signal
	if DclIosPlugin.is_available():
		var dcl_ios_singleton = Engine.get_singleton("DclGodotiOS")
		if dcl_ios_singleton:
			dcl_ios_singleton.deeplink_received.connect(_on_deeplink_received)

	# Setup
	nft_frame_loader = NftFrameStyleLoader.new()
	nft_fetcher = OpenSeaFetcher.new()
	music_player = MusicPlayer.new()
	snapshot = Snapshot.new()
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

	# Initialize environment from deep link or default to "org"
	var env = deep_link_obj.dclenv if not deep_link_obj.dclenv.is_empty() else "org"
	DclGlobal.set_dcl_environment(env)
	if env != "org":
		print("[GLOBAL] Environment set to: ", env)

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

	# Clear cache if needed (startup flag or version changed) - await completion
	await _async_clear_cache_if_needed()

	# #[itest] only needs a godot context, not the all explorer one
	if cli.test_runner:
		print("Running godot-tests...")
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	session_id = DclConfig.generate_uuid_v4()
	# Initialize metrics with proper user_id and session_id
	self.metrics = Metrics.create_metrics(self.config.analytics_user_id, session_id)
	self.metrics.set_debug_level(0)  # 0 off - 1 on
	self.metrics.set_name("metrics")

	var sentry_user = SentryUser.new()
	sentry_user.id = self.config.analytics_user_id
	SentrySDK.set_tag("dcl_session_id", session_id)

	# Emit test messages to verify Sentry integration (all builds except production)
	# Note: Rust messages must come BEFORE GDScript ones because push_error() captures an event
	# and we want Rust breadcrumbs to be included in that event
	if not DclGlobal.is_production():
		DclGlobal.emit_sentry_rust_test_messages()
		_emit_sentry_godot_test_messages()

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

	# Add stress test controller if stress testing is enabled
	if cli.stress_test:
		print("✓ StressTest initialized for scene loading/unloading stress test")
		var stress_test_controller = load("res://src/tools/stress_test_controller.gd").new()
		stress_test_controller.set_name("StressTestController")
		get_tree().root.add_child.call_deferred(stress_test_controller)

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


## Async helper to clear cache and wait for completion before anything loads.
func _async_clear_cache_if_needed() -> void:
	var should_clear_startup = cli.clear_cache_startup
	var version_changed = config.local_assets_cache_version != Global.LOCAL_ASSETS_CACHE_VERSION

	if should_clear_startup or version_changed:
		if should_clear_startup:
			prints("Clear cache startup!")
		if version_changed:
			prints("Local assets cache version changed, clearing cache!")

		var clear_promise = Global.content_provider.clear_cache_folder()
		await PromiseUtils.async_awaiter(clear_promise)
		prints("Cache cleared successfully!")

		if version_changed:
			config.local_assets_cache_version = Global.LOCAL_ASSETS_CACHE_VERSION
			config.save_to_settings_file()


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
	elif DclAndroidPlugin.is_available():
		DclAndroidPlugin.open_custom_tab_url(url)
	else:
		OS.shell_open(url)


func open_url(url: String, use_webkit: bool = false):
	if use_webkit and not Global.is_xr():
		if DclIosPlugin.is_available():
			DclIosPlugin.open_auth_url(url)
		elif DclAndroidPlugin.is_available():
			if player_identity.target_config_id == "androidSocial":
				DclAndroidPlugin.open_custom_tab_url(url)  # FOR SOCIAL
			else:
				DclAndroidPlugin.open_webview(url, "")  # FOR WALLET CONNECT
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
	elif cli.emulate_ios:
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_ios_window_size(false)
		get_window().move_to_center()
	elif cli.emulate_android:
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_android_window_size(false)
		get_window().move_to_center()
	else:
		get_window().size = Vector2i(1280, 720)
		get_window().move_to_center()
	_is_portrait = false


func is_orientation_portrait() -> bool:
	return _is_portrait


func set_orientation_portrait():
	if Global.is_mobile() and !Global.is_virtual_mobile():
		DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR_PORTRAIT)
	elif cli.emulate_ios:
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_ios_window_size(true)
		get_window().move_to_center()
	elif cli.emulate_android:
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_android_window_size(true)
		get_window().move_to_center()
	else:
		get_window().size = Vector2i(720, 1280)
		get_window().move_to_center()
	_is_portrait = true


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


# Save profile (ADR-290: snapshots are no longer uploaded)
func async_save_profile_metadata(profile: DclUserProfile):
	await ProfileService.async_deploy_profile(profile)


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


func _emit_sentry_godot_test_messages() -> void:
	print("[Sentry Test] GDScript: print() - breadcrumb")
	print_rich("[Sentry Test] GDScript: print_rich() - breadcrumb")
	push_warning("[Sentry Test] GDScript: push_warning() - breadcrumb")
	push_error("[Sentry Test] GDScript: push_error() - event")
	# Also test SentrySDK.capture_message directly
	SentrySDK.capture_message(
		"[Sentry Test] GDScript: capture_message INFO - breadcrumb", SentrySDK.LEVEL_INFO
	)
	SentrySDK.capture_message(
		"[Sentry Test] GDScript: capture_message WARNING - breadcrumb", SentrySDK.LEVEL_WARNING
	)
	SentrySDK.capture_message(
		"[Sentry Test] GDScript: capture_message ERROR - event", SentrySDK.LEVEL_ERROR
	)


func check_deep_link_teleport_to():
	if Global.is_mobile():
		var new_deep_link_url: String = ""
		if DclAndroidPlugin.is_available():
			var args = DclAndroidPlugin.get_deeplink_args()
			new_deep_link_url = args.get("data", "")
		elif DclIosPlugin.is_available():
			var args = DclIosPlugin.get_deeplink_args()
			new_deep_link_url = args.get("data", "")

		if not new_deep_link_url.is_empty():
			deep_link_url = new_deep_link_url
			deep_link_obj = DclParseDeepLink.parse_decentraland_link(deep_link_url)

		if Global.deep_link_obj.is_location_defined():
			# Use preview URL as realm if specified, otherwise use realm, otherwise main
			var realm = Global.deep_link_obj.preview
			if realm.is_empty():
				realm = Global.deep_link_obj.realm
			if realm.is_empty():
				realm = DclUrls.main_realm()

			Global.teleport_to(Global.deep_link_obj.location, realm)
		elif not Global.deep_link_obj.preview.is_empty():
			# Preview without location - just set realm, don't teleport
			Global.teleport_to(Vector2i.ZERO, Global.deep_link_obj.preview)
		elif not Global.deep_link_obj.realm.is_empty():
			Global.teleport_to(Vector2i.ZERO, Global.deep_link_obj.realm)


func _on_deeplink_received(url: String) -> void:
	if not url.is_empty():
		deep_link_url = url
		deep_link_obj = DclParseDeepLink.parse_decentraland_link(url)

		# Handle signin deep link for mobile auth flow
		if deep_link_obj.is_signin_request():
			_handle_signin_deep_link(deep_link_obj.signin_identity_id)
		else:
			deep_link_received.emit.call_deferred()


func _handle_signin_deep_link(identity_id: String) -> void:
	if Global.player_identity.has_pending_mobile_auth():
		Global.player_identity.complete_mobile_connect_account(identity_id)
	else:
		printerr("[DEEPLINK] Received signin deep link but no pending mobile auth")


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_IN or what == NOTIFICATION_READY:
		if Global.is_mobile():
			if DclAndroidPlugin.is_available():
				deep_link_url = DclAndroidPlugin.get_deeplink_args().get("data", "")
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
