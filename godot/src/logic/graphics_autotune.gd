class_name GraphicsAutotune
extends RefCounted

## First-launch hardware benchmark and the dynamic-graphics profile/FPS-cap wiring.

var _hardware_benchmark: HardwareBenchmark = null


## Check if first launch benchmark should run (mobile only, first launch or dev builds)
## This is called by the engine host once the engine is up, to avoid blocking the first frames.
## Skipped in bench mode: HardwareBenchmark calls viewport_set_measure_render_time and
## can clobber the gp_benchmark_runner's measurement state mid-bench.
func should_run_first_launch_benchmark() -> bool:
	if Global.cli.bench_mode:
		return false
	return (
		Global.is_mobile()
		and (not Global.get_config().first_launch_completed or DclGlobal.is_dev())
	)


## Run the first launch benchmark. Called by the engine host once the engine is up.
func run_first_launch_benchmark() -> void:
	if _hardware_benchmark != null:
		return  # Already running
	_run_first_launch_benchmark()


func init_dynamic_graphics() -> void:
	# In bench mode, skip DG init entirely: the bench runner pins force-graphic-profile,
	# disables thermal cap, and disconnects DG's signal handlers anyway — initializing
	# DG just adds startup CPU noise and queues thermal_fps_cap signals that race the
	# bench setup.
	if Global.cli.bench_mode:
		print("[DynamicGraphics] skipped (bench_mode=true)")
		return
	# Initialize with config values and connect signals
	Global.dynamic_graphics_manager.initialize(
		Global.get_config().dynamic_graphics_enabled,
		Global.get_config().graphic_profile,
		Global.get_config().limit_fps
	)
	Global.loading_started.connect(Global.dynamic_graphics_manager.on_loading_started)
	Global.loading_finished.connect(Global.dynamic_graphics_manager.on_loading_finished)
	Global.dynamic_graphics_manager.profile_change_requested.connect(_on_dynamic_profile_change)
	Global.dynamic_graphics_manager.thermal_fps_cap_changed.connect(_on_thermal_fps_cap_changed)
	# Listen for FPS limit changes
	Global.get_config().param_changed.connect(_on_config_param_changed)


func _run_first_launch_benchmark() -> void:
	print("[HardwareBenchmark] Running first launch benchmark...")
	_hardware_benchmark = HardwareBenchmark.new()
	_hardware_benchmark.benchmark_completed.connect(_on_first_launch_benchmark_completed)
	Global.get_tree().root.add_child(_hardware_benchmark)
	_hardware_benchmark.run_benchmark()


func _on_first_launch_benchmark_completed(profile: int, gpu_score: float, ram_gb: float) -> void:
	print(
		(
			"[HardwareBenchmark] First launch complete: profile=%d, gpu=%.1fms, ram=%.1fGB"
			% [profile, gpu_score, ram_gb]
		)
	)

	# Store benchmark results for debugging/analytics
	Global.get_config().benchmark_gpu_score = gpu_score
	Global.get_config().benchmark_ram_gb = ram_gb
	Global.get_config().first_launch_completed = true

	# Apply the detected profile
	GraphicSettings.apply_graphic_profile(profile)

	# Sync the dynamic graphics manager with the new profile
	Global.dynamic_graphics_manager.on_manual_profile_change(profile)

	# Save configuration
	Global.get_config().save_to_settings_file()

	# Cleanup benchmark
	if is_instance_valid(_hardware_benchmark):
		_hardware_benchmark.queue_free()
		_hardware_benchmark = null


func _on_config_param_changed(param: int) -> void:
	if param == ConfigData.ConfigParams.LIMIT_FPS:
		Global.dynamic_graphics_manager.on_fps_limit_changed(Global.get_config().limit_fps)
		# Re-apply FPS limit considering thermal cap
		GraphicSettings.apply_fps_limit_with_thermal_cap(
			Global.get_config().limit_fps, Global.dynamic_graphics_manager.get_thermal_fps_cap()
		)


func _on_thermal_fps_cap_changed(fps_cap: int) -> void:
	# Apply the effective FPS limit (considering both user setting and thermal cap)
	GraphicSettings.apply_fps_limit_with_thermal_cap(Global.get_config().limit_fps, fps_cap)
	if fps_cap == 0:
		print("[ThermalFPS] Cap removed, using user setting")
	else:
		print("[ThermalFPS] Cap applied: %d FPS" % fps_cap)


func _on_dynamic_profile_change(new_profile: int) -> void:
	var old_profile: int = Global.get_config().graphic_profile
	if old_profile == new_profile:
		# No actual change, skip
		return

	var old_name: String = GraphicSettings.PROFILE_NAMES[old_profile]
	var new_name: String = GraphicSettings.PROFILE_NAMES[new_profile]

	print("[DynamicGraphics] Profile changed: %s -> %s" % [old_name, new_name])

	# Apply the profile change requested by the dynamic graphics manager
	GraphicSettings.apply_graphic_profile(new_profile)
	Global.get_config().save_to_settings_file()
