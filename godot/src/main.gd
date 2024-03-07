extends Node


func _ready():
	start.call_deferred()


func start():
	var args = OS.get_cmdline_args()
	if args.has("--test"):
		return

	if not OS.has_feature("Server"):
		print("Running from platform")

		# Apply basic config
		var main_window: Window = get_node("/root")
		GraphicSettings.apply_window_config()
		main_window.move_to_center()
		GraphicSettings.connect_global_signal(main_window)
		GraphicSettings.apply_ui_zoom(main_window)
		main_window.get_viewport().scaling_3d_scale = Global.config.resolution_3d_scale

		AudioSettings.apply_volume_settings()
	else:
		print("Running from Server")

	if Global.is_mobile():
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

	self._start.call_deferred()


func _start():
	var args = OS.get_cmdline_args()

	if args.has("--avatar-renderer"):
		get_tree().change_scene_to_file(
			"res://src/tool/avatar_renderer/avatar_renderer_standalone.tscn"
		)
	elif args.has("--scene-renderer"):
		get_tree().change_scene_to_file("res://src/tool/scene_renderer/scene.tscn")
	elif args.has("--scene-test"):
		Global.config.guest_profile = {}
		Global.config.save_to_settings_file()
		Global.player_identity.set_default_profile()
		Global.player_identity.create_guest_account()

		var new_stored_account: Dictionary = {}
		if Global.player_identity.get_recover_account_to(new_stored_account):
			Global.config.session_account = new_stored_account
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
	else:
		get_tree().change_scene_to_file("res://src/ui/components/auth/lobby.tscn")
