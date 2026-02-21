extends Node

# Startup instrumentation - capture engine boot time
var _startup_time: int = Time.get_ticks_msec()


func _ready():
	print("[Startup] main._ready: %dms" % (Time.get_ticks_msec() - _startup_time))
	Global.set_orientation_portrait()
	start.call_deferred()


func start():
	print("[Startup] main.start: %dms" % (Time.get_ticks_msec() - _startup_time))
	get_tree().quit_on_go_back = false

	# Check if help was requested
	if Global.cli.is_help_requested():
		# Help text is already printed by the Rust side
		get_tree().quit(0)
		return

	if Global.cli.test_runner:
		return

	if not OS.has_feature("Server"):
		print("Running from platform - version ", Global.renderer_version)

		# Apply basic config
		var main_window: Window = get_node("/root")
		GraphicSettings.apply_window_config()
		GraphicSettings.apply_low_processor_mode()
		main_window.move_to_center()
		GraphicSettings.connect_global_signal(main_window)
		GraphicSettings.apply_ui_zoom(main_window)
		main_window.get_viewport().scaling_3d_scale = Global.get_config().resolution_3d_scale

		AudioSettings.apply_volume_settings()

		GeneralSettings.apply_max_cache_size()
	else:
		print("Running from Server - version ", Global.renderer_version)

	if Global.is_mobile():
		InputMap.action_erase_events("ia_pointer")

	self._start.call_deferred()


func _start():
	print("[Startup] main._start: %dms" % (Time.get_ticks_msec() - _startup_time))
	if Global.cli.asset_server:
		print("Running in Asset Server mode")
		_start_asset_server()
		return

	# Floating Islands Benchmark mode - don't load any UI scene
	if Global.cli.fi_benchmark_size >= 0:
		return

	if Global.is_xr():
		print("Running in XR mode")
		Global.set_orientation_landscape()
		get_tree().change_scene_to_file("res://src/vr/vr_lobby.tscn")
	elif Global.cli.emote_test_mode:
		print("Running in Emote Test mode")
		get_tree().change_scene_to_file("res://src/test/emote/emote_tester_standalone.tscn")
	elif Global.cli.avatar_renderer_mode:
		print("Running in Avatar-Renderer mode")
		get_tree().change_scene_to_file(
			"res://src/tool/avatar_renderer/avatar_renderer_standalone.tscn"
		)
	elif Global.cli.client_test_mode:
		print("Running in Client Test mode")
		get_tree().change_scene_to_file("res://src/client_tests/client_test_scene.tscn")
	elif Global.cli.scene_test_mode or Global.cli.scene_renderer_mode:
		print("Running in Scene Test mode")
		Global.get_config().guest_profile = {}
		Global.get_config().save_to_settings_file()
		Global.player_identity.set_default_profile()
		Global.player_identity.create_guest_account()

		var new_stored_account: Dictionary = {}
		if Global.player_identity.get_recover_account_to(new_stored_account):
			Global.get_config().session_account = new_stored_account
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
	else:
		print("Running in regular mode")
		# EULA check is handled inside lobby.gd — always go to lobby
		get_tree().change_scene_to_file("res://src/ui/components/auth/lobby.tscn")


func _start_asset_server():
	# Check if asset_server feature was compiled
	if not ClassDB.class_exists(&"DclAssetServer"):
		push_error("Asset server requires the 'asset_server' feature to be enabled during build.")
		push_error("Build with: cargo run -- build --features asset_server")
		get_tree().quit(1)
		return

	# Create and start the asset server
	var asset_server = ClassDB.instantiate(&"DclAssetServer")
	asset_server.set_port(Global.cli.asset_server_port)
	asset_server.set_name("AssetServer")
	get_tree().root.add_child(asset_server)
	asset_server.start()

	# Keep the process running in headless mode
	print("Asset server is running. Press Ctrl+C to stop.")
