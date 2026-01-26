## Floating Islands Benchmark Runner
##
## Uses the REAL Global.scene_fetcher to generate floating islands.
## Activated when --fi-benchmark-size is set.

extends Node

var parcel_count: int = 0
var output_path: String = ""
var screenshot_viewport: SubViewport = null


func _ready():
	parcel_count = Global.cli.fi_benchmark_size
	output_path = Global.cli.fi_benchmark_output

	if parcel_count < 0:
		queue_free()
		return

	log_msg("FI Benchmark: Starting with %d scene parcels..." % parcel_count)
	log_msg("FI Benchmark: Output path: %s" % output_path)

	# Wait for Global to fully initialize
	await get_tree().create_timer(1.0).timeout

	run_benchmark()


# gdlint:ignore = async-function-name
func run_benchmark():
	# Create camera to see the parcels
	setup_camera()

	# Measure baseline
	log_msg("FI Benchmark: Collecting baseline metrics...")
	var baseline = collect_metrics()

	# Generate floating islands using the REAL EmptyParcel scene
	log_msg("FI Benchmark: Generating floating islands for %d scene parcels..." % parcel_count)
	var start_time = Time.get_ticks_msec()
	await generate_floating_islands()

	# Wait for all async generation to complete (terrain, cliffs, grass, trees, etc.)
	log_msg("FI Benchmark: Waiting for async generation to complete...")
	await wait_for_generation_complete()

	var generation_time = Time.get_ticks_msec() - start_time
	log_msg("FI Benchmark: Generation completed in %d ms" % generation_time)

	# Set fake player at center (0,0) so grass renders within culling range
	log_msg("FI Benchmark: Setting fake player position at center...")
	Global.scene_fetcher.current_position = Vector2i(0, 0)
	Global.scene_fetcher.player_parcel_changed.emit(Vector2i(0, 0))

	# Wait additional time for memory to stabilize
	log_msg("FI Benchmark: Waiting for memory to stabilize...")
	await get_tree().create_timer(3.0).timeout

	# Measure final metrics
	log_msg("FI Benchmark: Collecting final metrics...")
	var final_metrics = collect_metrics()

	# Count nodes by type
	log_msg("FI Benchmark: Counting nodes by type...")
	var node_breakdown = count_nodes_by_type()

	# Build result
	var result = {
		"parcel_count": parcel_count,
		"generation_time_ms": generation_time,
		"baseline": baseline,
		"final": final_metrics,
		"delta": calculate_delta(baseline, final_metrics),
		"node_breakdown": node_breakdown
	}

	# Write JSON output
	write_results(result)

	# Take screenshot
	log_msg("FI Benchmark: Taking screenshot...")
	await take_screenshot()

	log_msg("FI Benchmark: Complete!")
	get_tree().quit(0)


# gdlint:ignore = async-function-name
func wait_for_generation_complete():
	# Wait until node count stabilizes (no new nodes being added)
	var last_node_count = 0
	var stable_frames = 0
	var required_stable_frames = 30  # ~0.5 seconds at 60fps
	var start_time = Time.get_ticks_msec()

	while stable_frames < required_stable_frames:
		await get_tree().process_frame
		var current_count = count_total_nodes()

		if current_count == last_node_count:
			stable_frames += 1
		else:
			stable_frames = 0
			last_node_count = current_count

		# Safety timeout: max 60 seconds
		if Time.get_ticks_msec() - start_time > 60000:
			log_msg("FI Benchmark: Generation timeout, proceeding anyway...")
			break

	log_msg("FI Benchmark: Node count stabilized at %d nodes" % last_node_count)


func count_total_nodes() -> int:
	var count = 0
	for parcel_key in Global.scene_fetcher.loaded_empty_scenes:
		var parcel = Global.scene_fetcher.loaded_empty_scenes[parcel_key]
		if is_instance_valid(parcel):
			count += count_nodes_in_tree(parcel)
	return count


func count_nodes_in_tree(node: Node) -> int:
	var count = 1
	for child in node.get_children():
		count += count_nodes_in_tree(child)
	return count


