extends Node


func _ready():
	Global.set_orientation_portrait()
	start.call_deferred()


func start():
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
		GraphicSettings.apply_fps_limit()
		main_window.move_to_center()
		GraphicSettings.connect_global_signal(main_window)
		GraphicSettings.apply_ui_zoom(main_window)
		main_window.get_viewport().scaling_3d_scale = Global.get_config().resolution_3d_scale

		AudioSettings.apply_volume_settings()

		GeneralSettings.apply_max_cache_size()
	else:
		print("Running from Server - version ", Global.renderer_version)

	if Global.is_mobile():
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
		InputMap.action_erase_events("ia_pointer")

	self._start.call_deferred()


func _start():
	if Global.is_xr():
		print("Running in XR mode")
		Global.set_orientation_landscape()
		get_tree().change_scene_to_file("res://src/vr/vr_lobby.tscn")
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
		var current_terms_and_conditions_version: int = (
			Global.get_config().terms_and_conditions_version
		)
		if current_terms_and_conditions_version != Global.TERMS_AND_CONDITIONS_VERSION:
			get_tree().change_scene_to_file(
				"res://src/ui/components/terms_and_conditions/terms_and_conditions.tscn"
			)
		else:
			get_tree().change_scene_to_file("res://src/ui/components/auth/lobby.tscn")
