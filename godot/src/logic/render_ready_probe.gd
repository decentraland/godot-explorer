extends Node

## Times how long the meshes in view stay undrawn (async shader compilation skips a surface until
## its pipeline compiles) after the loading screen lifts or a graphics setting changes, and reports
## it as a RENDER_READY metric, a `[RenderReady]` log line and a Sentry breadcrumb.

const TIMEOUT_MSEC := 15000
const MAX_SAMPLES := 64
const DRAW_READY_METHOD := &"instance_geometry_is_draw_ready"
# What a graphics profile applies that changes scene shader variants or render passes.
const SETTINGS_PARAMS := [
	ConfigData.ConfigParams.GRAPHIC_PROFILE,
	ConfigData.ConfigParams.ANTI_ALIASING,
	ConfigData.ConfigParams.SHADOW_QUALITY,
	ConfigData.ConfigParams.BLOOM_QUALITY,
	ConfigData.ConfigParams.SKY_BOX,
	ConfigData.ConfigParams.RESOLUTION_3D_SCALE,
	ConfigData.ConfigParams.PARTICLE_QUALITY,
]
const PIPELINE_COUNTERS := {
	"pipelines_mesh": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_MESH,
	"pipelines_surface": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SURFACE,
	"pipelines_draw": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_DRAW,
	"pipelines_specialization": RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SPECIALIZATION,
}

var _reason := ""
var _profile := -1
var _from_profile := -1
var _menu_open := false
var _triggered_msec := 0
var _start_msec := -1
var _last_frame_msec := 0
var _max_frame_msec := 0
var _frames := 0
var _pipelines_before := {}
var _sample := PackedInt64Array()
var _ready_at_start_pct := -1
var _t50_msec := -1
var _t90_msec := -1


func _ready() -> void:
	set_process(false)
	if not RenderingServer.has_method(DRAW_READY_METHOD):
		return
	_profile = Global.get_config().graphic_profile
	Global.loading_finished.connect(_start.bind("loading"))
	Global.get_config().param_changed.connect(_on_config_param_changed)


func _on_config_param_changed(param: int) -> void:
	if param in SETTINGS_PARAMS:
		_start("settings")


# A profile apply emits several settings in one frame; each restarts the measurement, and the
# profile is set last, so `_from_profile` still holds the one before the apply.
func _start(reason: String) -> void:
	if not Global.feature_flags.is_enabled(FeatureFlags.FLAG_RENDER_READY_PROBE):
		return
	_reason = reason
	_from_profile = _profile
	_profile = Global.get_config().graphic_profile
	_menu_open = get_viewport().disable_3d
	_triggered_msec = Time.get_ticks_msec()
	_start_msec = -1
	_frames = 0
	_max_frame_msec = 0
	_ready_at_start_pct = -1
	_t50_msec = -1
	_t90_msec = -1
	for key in PIPELINE_COUNTERS:
		_pipelines_before[key] = RenderingServer.get_rendering_info(PIPELINE_COUNTERS[key])
	set_process(true)