# gdlint:ignore = async-function-name
func generate_floating_islands():
	# Use the REAL Global.scene_fetcher to generate floating islands
	var sf = Global.scene_fetcher

	# WORST CASE: Always 4 parcels in a cross pattern, but with increasing separation
	# parcel_count is used as the ARM LENGTH (distance from center to each parcel)
	# This generates more empty parcels as the separation increases
	var arm_length = parcel_count
	var parcels = generate_cross_with_separation(arm_length)

	if arm_length == 0:
		log_msg("FI Benchmark: 1 parcel at origin (base case)")
	else:
		log_msg(
			"FI Benchmark: 4 parcels in CROSS with arm_length=%d (worst case)" % arm_length
		)

	# Create fake scene items to populate loaded_scenes
	var idx = 0
	for parcel_pos in parcels:
		var fake_scene = SceneFetcher.SceneItem.new()
		fake_scene.id = "benchmark_%d" % idx
		var parcel_array: Array[Vector2i] = [parcel_pos]
		fake_scene.parcels = parcel_array
		fake_scene.is_global = false
		fake_scene.scene_number_id = 1000 + idx
		sf.loaded_scenes[fake_scene.id] = fake_scene
		idx += 1

	# Clear hash to force regeneration
	sf.last_scene_group_hash = ""

	log_msg("FI Benchmark: Triggering _regenerate_floating_islands()...")

	# Call the REAL floating island generation
	sf._regenerate_floating_islands()

	# Wait for async generation to complete
	while sf._floating_islands_generating:
		await get_tree().process_frame

	log_msg("FI Benchmark: SceneFetcher created %d empty parcels" % sf.loaded_empty_scenes.size())


## Generate parcels for benchmark:
## - arm_length=0: single parcel at origin (base case)
## - arm_length>0: 4 parcels in cross pattern (worst case)
func generate_cross_with_separation(arm_length: int) -> Array[Vector2i]:
	if arm_length == 0:
		# Single parcel at origin
		return [Vector2i(0, 0)]

	# 4 parcels in cross pattern
	return [
		Vector2i(-arm_length, 0),  # Left
		Vector2i(arm_length, 0),  # Right
		Vector2i(0, -arm_length),  # Up
		Vector2i(0, arm_length),  # Down
	]


func calculate_grid(count: int) -> Dictionary:
	# Calculate optimal grid dimensions
	var width = int(sqrt(count))
	if width < 1:
		width = 1
	var height = int(ceil(float(count) / width))
	return {"width": width, "height": height}


func collect_metrics() -> Dictionary:
	return {
		"memory_static_mb": Performance.get_monitor(Performance.MEMORY_STATIC) / 1048576.0,
		"memory_static_max_mb": Performance.get_monitor(Performance.MEMORY_STATIC_MAX) / 1048576.0,
		"object_count": Performance.get_monitor(Performance.OBJECT_COUNT),
		"object_node_count": Performance.get_monitor(Performance.OBJECT_NODE_COUNT),
		"object_orphan_node_count": Performance.get_monitor(Performance.OBJECT_ORPHAN_NODE_COUNT),
		"render_total_draw_calls":
		Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
		"render_total_primitives":
		Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME),
		"video_mem_used_mb": Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1048576.0,
	}


func calculate_delta(baseline: Dictionary, final: Dictionary) -> Dictionary:
	var delta = {}
	for key in baseline.keys():
		delta[key] = final[key] - baseline[key]
	return delta


func count_nodes_by_type() -> Dictionary:
	var counts = {"terrain": 0, "cliff": 0, "grass": 0, "tree": 0, "rock": 0, "prop": 0, "other": 0}

	for parcel_key in Global.scene_fetcher.loaded_empty_scenes:
		var parcel = Global.scene_fetcher.loaded_empty_scenes[parcel_key]
		if is_instance_valid(parcel):
			count_nodes_recursive(parcel, counts)

	return counts


func count_nodes_recursive(node: Node, counts: Dictionary):
	var name_lower = node.name.to_lower()

	if "terrain" in name_lower:
		counts.terrain += 1
	elif "cliff" in name_lower:
		counts.cliff += 1
	elif "grass" in name_lower:
		counts.grass += 1
	elif "tree" in name_lower:
		counts.tree += 1
	elif "rock" in name_lower:
		counts.rock += 1
	elif "prop" in name_lower:
		counts.prop += 1
	else:
		counts.other += 1

	for child in node.get_children():
		count_nodes_recursive(child, counts)


func write_results(result: Dictionary):
	if output_path.is_empty():
		output_path = OS.get_user_data_dir() + "/output/fi-benchmark.json"

	# Ensure directory exists
	var dir = output_path.get_base_dir()
	DirAccess.make_dir_recursive_absolute(dir)

	var file = FileAccess.open(output_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(result, "\t"))
		file.close()
		log_msg("FI Benchmark: Results written to %s" % output_path)
	else:
		push_error("FI Benchmark: Failed to write results to %s" % output_path)


