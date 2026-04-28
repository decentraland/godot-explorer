extends Node3D

enum Phase { LOADING, OFF, ON, INTERACTIVE }

const AVATAR_SCENE = preload("res://src/decentraland_components/avatar/avatar.tscn")
const NUM_AVATARS: int = 200
const MAX_PARALLEL_LOADS: int = 10
const SPAWN_DEPTH_MIN: float = 5.0
const SPAWN_DEPTH_MAX: float = 60.0
const SPAWN_HORIZONTAL_FOV_FRACTION: float = 0.85
const FPS_SAMPLE_WINDOW: int = 60
const PHASE_WARMUP_SEC: float = 5.0
const PHASE_MEASURE_SEC: float = 15.0
const OUTPUT_PATH: String = "user://impostor_benchmark.log"
const ADDRESSES_CACHE_PATH: String = "user://impostor_benchmark_addresses.json"
const DEPLOYMENTS_URL_FMT: String = "https://peer.decentraland.org/content/deployments?entityType=profile&from=%d&to=%d&limit=400"

var _avatars: Array = []
var _frame_dts: Array[float] = []
var _phase: int = Phase.LOADING
var _phase_start_ms: int = 0
var _phase_dts: Array[float] = []
var _results: Dictionary = {}
var _auto_quit: bool = false
var _loads_in_flight: int = 0
var _loads_completed: int = 0
var _loads_failed: int = 0

@onready var _label_status: Label = $UI/Label_Status
@onready var _label_fps: Label = $UI/Label_FPS
@onready var _button_toggle: Button = $UI/Button_ToggleImpostors
@onready var _camera: Camera3D = $Camera3D


func _ready() -> void:
	Engine.max_fps = 0
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)

	_auto_quit = (
		Global.cli.avatar_impostor_benchmark
		or Global.has_meta("avatar_impostor_benchmark_auto_quit")
	)

	Global.get_config().avatar_impostors_enabled = true
	_refresh_button_label()
	_button_toggle.pressed.connect(_on_toggle_pressed)

	_label_fps.text = "FPS: --"
	_label_status.text = "Fetching real profile addresses from catalyst..."
	_log("benchmark: starting (N=%d, auto_quit=%s)" % [NUM_AVATARS, str(_auto_quit)])
	_async_run()


