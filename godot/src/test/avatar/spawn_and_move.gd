class_name TestSpawnAndMoveAvatars
extends Node

const TEST_AVATAR_N: int = 50
const WARMUP_AVATARS: int = 5
const GAP_FRAMES: int = 10
const MAX_MEASURE_FRAMES: int = 600
const HICCUP_MS_A: float = 33.0
const HICCUP_MS_B: float = 50.0
const SYNC_PHASES: Array = ["hide", "mesh_duplicate", "toon", "color_facial"]
const OUTPUT_PATH: String = "user://output/avatar-spawn-bench.json"

var wearable_data = {}

var _ready_to_run: bool = false
var _spawn_index: int = 0
var _measuring: bool = false
var _gap_left: int = 0
var _measure_frames: int = 0
var _cur_alias: int = 0
var _cur_start_usec: int = 0
var _cur_frame_max_ms: float = 0.0
var _cur_over_a: int = 0
var _cur_over_b: int = 0
var _rows: Array = []
var _pending_avatar_node = null


# gdlint:ignore = async-function-name
func _ready():
	for wearable_id in Wearables.BASE_WEARABLES:
		var key = Wearables.get_base_avatar_urn(wearable_id)
		wearable_data[key] = null

	var promise = Global.content_provider.fetch_wearables(
		wearable_data.keys(), Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)

	for wearable_id in wearable_data:
		wearable_data[wearable_id] = Global.content_provider.get_wearable(wearable_id)
		if wearable_data[wearable_id] == null:
			printerr("Error loading wearable_id ", wearable_id)

	AvatarBuildProfiler.enabled = true
	Global.avatars.avatar_added.connect(_on_avatar_added)
	_ready_to_run = true


func _on_avatar_added(node):
	_pending_avatar_node = node


func set_wearable_data(_wearable_data):
	wearable_data = _wearable_data


func get_random_body():
	var to_pick = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable.get_category() == Wearables.Categories.BODY_SHAPE:
			to_pick.push_back(wearable_id)

	return to_pick.pick_random()


func get_random_wearable(category: String, body_shape_id: String):
	var to_pick = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable.get_category() == category:
			if Wearables.can_equip(wearable, body_shape_id):
				to_pick.push_back(wearable_id)

	return to_pick.pick_random()


func generate_random_address() -> String:
	var address_length = 42  # Adjust the length based on your needs
	var characters = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
	var address = ""
	for i in range(address_length):
		var random_index = randi() % characters.length()
		address += characters.substr(random_index, 1)

	return address


func _process(dt):
	if not _ready_to_run:
		return

	if _measuring:
		var ms: float = dt * 1000.0
		_cur_frame_max_ms = maxf(_cur_frame_max_ms, ms)
		if ms >= HICCUP_MS_A:
			_cur_over_a += 1
		if ms >= HICCUP_MS_B:
			_cur_over_b += 1
		_measure_frames += 1
		if _measure_frames >= MAX_MEASURE_FRAMES:
			printerr("Avatar #", _cur_alias, " never emitted avatar_loaded, skipping")
			_finish_current(true)
		return

	if _gap_left > 0:
		_gap_left -= 1
		return

	if _spawn_index >= TEST_AVATAR_N:
		_finalize_report()
		set_process(false)
		return

	_spawn_one()


func _spawn_one():
	var body_shape_id = get_random_body()

	var profile_data: DclUserProfile = DclUserProfile.new()
	var avatar_data: DclAvatarWireFormat = profile_data.get_avatar()

	profile_data.set_name("Avatar#" + str(_spawn_index))
	avatar_data.set_eyes_color(Color(randf(), randf(), randf()))
	avatar_data.set_hair_color(Color(randf(), randf(), randf()))
	avatar_data.set_skin_color(Color(0.8, 0.6078, 0.4667, 1))
	avatar_data.set_body_shape(body_shape_id)
	profile_data.set_avatar(avatar_data)
	var avatar_wearables := PackedStringArray(
		[
			get_random_wearable(Wearables.Categories.MOUTH, body_shape_id),
			get_random_wearable(Wearables.Categories.HAIR, body_shape_id),
			get_random_wearable(Wearables.Categories.UPPER_BODY, body_shape_id),
			get_random_wearable(Wearables.Categories.LOWER_BODY, body_shape_id),
			get_random_wearable(Wearables.Categories.FEET, body_shape_id),
			get_random_wearable(Wearables.Categories.EYES, body_shape_id),
		]
	)
	avatar_data.set_wearables(avatar_wearables)

	var initial_position := Vector3(randf_range(-10, 10), 0.0, randf_range(-10, 10)).normalized()
	var transform = Transform3D(Basis.IDENTITY, initial_position)
	var alias = 10000 + _spawn_index
	var address := generate_random_address()
	_pending_avatar_node = null
	Global.avatars.add_avatar(alias, address)

	var avatar_node = _pending_avatar_node
	if avatar_node == null:
		printerr("Avatar #", _spawn_index, " node not found after add_avatar, skipping")
		_spawn_index += 1
		return
	avatar_node.avatar_loaded.connect(_on_avatar_loaded, CONNECT_ONE_SHOT)

	_cur_alias = alias
	_cur_frame_max_ms = 0.0
	_cur_over_a = 0
	_cur_over_b = 0
	_measure_frames = 0
	_cur_start_usec = Time.get_ticks_usec()
	_measuring = true

	Global.avatars.update_dcl_avatar_by_alias(alias, profile_data)
	Global.avatars.update_avatar_transform_with_godot_transform(alias, transform)


