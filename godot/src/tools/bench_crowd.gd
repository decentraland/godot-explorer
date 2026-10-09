class_name BenchCrowd
extends Node

## Fixed benchmark crowd for device runs (#3015), non-production only. `--bench-avatars=N` (or
## `bench-avatars=N` in the deep link) adds N walking avatars from filler_avatars.json after the
## loading screen; every SWAP_INTERVAL_SEC one of them changes to the next profile in the fixture,
## so each run carries the same avatar load and outfit churn whoever is online. N=0 only logs stats.
## Extra flags: `--bench-still` (no walking), `--bench-pause-scenes` (pause the scene runner).

const FIXTURE := "res://assets/bench/filler_avatars.json"
const ALIAS_BASE := 900000
const SPAWN_INTERVAL_SEC := 0.5
const SWAP_INTERVAL_SEC := 2.0
const MOVE_HZ := 10.0
const WALK_SPEED := 1.4
const RADIUS_MIN := 3.0
const RADIUS_MAX := 14.0
const STATS_INTERVAL_SEC := 5.0
const STATS_FILE := "user://bench_stats.log"

var target_count := 0
var _profiles: Array = []
var _fillers: Array[Dictionary] = []  # {alias, address, pos: Vector3, goal: Vector3}
var _next_profile := 0
var _next_swap := 0
var _swaps := 0
var _center := Vector3.ZERO
var _rng := RandomNumberGenerator.new()
var _spawn_clock := 0.0
var _swap_clock := 0.0
var _move_clock := 0.0
var _stats_clock := 0.0
var _frame_ms: PackedFloat32Array = []
var _active := false
var _walking := not OS.get_cmdline_user_args().has("--bench-still")


static func attach_if_requested(parent: Node) -> void:
	var count := requested_count()
	if count < 0:
		return
	var crowd := BenchCrowd.new()
	crowd.name = "BenchCrowd"
	crowd.target_count = count
	parent.add_child(crowd)


## -1 when not requested.
static func requested_count() -> int:
	if Global.is_production():
		return -1
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--bench-avatars="):
			return int(arg.get_slice("=", 1))
	if Global.deep_link_obj != null and Global.deep_link_obj.params.has("bench-avatars"):
		return int(Global.deep_link_obj.params["bench-avatars"])
	return -1


func _ready() -> void:
	_rng.seed = 3015
	var file := FileAccess.open(FIXTURE, FileAccess.READ)
	if file == null:
		printerr("BenchCrowd: missing ", FIXTURE)
		return
	_profiles = JSON.parse_string(file.get_as_text())
	Global.loading_finished.connect(_on_loading_finished, CONNECT_ONE_SHOT)


func _on_loading_finished() -> void:
	var player = get_parent().get("player")
	if player is Node3D:
		_center = player.global_position
	if OS.get_cmdline_user_args().has("--bench-pause-scenes"):
		Global.scene_runner.set_pause(true)
	_active = true


func _process(delta: float) -> void:
	_frame_ms.push_back(delta * 1000.0)
	_stats_clock += delta
	if _stats_clock >= STATS_INTERVAL_SEC:
		_stats_clock = 0.0
		_log_stats()
	if not _active or _profiles.is_empty():
		return
	if _fillers.size() < target_count:
		_spawn_clock += delta
		if _spawn_clock >= SPAWN_INTERVAL_SEC:
			_spawn_clock = 0.0
			_spawn_next()
	elif target_count > 0:
		_swap_clock += delta
		if _swap_clock >= SWAP_INTERVAL_SEC:
			_swap_clock = 0.0
			_swap_next()
	_move_clock += delta
	if _walking and _move_clock >= 1.0 / MOVE_HZ:
		_walk(_move_clock)
		_move_clock = 0.0


func _spawn_next() -> void:
	var index := _fillers.size()
	var alias := ALIAS_BASE + index
	var address := "0xbe0c%036x" % (index + 1)
	var pos := _random_point()
	Global.avatars.add_avatar(alias, address)
	var filler := {"alias": alias, "address": address, "pos": pos, "goal": _random_point()}
	_fillers.push_back(filler)
	_apply_profile(filler)
	Global.avatars.update_avatar_transform_with_godot_transform(alias, Transform3D(Basis(), pos))


