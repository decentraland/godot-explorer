extends Node3D

enum Phase { INIT, LOADING, OFF, ON, DONE }

const AVATAR_SCENE = preload("res://src/decentraland_components/avatar/avatar.tscn")
const NUM_AVATARS: int = 50
const SPAWN_RADIUS_MIN: float = 5.0
const SPAWN_RADIUS_MAX: float = 50.0
const PHASE_WARMUP_SEC: float = 5.0
const PHASE_MEASURE_SEC: float = 15.0
const OUTPUT_PATH: String = "user://impostor_benchmark.log"

var _avatars: Array = []
var _phase: int = Phase.INIT
var _phase_start_ms: int = 0
var _frame_dts: Array[float] = []
var _results: Dictionary = {}

@onready var _label: Label = $UI/Label_Status
@onready var _camera: Camera3D = $Camera3D


func _ready() -> void:
	_log(
		(
			"benchmark: starting (N=%d, warmup=%ss, measure=%ss)"
			% [NUM_AVATARS, PHASE_WARMUP_SEC, PHASE_MEASURE_SEC]
		)
	)
	_phase = Phase.LOADING
	_async_setup_avatars()


# gdlint:ignore = async-function-name
func _async_setup_avatars() -> void:
	if Global.player_identity.get_profile_or_null() == null:
		Global.player_identity.set_default_profile()

	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if profile == null:
		_log("benchmark: error - no profile available")
		_phase = Phase.DONE
		return

	var rng := RandomNumberGenerator.new()
	rng.seed = 42

	for i in range(NUM_AVATARS):
		var angle: float = rng.randf_range(0.0, TAU)
		var dist: float = rng.randf_range(SPAWN_RADIUS_MIN, SPAWN_RADIUS_MAX)
		var avatar = AVATAR_SCENE.instantiate()
		avatar.position = Vector3(cos(angle) * dist, 0.0, sin(angle) * dist)
		avatar.rotation.y = rng.randf_range(0.0, TAU)
		add_child(avatar)
		_avatars.append(avatar)
		_label.text = "Loading avatar %d/%d..." % [i + 1, NUM_AVATARS]
		await avatar.async_update_avatar_from_profile(profile)
		await get_tree().process_frame

	_log("benchmark: %d avatars loaded" % _avatars.size())
	_start_phase(Phase.OFF, false)


func _start_phase(phase: int, impostors_enabled: bool) -> void:
	Global.get_config().avatar_impostors_enabled = impostors_enabled
	_phase = phase
	_phase_start_ms = Time.get_ticks_msec()
	_frame_dts.clear()
	_log("benchmark: phase %s (impostors=%s)" % [_phase_name(phase), str(impostors_enabled)])


func _phase_name(phase: int) -> String:
	match phase:
		Phase.OFF:
			return "OFF"
		Phase.ON:
			return "ON"
		_:
			return "?"


func _process(delta: float) -> void:
	if _phase != Phase.OFF and _phase != Phase.ON:
		return

	var elapsed_ms: int = Time.get_ticks_msec() - _phase_start_ms
	var elapsed_sec: float = float(elapsed_ms) / 1000.0

	if elapsed_sec < PHASE_WARMUP_SEC:
		_label.text = (
			"%s (warmup %ds / %ds)" % [_phase_name(_phase), int(elapsed_sec), int(PHASE_WARMUP_SEC)]
		)
		return

	if elapsed_sec < PHASE_WARMUP_SEC + PHASE_MEASURE_SEC:
		_frame_dts.append(delta)
		_label.text = (
			"%s (measure %ds / %ds, samples=%d)"
			% [
				_phase_name(_phase),
				int(elapsed_sec - PHASE_WARMUP_SEC),
				int(PHASE_MEASURE_SEC),
				_frame_dts.size()
			]
		)
		return

	_finish_phase()


func _finish_phase() -> void:
	var avg_dt: float = _avg(_frame_dts)
	var p99_dt: float = _percentile(_frame_dts, 0.99)
	var p1_dt: float = _percentile(_frame_dts, 0.01)
	var avg_fps: float = (1.0 / avg_dt) if avg_dt > 0.0 else 0.0
	var name: String = _phase_name(_phase)
	_results[name] = {
		"avg_fps": avg_fps,
		"avg_ms": avg_dt * 1000.0,
		"p99_ms": p99_dt * 1000.0,
		"p1_ms": p1_dt * 1000.0,
		"samples": _frame_dts.size(),
	}
	_log("benchmark: phase %s done — avg fps=%.2f" % [name, avg_fps])

	if _phase == Phase.OFF:
		_start_phase(Phase.ON, true)
	else:
		_phase = Phase.DONE
		_output_results()


func _output_results() -> void:
	var off: Dictionary = _results.get("OFF", {})
	var on: Dictionary = _results.get("ON", {})
	var off_fps: float = off.get("avg_fps", 0.0)
	var on_fps: float = on.get("avg_fps", 0.0)
	var delta_pct: float = ((on_fps - off_fps) / off_fps) * 100.0 if off_fps > 0.0 else 0.0

	var text := (
		"=== Avatar Impostor Benchmark ===\n"
		+ (
			"Avatars: %d (radius %.0f-%.0f m)\n"
			% [_avatars.size(), SPAWN_RADIUS_MIN, SPAWN_RADIUS_MAX]
		)
		+ "Warmup: %.0fs   Measure: %.0fs\n\n" % [PHASE_WARMUP_SEC, PHASE_MEASURE_SEC]
		+ "Impostors OFF:\n"
		+ "  avg fps : %.2f\n" % off_fps
		+ "  avg ms  : %.2f\n" % off.get("avg_ms", 0.0)
		+ "  p99 ms  : %.2f\n" % off.get("p99_ms", 0.0)
		+ "  p1 ms   : %.2f\n" % off.get("p1_ms", 0.0)
		+ "  samples : %d\n\n" % off.get("samples", 0)
		+ "Impostors ON:\n"
		+ "  avg fps : %.2f\n" % on_fps
		+ "  avg ms  : %.2f\n" % on.get("avg_ms", 0.0)
		+ "  p99 ms  : %.2f\n" % on.get("p99_ms", 0.0)
		+ "  p1 ms   : %.2f\n" % on.get("p1_ms", 0.0)
		+ "  samples : %d\n\n" % on.get("samples", 0)
		+ "Delta FPS: %+.1f%%\n" % delta_pct
	)

	print(text)
	_label.text = text

	var file := FileAccess.open(OUTPUT_PATH, FileAccess.WRITE)
	if file != null:
		file.store_string(text)
		file.close()
		_log("benchmark: results written to %s" % OUTPUT_PATH)


func _avg(arr: Array[float]) -> float:
	if arr.is_empty():
		return 0.0
	var sum: float = 0.0
	for v in arr:
		sum += v
	return sum / float(arr.size())


func _percentile(arr: Array[float], pct: float) -> float:
	if arr.is_empty():
		return 0.0
	var sorted := arr.duplicate()
	sorted.sort()
	var idx: int = clampi(int(pct * sorted.size()), 0, sorted.size() - 1)
	return sorted[idx]


func _log(msg: String) -> void:
	print("[ImpostorBenchmark] ", msg)