func _on_avatar_loaded():
	_finish_current(false)


func _finish_current(timed_out: bool):
	var wall_ms := (Time.get_ticks_usec() - _cur_start_usec) / 1000.0
	var phases := AvatarBuildProfiler.finish()

	var sync_ms := 0.0
	for phase in SYNC_PHASES:
		sync_ms += float(phases.get(phase, 0)) / 1000.0

	if not timed_out and _spawn_index >= WARMUP_AVATARS:
		var phases_ms := {}
		for phase in phases:
			phases_ms[phase] = float(phases[phase]) / 1000.0
		(
			_rows
			. append(
				{
					"avatar": _spawn_index,
					"wall_ms": wall_ms,
					"max_frame_ms": _cur_frame_max_ms,
					"sync_ms": sync_ms,
					"frames_over_%dms" % int(HICCUP_MS_A): _cur_over_a,
					"frames_over_%dms" % int(HICCUP_MS_B): _cur_over_b,
					"phases_ms": phases_ms,
				}
			)
		)

	_measuring = false
	_gap_left = GAP_FRAMES
	_spawn_index += 1


func _percentile(sorted_values: Array, p: float) -> float:
	if sorted_values.is_empty():
		return 0.0
	var idx := int(ceil(p * sorted_values.size())) - 1
	idx = clampi(idx, 0, sorted_values.size() - 1)
	return sorted_values[idx]


func _finalize_report():
	if _rows.is_empty():
		print("[avatar-spawn-bench] no measured rows")
		return

	var max_frames := []
	var sync_values := []
	var wall_values := []
	var phase_totals := {}
	for row in _rows:
		max_frames.append(row["max_frame_ms"])
		sync_values.append(row["sync_ms"])
		wall_values.append(row["wall_ms"])
		for phase in row["phases_ms"]:
			phase_totals[phase] = (
				float(phase_totals.get(phase, 0.0)) + float(row["phases_ms"][phase])
			)
	max_frames.sort()
	sync_values.sort()
	wall_values.sort()

	var n := _rows.size()
	var phase_avg := {}
	for phase in phase_totals:
		phase_avg[phase] = phase_totals[phase] / n

	var aggregates := {
		"measured_avatars": n,
		"warmup_avatars": WARMUP_AVATARS,
		"max_frame_ms_p50": _percentile(max_frames, 0.50),
		"max_frame_ms_p95": _percentile(max_frames, 0.95),
		"max_frame_ms_p99": _percentile(max_frames, 0.99),
		"max_frame_ms_max": max_frames[n - 1],
		"sync_ms_p50": _percentile(sync_values, 0.50),
		"sync_ms_p95": _percentile(sync_values, 0.95),
		"wall_ms_p50": _percentile(wall_values, 0.50),
		"phase_avg_ms": phase_avg,
	}

	print("[avatar-spawn-bench] measured ", n, " avatars (", WARMUP_AVATARS, " warmup discarded)")
	print(
		(
			"  hiccup max_frame_ms  p50=%.1f  p95=%.1f  p99=%.1f  max=%.1f"
			% [
				aggregates["max_frame_ms_p50"],
				aggregates["max_frame_ms_p95"],
				aggregates["max_frame_ms_p99"],
				aggregates["max_frame_ms_max"],
			]
		)
	)
	print(
		(
			"  sync build cost ms   p50=%.1f  p95=%.1f"
			% [aggregates["sync_ms_p50"], aggregates["sync_ms_p95"]]
		)
	)
	var phase_keys := phase_avg.keys()
	phase_keys.sort_custom(func(a, b): return phase_avg[a] > phase_avg[b])
	print("  phase avg (ms, sorted by cost):")
	for phase in phase_keys:
		print("    %-16s %.2f" % [phase, phase_avg[phase]])

	var out := {"aggregates": aggregates, "rows": _rows}
	DirAccess.make_dir_recursive_absolute(OUTPUT_PATH.get_base_dir())
	var file := FileAccess.open(OUTPUT_PATH, FileAccess.WRITE)
	if file != null:
		file.store_string(JSON.stringify(out, "\t"))
		file.close()
		print(
			"[avatar-spawn-bench] report written to ", ProjectSettings.globalize_path(OUTPUT_PATH)
		)
	else:
		printerr("[avatar-spawn-bench] failed to write report to ", OUTPUT_PATH)