func log_msg(msg: String):
	print(msg)


# gdlint:ignore = async-function-name
func take_screenshot():
	if screenshot_viewport == null:
		log_msg("FI Benchmark: No viewport for screenshot")
		return

	# Check if we're in headless mode (no rendering)
	if OS.has_feature("Server") or DisplayServer.get_name() == "headless":
		log_msg("FI Benchmark: Skipping screenshot (headless mode)")
		return

	# Wait frames for SubViewport to render
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	# Get screenshot from SubViewport
	var texture = screenshot_viewport.get_texture()
	if texture == null:
		log_msg("FI Benchmark: Skipping screenshot (no texture in headless)")
		return

	var image = texture.get_image()
	if image == null:
		log_msg("FI Benchmark: Skipping screenshot (no image)")
		return

	# Save screenshot next to the JSON output
	var screenshot_path = output_path.replace(".json", ".png")
	var error = image.save_png(screenshot_path)
	if error == OK:
		log_msg("FI Benchmark: Screenshot saved to %s" % screenshot_path)
	else:
		push_error("FI Benchmark: Failed to save screenshot to %s" % screenshot_path)


func setup_camera():
	# For cross pattern: arm_length is parcel_count, extends from -arm to +arm
	# Plus padding of 2 on each side
	var arm_length = parcel_count
	var padding = 2  # SceneFetcher uses padding of 2

	# Calculate bounding box in parcel coordinates
	var min_parcel = -arm_length - padding
	var max_parcel = arm_length + padding

	# Convert to world coordinates (each parcel is 16m, Z is negated in DCL)
	var world_min_x = min_parcel * 16.0
	var world_max_x = (max_parcel + 1) * 16.0
	var world_min_z = -(max_parcel + 1) * 16.0
	var world_max_z = -min_parcel * 16.0

	# Calculate center and size
	var center_x = (world_min_x + world_max_x) / 2.0
	var center_z = (world_min_z + world_max_z) / 2.0
	var island_size = max(world_max_x - world_min_x, world_max_z - world_min_z)

	# Add environment and light to the MAIN world first
	var env = Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.3, 0.4, 0.6)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color.WHITE
	env.ambient_light_energy = 0.8

	var world_env = WorldEnvironment.new()
	world_env.environment = env
	get_tree().root.add_child(world_env)

	var light = DirectionalLight3D.new()
	light.name = "BenchmarkLight"
	light.rotation_degrees = Vector3(-45, 30, 0)
	get_tree().root.add_child(light)

	# Create MAIN camera for the window view (non-headless)
	var main_camera = Camera3D.new()
	main_camera.name = "MainBenchmarkCamera"
	main_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	main_camera.size = island_size * 1.2
	main_camera.far = 1000
	main_camera.near = 0.1
	# Position camera at Y=15 to stay within grass shader fade range (16-24m)
	main_camera.position = Vector3(center_x, 15, center_z)
	main_camera.rotation_degrees = Vector3(-90, 0, 0)
	get_tree().root.add_child(main_camera)
	main_camera.make_current()

	# Make window square to match SubViewport aspect ratio (non-headless only)
	if not OS.has_feature("Server") and DisplayServer.get_name() != "headless":
		get_window().size = Vector2i(1024, 1024)

	# Create SubViewport for screenshots (1024x1024 square)
	screenshot_viewport = SubViewport.new()
	screenshot_viewport.name = "ScreenshotViewport"
	screenshot_viewport.size = Vector2i(1024, 1024)
	screenshot_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	screenshot_viewport.transparent_bg = false
	screenshot_viewport.world_3d = get_viewport().world_3d

	# Create camera inside the SubViewport for screenshots
	var screenshot_camera = Camera3D.new()
	screenshot_camera.name = "ScreenshotCamera"
	screenshot_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	screenshot_camera.size = island_size * 1.2
	screenshot_camera.far = 1000
	screenshot_camera.near = 0.1
	# Position camera at Y=15 to stay within grass shader fade range (16-24m)
	screenshot_camera.position = Vector3(center_x, 15, center_z)
	screenshot_camera.rotation_degrees = Vector3(-90, 0, 0)
	screenshot_viewport.add_child(screenshot_camera)

	get_tree().root.add_child(screenshot_viewport)