# gdlint:ignore = async-function-name
func _async_run() -> void:
	if Global.player_identity.get_profile_or_null() == null:
		Global.player_identity.set_default_profile()

	var addresses: Array = await _async_fetch_addresses()
	if addresses.is_empty():
		_label_status.text = "ERROR: failed to fetch profile addresses"
		return
	_log("benchmark: fetched %d unique addresses" % addresses.size())

	var rng := RandomNumberGenerator.new()
	rng.seed = 42

	var fov_v_rad: float = deg_to_rad(_camera.fov) if _camera != null else deg_to_rad(75.0)
	var aspect: float = (
		float(get_viewport().get_visible_rect().size.x)
		/ float(get_viewport().get_visible_rect().size.y)
	)
	var half_fov_h: float = atan(tan(fov_v_rad * 0.5) * aspect) * SPAWN_HORIZONTAL_FOV_FRACTION

	var count: int = mini(NUM_AVATARS, addresses.size())
	var next_idx: int = 0
	while _loads_completed + _loads_failed < count:
		while _loads_in_flight < MAX_PARALLEL_LOADS and next_idx < count:
			var depth: float = rng.randf_range(SPAWN_DEPTH_MIN, SPAWN_DEPTH_MAX)
			var x_offset: float = rng.randf_range(-1.0, 1.0) * depth * tan(half_fov_h)
			var avatar = AVATAR_SCENE.instantiate()
			avatar.hide_name = true
			avatar.position = Vector3(x_offset, 0.0, -depth)
			avatar.rotation.y = rng.randf_range(0.0, TAU)
			add_child(avatar)
			_avatars.append(avatar)
			_loads_in_flight += 1
			_async_load_one(avatar, addresses[next_idx])
			next_idx += 1
		_label_status.text = (
			"Loading %d/%d  (in flight %d, failed %d)"
			% [_loads_completed, count, _loads_in_flight, _loads_failed]
		)
		await get_tree().process_frame

	_log(
		(
			"benchmark: %d avatars loaded (%d failed) — starting auto bench"
			% [_loads_completed, _loads_failed]
		)
	)

	# Trigger a random looping emote on every avatar. Stress-tests the LOD
	# pipeline with emotes active (no longer bypasses impostor / cap).
	var emotes := ["dance", "disco", "tektonik", "robot", "tik", "raiseHand", "hammer"]
	for avatar in _avatars:
		if not is_instance_valid(avatar) or avatar.emote_controller == null:
			continue
		avatar.async_play_emote(emotes[rng.randi() % emotes.size()])

	# Pre-fetch impostor textures by enabling impostors and waiting until the
	# capture pipeline settles. Polls the diagnostics dict so we don't measure
	# while half the impostors are still texture_loaded=false (which would
	# make ON look artificially fast — invisible avatars).
	Global.get_config().avatar_impostors_enabled = true
	_refresh_button_label()
	var pre_fetch_deadline_ms: int = Time.get_ticks_msec() + 60_000
	while Time.get_ticks_msec() < pre_fetch_deadline_ms:
		var diag: Dictionary = Global.avatars.impostor_diagnostics()
		var loaded: int = diag.get("texture_loaded", 0)
		var total: int = diag.get("total_slots", 0)
		_label_status.text = "Pre-fetching impostors: %d/%d loaded" % [loaded, total]
		if total > 0 and loaded >= total:
			break
		await get_tree().create_timer(0.5).timeout
	var final_diag: Dictionary = Global.avatars.impostor_diagnostics()
	_log(
		(
			"benchmark: impostor pre-fetch done — %d/%d loaded, %d visible"
			% [
				final_diag.get("texture_loaded", 0),
				final_diag.get("total_slots", 0),
				final_diag.get("currently_visible", 0),
			]
		)
	)

	# Phase OFF
	_start_phase(Phase.OFF, false)
	await get_tree().create_timer(PHASE_WARMUP_SEC + PHASE_MEASURE_SEC).timeout
	_finish_phase()

	# Phase ON
	_start_phase(Phase.ON, true)
	await get_tree().create_timer(PHASE_WARMUP_SEC + PHASE_MEASURE_SEC).timeout
	_finish_phase()

	_emit_results()
	_phase = Phase.INTERACTIVE
	_label_status.text = ("Auto bench done — toggle the impostors button to compare in real time")

	if _auto_quit:
		_log("benchmark: auto-quit")
		await get_tree().create_timer(0.5).timeout
		get_tree().quit(0)


# gdlint:ignore = async-function-name
func _async_load_one(avatar: Node, address: String) -> void:
	var promise: Promise = Global.content_provider.fetch_profile(address)
	var profile_result = await PromiseUtils.async_awaiter(promise)
	if profile_result is PromiseError or not (profile_result is DclUserProfile):
		_log("benchmark: skipped %s — fetch failed" % address)
		_loads_failed += 1
		_loads_in_flight -= 1
		if is_instance_valid(avatar):
			_avatars.erase(avatar)
			avatar.queue_free()
		return
	if not is_instance_valid(avatar):
		_loads_failed += 1
		_loads_in_flight -= 1
		return
	await avatar.async_update_avatar_from_profile(profile_result)
	# Make sure the AnimationTree is awake so the avatar plays its default idle
	# instead of sitting in T-pose. The avatar's own _process toggles this
	# back off when LOD switches to FAR.
	var anim_tree: Node = avatar.get_node_or_null("AnimationTree")
	if anim_tree != null:
		anim_tree.active = true
	_loads_completed += 1
	_loads_in_flight -= 1


