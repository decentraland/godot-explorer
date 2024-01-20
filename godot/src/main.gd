extends Node


func _ready():
	start.call_deferred()


func start():
	var args = OS.get_cmdline_args()
	if args.has("--test"):
		return

	if not OS.has_feature("Server"):
		print("Running from platform")
		var resolution_manager = ResolutionManager.new()
		resolution_manager.refresh_window_options()
		resolution_manager.center_window(get_window())
		resolution_manager.apply_fps_limit()
	else:
		print("Running from Server")

	if Global.is_mobile:
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
	else:
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
