class_name HardwareBenchmark
extends Node

## Hardware benchmark for automatic graphics profile detection
## Runs a quick render test and measures GPU performance to select optimal profile

signal benchmark_completed(profile: int, gpu_score: float, ram_gb: float)

# Profile thresholds (GPU score is render time in ms - lower is better)
# Selects the HIGHEST profile where ALL metrics meet requirements
const PROFILE_THRESHOLDS: Array[Dictionary] = [
	# Very Low (0) - No requirements, always available
	{"gpu_max_ms": INF, "ram_min_gb": 0.0, "name": "Very Low"},
	# Low (1)
	{"gpu_max_ms": 25.0, "ram_min_gb": 2.0, "name": "Low"},
	# Medium (2)
	{"gpu_max_ms": 15.0, "ram_min_gb": 4.0, "name": "Medium"},
	# High (3)
	{"gpu_max_ms": 8.0, "ram_min_gb": 6.0, "name": "High"},
]

# Benchmark configuration
const FRAMES_TO_RENDER: int = 60
const WARMUP_FRAMES: int = 10
const MESH_GRID_SIZE: int = 10  # Creates a 10x4x10 grid (400 meshes)
const MESH_GRID_HEIGHT: int = 4
const MESHES_PER_BATCH: int = 20  # Meshes created per frame to avoid UI freeze

# Benchmark state
var _is_running: bool = false
var _frame_times: Array[float] = []
var _frames_rendered: int = 0
var _benchmark_viewport: SubViewport = null
var _benchmark_start_time: int = 0


func _ready() -> void:
	set_process(false)


# gdlint:ignore = async-function-name
func run_benchmark() -> void:
	if _is_running:
		return

	_benchmark_start_time = Time.get_ticks_msec()
	print("[Startup] hardware_benchmark.run_benchmark start: %dms" % _benchmark_start_time)

	_is_running = true
	_frame_times.clear()
	_frames_rendered = 0

	# Create benchmark viewport and scene asynchronously to avoid UI freeze
	_setup_benchmark_viewport()
	await _create_benchmark_scene_async()

	print(
		(
			"[Startup] hardware_benchmark scene created: %dms"
			% (Time.get_ticks_msec() - _benchmark_start_time)
		)
	)

	# Enable render time measurement
	if _benchmark_viewport:
		var rid: RID = _benchmark_viewport.get_viewport_rid()
		RenderingServer.viewport_set_measure_render_time(rid, true)

	# Start processing frames
	set_process(true)
	print("[HardwareBenchmark] Starting benchmark...")


func _process(_delta: float) -> void:
	if not _is_running or not _benchmark_viewport:
		return

	# Measure render time for this frame
	var rid: RID = _benchmark_viewport.get_viewport_rid()
	var cpu_time: float = RenderingServer.viewport_get_measured_render_time_cpu(rid)
	var gpu_time: float = RenderingServer.viewport_get_measured_render_time_gpu(rid)
	var frame_time: float = maxf(cpu_time, gpu_time)

	# Skip first few frames (warm-up for shader compilation)
	if _frames_rendered >= WARMUP_FRAMES:
		_frame_times.append(frame_time)

	_frames_rendered += 1

	# Check if benchmark is complete
	if _frames_rendered >= FRAMES_TO_RENDER + WARMUP_FRAMES:
		_finish_benchmark()


func _finish_benchmark() -> void:
	set_process(false)
	_is_running = false

	var benchmark_duration: int = Time.get_ticks_msec() - _benchmark_start_time
	print(
		(
			"[Startup] hardware_benchmark._finish_benchmark: %dms (duration: %dms)"
			% [Time.get_ticks_msec(), benchmark_duration]
		)
	)

	# Calculate average GPU score (render time in ms)
	var gpu_score: float = _calculate_average_frame_time()

	# Get system RAM
	var ram_gb: float = _get_system_ram_gb()

	# Determine optimal profile
	var optimal_profile: int = _determine_profile(gpu_score, ram_gb)

	print(
		(
			"[HardwareBenchmark] Complete: GPU=%.1fms, RAM=%.1fGB -> Profile=%d (%s)"
			% [gpu_score, ram_gb, optimal_profile, PROFILE_THRESHOLDS[optimal_profile].name]
		)
	)

	# Cleanup
	_cleanup_benchmark_viewport()

	# Emit result
	benchmark_completed.emit(optimal_profile, gpu_score, ram_gb)