func _process(_delta: float) -> void:
	var now := Time.get_ticks_msec()
	var viewport := get_viewport()
	var camera := viewport.get_camera_3d()
	if camera == null:
		set_process(false)
		return
	if _start_msec < 0:
		# Settings are changed from the menu, which turns 3D off: their cost lands once it closes.
		if viewport.disable_3d:
			return
		_start_msec = now if _menu_open else _triggered_msec
		_last_frame_msec = now
		_sample = _sample_meshes(camera)
		if _sample.is_empty():
			set_process(false)
		# Readiness describes the last rendered frame, so the first check waits for one.
		return

	_frames += 1
	_max_frame_msec = maxi(_max_frame_msec, now - _last_frame_msec)
	_last_frame_msec = now
	var planes := camera.get_frustum()
	var in_view := 0
	var drawn := 0
	for id in _sample:
		var mesh := instance_from_id(id) as MeshInstance3D
		if mesh == null or not mesh.is_visible_in_tree():
			continue
		# Readiness needs a color-pass draw, so meshes the camera turned away from don't count.
		if not _in_frustum(planes, mesh.global_transform * mesh.get_aabb()):
			continue
		in_view += 1
		if RenderingServer.call(DRAW_READY_METHOD, mesh.get_instance()):
			drawn += 1

	var elapsed := now - _start_msec
	var fraction := float(drawn) / in_view if in_view > 0 else 0.0
	if _ready_at_start_pct < 0:
		_ready_at_start_pct = roundi(fraction * 100.0)
	if _t50_msec < 0 and fraction >= 0.5:
		_t50_msec = elapsed
	if _t90_msec < 0 and fraction >= 0.9:
		_t90_msec = elapsed
	if fraction >= 1.0:
		_report(in_view, drawn, elapsed)
	elif elapsed >= TIMEOUT_MSEC:
		_report(in_view, drawn, -1)


func _report(in_view: int, drawn: int, t100_msec: int) -> void:
	set_process(false)
	var config := Global.get_config()
	var pos: Vector2i = Global.scene_fetcher.current_position
	var data := {
		"reason": _reason,
		"profile": _profile,
		"from_profile": _from_profile,
		"anti_aliasing": config.anti_aliasing,
		"shadow_quality": config.shadow_quality,
		"bloom_quality": config.bloom_quality,
		"async_shaders": Global.feature_flags.is_async_shader_compilation_enabled(),
		"samples": _sample.size(),
		"in_view": in_view,
		"not_drawn": in_view - drawn,
		"ready_at_start_pct": _ready_at_start_pct,
		"t50_ms": _t50_msec,
		"t90_ms": _t90_msec,
		"t100_ms": t100_msec,
		"menu_ms": _start_msec - _triggered_msec,
		"frames": _frames,
		"max_frame_ms": _max_frame_msec,
		"launch_count": config.review_session_count,
		"scene_id": Global.scene_fetcher.current_scene_entity_id,
		"position": "%d,%d" % [pos.x, pos.y],
	}
	for key in PIPELINE_COUNTERS:
		data[key] = (
			RenderingServer.get_rendering_info(PIPELINE_COUNTERS[key]) - _pipelines_before[key]
		)
	var json := JSON.stringify(data)
	print("[RenderReady] ", json)
	Global.metrics.track_screen_viewed("RENDER_READY", json)
	if SentrySDK.is_enabled():
		var crumb := SentryBreadcrumb.create("Render ready: %s" % _reason)
		crumb.category = "render"
		crumb.set_data(data)
		SentrySDK.add_breadcrumb(crumb)


# The nearest meshes the camera would draw, as object ids so a freed mesh just drops out.
static func _sample_meshes(camera: Camera3D) -> PackedInt64Array:
	var eye := camera.global_position
	var candidates: Array = []
	var culled := RenderingServer.instances_cull_convex(
		camera.get_frustum(), camera.get_world_3d().scenario
	)
	for id in culled:
		var mesh := instance_from_id(id) as MeshInstance3D
		if mesh == null or mesh.mesh == null or not mesh.is_visible_in_tree():
			continue
		# Never drawn by this camera (other layer, shadow-only) or drawn only within a range: they
		# would never be ready.
		if (mesh.layers & camera.cull_mask) == 0:
			continue
		if mesh.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY:
			continue
		if mesh.visibility_range_end > 0.0:
			continue
		candidates.push_back([eye.distance_squared_to(mesh.global_position), id])
	candidates.sort_custom(func(a, b): return a[0] < b[0])
	var ids := PackedInt64Array()
	for candidate in candidates.slice(0, MAX_SAMPLES):
		ids.push_back(candidate[1])
	return ids


static func _in_frustum(planes: Array[Plane], box: AABB) -> bool:
	for plane in planes:
		if plane.is_point_over(box.get_support(-plane.normal)):
			return false
	return true