func _start_phase(phase: int, impostors_enabled: bool) -> void:
	Global.get_config().avatar_impostors_enabled = impostors_enabled
	_refresh_button_label()
	_phase = phase
	_phase_dts.clear()
	_phase_start_ms = Time.get_ticks_msec()
	var name: String = _phase_name(phase)
	_log("benchmark: phase %s (impostors=%s)" % [name, str(impostors_enabled)])
	_label_status.text = (
		"Phase %s — warmup %ds, measure %ds" % [name, int(PHASE_WARMUP_SEC), int(PHASE_MEASURE_SEC)]
	)


func _finish_phase() -> void:
	var avg_dt: float = _avg(_phase_dts)
	var p99_dt: float = _percentile(_phase_dts, 0.99)
	var p1_dt: float = _percentile(_phase_dts, 0.01)
	var avg_fps: float = (1.0 / avg_dt) if avg_dt > 0.0 else 0.0
	var name: String = _phase_name(_phase)
	_results[name] = {
		"avg_fps": avg_fps,
		"avg_ms": avg_dt * 1000.0,
		"p99_ms": p99_dt * 1000.0,
		"p1_ms": p1_dt * 1000.0,
		"samples": _phase_dts.size(),
	}
	_log("benchmark: phase %s done — avg fps=%.2f" % [name, avg_fps])