func _calculate_average_frame_time() -> float:
	if _frame_times.is_empty():
		return 999.0  # Very high score = low performance

	var total: float = 0.0
	for time in _frame_times:
		total += time
	return total / _frame_times.size()


func _get_system_ram_gb() -> float:
	# Try to get RAM from mobile plugins first (more reliable on mobile)
	var ram_mb: int = -1

	if DclAndroidPlugin.is_available():
		ram_mb = DclAndroidPlugin.get_total_ram_mb()
	elif DclIosPlugin.is_available():
		ram_mb = DclIosPlugin.get_total_ram_mb()

	# If mobile plugin returned valid RAM, use it
	if ram_mb > 0:
		return ram_mb / 1024.0  # Convert MB to GB

	# Fallback: try OS.get_memory_info() for desktop
	var memory_info: Dictionary = OS.get_memory_info()
	if memory_info.has("physical"):
		var physical_bytes: int = memory_info["physical"]
		# Validate the value is reasonable (> 512MB)
		if physical_bytes > 536870912:
			return physical_bytes / 1073741824.0  # Convert bytes to GB

	# Ultimate fallback: assume 4GB
	return 4.0


func _determine_profile(gpu_score: float, ram_gb: float) -> int:
	# Find the HIGHEST profile where ALL metrics meet requirements
	# Start from highest (3=High) and go down
	for i in range(PROFILE_THRESHOLDS.size() - 1, -1, -1):
		var threshold: Dictionary = PROFILE_THRESHOLDS[i]
		var gpu_ok: bool = gpu_score <= threshold.gpu_max_ms
		var ram_ok: bool = ram_gb >= threshold.ram_min_gb

		if gpu_ok and ram_ok:
			return i

	# Fallback to Very Low
	return 0


func _setup_benchmark_viewport() -> void:
	# Create a SubViewport for benchmark rendering
	_benchmark_viewport = SubViewport.new()
	_benchmark_viewport.size = Vector2i(1280, 720)  # Fixed size for consistent benchmark
	_benchmark_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	_benchmark_viewport.disable_3d = false
	_benchmark_viewport.own_world_3d = true  # Isolate benchmark from main scene
	add_child(_benchmark_viewport)


