extends Node

## main.tscn is the boot splash visually (purple background + logo, configured
## in the scene file). Its script kicks off Services.bootstrap under that
## splash so iOS sees a responsive main loop while the heavy startup work
## runs, then hands off to the target scene chosen by
## Global.async_route_to_target_scene.


func _ready() -> void:
	BootInstrumentation.mark("main._ready")
	Global.set_orientation_portrait()
	_async_boot_and_route.call_deferred()


# gdlint:ignore = async-function-name
func _async_boot_and_route() -> void:
	BootInstrumentation.mark("main._async_boot_and_route_start")
	get_tree().quit_on_go_back = false

	# Help requested via CLI: usage is already printed by the Rust side.
	if Services.cli.is_help_requested():
		get_tree().quit(0)
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
		main_window.get_viewport().scaling_3d_scale = Services.config.resolution_3d_scale

		AudioSettings.apply_volume_settings()

		GeneralSettings.apply_max_cache_size()
	else:
		print("Running from Server - version ", Global.renderer_version)

	if Global.is_mobile():
		InputMap.action_erase_events("ia_pointer")

	BootInstrumentation.mark("main.config_applied")
	await Services.bootstrap()
	BootInstrumentation.mark("main.bootstrap_returned")
	Global.async_route_to_target_scene()