func _emit_results() -> void:
	var off: Dictionary = _results.get("OFF", {})
	var on: Dictionary = _results.get("ON", {})
	var off_fps: float = off.get("avg_fps", 0.0)
	var on_fps: float = on.get("avg_fps", 0.0)
	var delta_pct: float = ((on_fps - off_fps) / off_fps) * 100.0 if off_fps > 0.0 else 0.0

	var text := (
		"=== Avatar Impostor Benchmark ===\n"
		+ (
			"Avatars: %d (depth %.0f-%.0f m, all in frustum)\n"
			% [_avatars.size(), SPAWN_DEPTH_MIN, SPAWN_DEPTH_MAX]
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
	var f := FileAccess.open(OUTPUT_PATH, FileAccess.WRITE)
	if f != null:
		f.store_string(text)
		f.close()
		_log("benchmark: results written to %s" % OUTPUT_PATH)


# gdlint:ignore = async-function-name
func _async_fetch_addresses() -> Array:
	var seen: Dictionary = {}
	var addresses: Array = []

	for addr in _load_cached_addresses():
		if not seen.has(addr):
			seen[addr] = true
			addresses.append(addr)

	if addresses.size() >= NUM_AVATARS:
		_log(
			(
				"benchmark: using %d cached addresses from %s"
				% [addresses.size(), ADDRESSES_CACHE_PATH]
			)
		)
		return addresses

	var now_ms: int = int(Time.get_unix_time_from_system() * 1000.0)
	var to_ms: int = now_ms
	var max_pages: int = 12

	for page in range(max_pages):
		if addresses.size() >= NUM_AVATARS:
			break
		var from_ms: int = to_ms - 30 * 24 * 60 * 60 * 1000
		var url: String = DEPLOYMENTS_URL_FMT % [from_ms, to_ms]
		var promise: Promise = Global.http_requester.request_json(
			url, HTTPClient.METHOD_GET, "", {}
		)
		var result = await PromiseUtils.async_awaiter(promise)
		if result is PromiseError:
			_log("benchmark: deployments fetch failed: " + result.get_error())
			break

		var json = result.get_string_response_as_json()
		if not (json is Dictionary):
			break
		var deployments = json.get("deployments", [])
		if not (deployments is Array) or deployments.is_empty():
			break

		var oldest_ts: int = to_ms
		for d in deployments:
			var ts = d.get("entityTimestamp", 0)
			if ts is float or ts is int:
				oldest_ts = mini(oldest_ts, int(ts))
			var pointers = d.get("pointers", [])
			if pointers is Array and not pointers.is_empty():
				var addr: String = String(pointers[0]).to_lower()
				if addr.begins_with("0x") and addr.length() == 42 and not seen.has(addr):
					seen[addr] = true
					addresses.append(addr)
					if addresses.size() >= NUM_AVATARS:
						break

		_log("benchmark: page %d -> %d unique addresses so far" % [page, addresses.size()])
		if oldest_ts >= to_ms:
			break
		to_ms = oldest_ts - 1

	_save_cached_addresses(addresses)
	return addresses


func _load_cached_addresses() -> Array:
	if not FileAccess.file_exists(ADDRESSES_CACHE_PATH):
		return []
	var f := FileAccess.open(ADDRESSES_CACHE_PATH, FileAccess.READ)
	if f == null:
		return []
	var text: String = f.get_as_text()
	f.close()
	var parsed = JSON.parse_string(text)
	if not (parsed is Array):
		return []
	var out: Array = []
	for v in parsed:
		var s: String = String(v).to_lower()
		if s.begins_with("0x") and s.length() == 42:
			out.append(s)
	return out


func _save_cached_addresses(addresses: Array) -> void:
	var f := FileAccess.open(ADDRESSES_CACHE_PATH, FileAccess.WRITE)
	if f == null:
		_log("benchmark: failed to write address cache to %s" % ADDRESSES_CACHE_PATH)
		return
	f.store_string(JSON.stringify(addresses))
	f.close()
	_log("benchmark: cached %d addresses to %s" % [addresses.size(), ADDRESSES_CACHE_PATH])


func _on_toggle_pressed() -> void:
	var enabled: bool = not Global.get_config().avatar_impostors_enabled
	Global.get_config().avatar_impostors_enabled = enabled
	_refresh_button_label()
	_log("benchmark: impostors %s" % ("ON" if enabled else "OFF"))


func _refresh_button_label() -> void:
	var on: bool = Global.get_config().avatar_impostors_enabled
	_button_toggle.text = (
		"Impostors: ON (tap to disable)" if on else "Impostors: OFF (tap to enable)"
	)


func _process(delta: float) -> void:
	# Phase measurement (sample only after warmup)
	if _phase == Phase.OFF or _phase == Phase.ON:
		var elapsed: float = (Time.get_ticks_msec() - _phase_start_ms) / 1000.0
		if elapsed >= PHASE_WARMUP_SEC and elapsed < PHASE_WARMUP_SEC + PHASE_MEASURE_SEC:
			_phase_dts.append(delta)

	# Live FPS HUD
	_frame_dts.append(delta)
	if _frame_dts.size() > FPS_SAMPLE_WINDOW:
		_frame_dts.pop_front()
	if _frame_dts.size() >= 5:
		var avg_dt: float = 0.0
		for dt in _frame_dts:
			avg_dt += dt
		avg_dt /= float(_frame_dts.size())
		var fps: float = 1.0 / avg_dt if avg_dt > 0.0 else 0.0
		_label_fps.text = (
			"FPS: %5.1f   |   %5.2f ms   |   N=%d   |   %s"
			% [
				fps,
				avg_dt * 1000.0,
				_avatars.size(),
				"IMPOSTORS ON" if Global.get_config().avatar_impostors_enabled else "IMPOSTORS OFF"
			]
		)


func _phase_name(phase: int) -> String:
	match phase:
		Phase.OFF:
			return "OFF"
		Phase.ON:
			return "ON"
		_:
			return "?"


func _avg(arr: Array[float]) -> float:
	if arr.is_empty():
		return 0.0
	var s: float = 0.0
	for v in arr:
		s += v
	return s / float(arr.size())


func _percentile(arr: Array[float], pct: float) -> float:
	if arr.is_empty():
		return 0.0
	var sorted := arr.duplicate()
	sorted.sort()
	var idx: int = clampi(int(pct * sorted.size()), 0, sorted.size() - 1)
	return sorted[idx]


func _log(msg: String) -> void:
	print("[ImpostorBenchmark] ", msg)