## Create benchmark scene asynchronously to avoid blocking the UI thread
## Yields between batches of mesh creation to allow UI updates
func _create_benchmark_scene_async() -> void:
	if not _benchmark_viewport:
		return

	# Add a camera
	var camera := Camera3D.new()
	camera.position = Vector3(0, 8, 20)
	camera.look_at(Vector3.ZERO)
	_benchmark_viewport.add_child(camera)

	# Add main directional light with shadows
	var sun := DirectionalLight3D.new()
	sun.position = Vector3(10, 20, 10)
	sun.look_at(Vector3.ZERO)
	sun.shadow_enabled = true
	sun.light_energy = 1.2
	sun.shadow_bias = 0.02
	_benchmark_viewport.add_child(sun)

	# Add secondary fill light
	var fill_light := DirectionalLight3D.new()
	fill_light.position = Vector3(-10, 5, -5)
	fill_light.look_at(Vector3.ZERO)
	fill_light.light_energy = 0.3
	_benchmark_viewport.add_child(fill_light)

	# Add multiple point lights for more realistic lighting
	for i in range(4):
		var point_light := OmniLight3D.new()
		var angle: float = i * PI / 2.0
		point_light.position = Vector3(cos(angle) * 8, 3, sin(angle) * 8)
		point_light.light_energy = 2.0
		point_light.omni_range = 10.0
		point_light.shadow_enabled = true
		_benchmark_viewport.add_child(point_light)

	# Add environment with bloom
	var env := WorldEnvironment.new()
	var environment := Environment.new()
	environment.background_mode = Environment.BG_SKY
	var sky := Sky.new()
	var sky_material := ProceduralSkyMaterial.new()
	sky_material.sky_top_color = Color(0.3, 0.5, 0.8)
	sky_material.sky_horizon_color = Color(0.6, 0.7, 0.9)
	sky_material.ground_bottom_color = Color(0.2, 0.2, 0.2)
	sky.sky_material = sky_material
	environment.sky = sky
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	environment.ambient_light_energy = 0.5
	environment.glow_enabled = true
	environment.glow_intensity = 0.8
	environment.glow_bloom = 0.3
	env.environment = environment
	_benchmark_viewport.add_child(env)

	# Allow UI to update after creating lights/environment
	await get_tree().process_frame

	# Create mesh types
	var box_mesh := BoxMesh.new()
	box_mesh.size = Vector3(0.8, 0.8, 0.8)

	var sphere_mesh := SphereMesh.new()
	sphere_mesh.radius = 0.5
	sphere_mesh.height = 1.0
	sphere_mesh.radial_segments = 32
	sphere_mesh.rings = 16

	var cylinder_mesh := CylinderMesh.new()
	cylinder_mesh.top_radius = 0.4
	cylinder_mesh.bottom_radius = 0.4
	cylinder_mesh.height = 1.0

	# Create materials with different shader paths for coverage
	var materials: Array[StandardMaterial3D] = []

	# Metallic (PBR metallic shader path)
	var mat1 := StandardMaterial3D.new()
	mat1.albedo_color = Color(0.9, 0.2, 0.2)
	mat1.metallic = 0.9
	mat1.roughness = 0.1
	materials.append(mat1)

	# Rough diffuse (PBR diffuse shader path)
	var mat2 := StandardMaterial3D.new()
	mat2.albedo_color = Color(0.2, 0.4, 0.9)
	mat2.metallic = 0.1
	mat2.roughness = 0.8
	materials.append(mat2)

	# Emissive (emission shader path)
	var mat3 := StandardMaterial3D.new()
	mat3.albedo_color = Color(0.2, 0.8, 0.3)
	mat3.emission_enabled = true
	mat3.emission = Color(0.2, 0.8, 0.3)
	mat3.emission_energy_multiplier = 2.0
	materials.append(mat3)

	# Transparent (alpha blend shader path)
	var mat4 := StandardMaterial3D.new()
	mat4.albedo_color = Color(0.8, 0.8, 0.2, 0.5)
	mat4.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat4.metallic = 0.5
	mat4.roughness = 0.3
	materials.append(mat4)

	var meshes: Array[Mesh] = [box_mesh, sphere_mesh, cylinder_mesh]

	# Create mesh grid using all materials and mesh types for shader coverage
	var mesh_count: int = 0
	var half_size: int = MESH_GRID_SIZE / 2
	for x in range(-half_size, half_size):
		for y in range(0, MESH_GRID_HEIGHT):
			for z in range(-half_size, half_size):
				var mesh_instance := MeshInstance3D.new()
				mesh_instance.mesh = meshes[mesh_count % meshes.size()]
				mesh_instance.position = Vector3(x * 1.5, y * 1.5, z * 1.5)
				mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
				mesh_instance.material_override = materials[mesh_count % materials.size()]
				_benchmark_viewport.add_child(mesh_instance)
				mesh_count += 1

				# Yield every MESHES_PER_BATCH to allow UI updates
				if mesh_count % MESHES_PER_BATCH == 0:
					await get_tree().process_frame


func _exit_tree() -> void:
	_cleanup_benchmark_viewport()


func _cleanup_benchmark_viewport() -> void:
	if _benchmark_viewport:
		_benchmark_viewport.queue_free()
		_benchmark_viewport = null