# Outfit churn: the oldest-changed filler takes the next fixture profile.
func _swap_next() -> void:
	var filler: Dictionary = _fillers[_next_swap % _fillers.size()]
	_next_swap += 1
	_swaps += 1
	_apply_profile(filler)


func _apply_profile(filler: Dictionary) -> void:
	var source: Dictionary = _profiles[_next_profile % _profiles.size()]
	_next_profile += 1
	var content: Dictionary = source.duplicate(true)
	# Fields the profile struct requires; the fixture only keeps the look.
	(
		content
		. merge(
			{
				"userId": filler.address,
				"ethAddress": filler.address,
				"description": "",
				"tutorialStep": 0,
				"version": _next_profile,
			},
			true
		)
	)
	var profile := DclUserProfile.from_godot_dictionary(
		{
			"version": _next_profile,
			"content": content,
			"base_url": Global.realm.get_profile_content_url()
		}
	)
	Global.avatars.update_dcl_avatar_by_alias(filler.alias, profile)
	DclProfiler.mark("BenchCrowd::profile", "alias=%d name=%s" % [filler.alias, source.name])


func _walk(dt: float) -> void:
	for f in _fillers:
		var to_goal: Vector3 = f.goal - f.pos
		to_goal.y = 0.0
		if to_goal.length() < 0.3:
			f.goal = _random_point()
			continue
		var step := to_goal.normalized() * minf(WALK_SPEED * dt, to_goal.length())
		f.pos += step
		var basis := Basis.looking_at(step.normalized(), Vector3.UP, true)
		Global.avatars.update_avatar_transform_with_godot_transform(
			f.alias, Transform3D(basis, f.pos)
		)


func _random_point() -> Vector3:
	var angle := _rng.randf() * TAU
	var radius := _rng.randf_range(RADIUS_MIN, RADIUS_MAX)
	return _center + Vector3(cos(angle) * radius, 0.0, sin(angle) * radius)


# One greppable line per interval: memory, pipelines, Godot objects and the frame-time spread.
func _log_stats() -> void:
	var rs := RenderingServer
	var frames := _frame_ms.duplicate()
	frames.sort()
	_frame_ms.clear()
	var count := frames.size()
	var over75 := 0
	var over100 := 0
	var over200 := 0
	for ms in frames:
		over75 += 1 if ms > 75.0 else 0
		over100 += 1 if ms > 100.0 else 0
		over200 += 1 if ms > 200.0 else 0
	var values := [
		Time.get_ticks_msec() / 1000,
		Global.scene_runner.get_process_memory_mb(),
		rs.get_rendering_info(RenderingServer.RENDERING_INFO_VIDEO_MEM_USED) >> 20,
		Global.avatars.get_avatars_count(),
		_swaps,
		rs.get_rendering_info(RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SURFACE),
		rs.get_rendering_info(RenderingServer.RENDERING_INFO_PIPELINE_COMPILATIONS_SPECIALIZATION),
		OS.get_static_memory_usage() >> 20,
		Performance.get_monitor(Performance.OBJECT_COUNT),
		count,
		frames[count / 2] if count > 0 else 0.0,
		frames[int(count * 0.9)] if count > 0 else 0.0,
		frames[count - 1] if count > 0 else 0.0,
		over75,
		over100,
		over200,
	]
	var line: String = (
		(
			"[BenchStats] t=%d mem=%d vmem=%d avatars=%d swaps=%d pso_surface=%d pso_spec=%d"
			+ " godot_heap=%d objects=%d frames=%d p50=%.1f p90=%.1f max=%.1f"
			+ " over75=%d over100=%d over200=%d"
		)
		% values
	)
	print(line)
	# Android drops log lines under load; the file survives for `adb run-as` pulls.
	var f := FileAccess.open(STATS_FILE, FileAccess.READ_WRITE)
	if f == null:
		f = FileAccess.open(STATS_FILE, FileAccess.WRITE)
	if f != null:
		f.seek_end()
		f.store_line(line)
