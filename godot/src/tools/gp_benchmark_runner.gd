## Genesis Plaza profiling benchmark (issue #1862).
##
## Activated by --gp-benchmark (desktop CLI) or `gp-benchmark=true` deep link
## param (mobile). Knobs (durations, toggles, tag, output path) come from
## godot/bench/genesis_plaza.config.json so the explorer's CLI surface stays
## small. Realm and location come from --realm/--location (desktop) or
## preview/realm/position deep link params (mobile).
##
## Flow:
##   1. Wait until explorer.tscn is the active scene; apply scene-runner toggles.
##   2. Wait for scene_runner.loading_complete — Rust signal that fires only on
##      genuine session completion, not when the 90s loading-screen timeout
##      hides the screen.
##   3. Wait warmup_seconds, then sample for sample_seconds.
##   4. Write JSON result and dump it to stdout (so it surfaces in adb logcat
##      on Android, where user:// lives in the app's private sandbox).

extends Node

const CONFIG_PATH := "res://bench/genesis_plaza.config.json"
const EXPLORER_SCENE := "res://src/ui/explorer.tscn"
const LOAD_TIMEOUT_MS := 1800000  # 30 min hard cap; mobile cold-start is slow
# Canonical benchmark pose. The scene's spawn point randomizes within a
# range (GP scene.json x[45-56] z[20.6-30]) so two devices land in
# different spots. We need a fixed viewpoint for the screenshot sanity
# check. Values are spawn-range midpoint expressed in Godot world coords:
# DCL local (50.5, 0, 25.3) on base parcel (-3,-2) → world (2.5, 0, 6.7).
const BENCH_POSE_POSITION := Vector3(2.5, 0.5, 6.7)
const BENCH_POSE_LOOK_AT := Vector3(8.0, 1.5, -28.0)

var config: Dictionary = {}
var samples: Array = []
var phase_started_at_ms: int = 0
var phase: String = "init"
var loading_complete_seen: bool = false
var pinned_transform: Transform3D
var pinned_camera_basis: Basis
var pose_pinned: bool = false


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS

	config = _load_config()
	if config.is_empty():
		push_error("GP Benchmark: failed to load %s; aborting" % CONFIG_PATH)
		queue_free()
		return

	_apply_deeplink_overrides()

	_log(
		(
			"config loaded: tag=%s warmup=%ds sample=%ds disable_tweens=%s disable_transforms=%s"
			% [
				config.get("tag", ""),
				int(config.get("warmup_seconds", 0)),
				int(config.get("sample_seconds", 0)),
				_toggle("disable_tweens"),
				_toggle("disable_transforms"),
			]
		)
	)

	Global.scene_runner.loading_complete.connect(_on_loading_complete)
	_set_phase("waiting_for_explorer")


func _on_loading_complete(session_id: int) -> void:
	# scene_fetcher emits loading_complete(-1) as a fallback when there are no
	# loadable scenes yet — that fires immediately on cold start, before any
	# scene data exists. Ignore it; only the genuine session completion
	# (positive id, emitted from Rust scene_manager) means scenes are loaded.
	if session_id < 0:
		return
	loading_complete_seen = true
	_log("scene_runner.loading_complete session=%d" % session_id)


func _process(_delta: float) -> void:
	match phase:
		"waiting_for_explorer":
			var current = get_tree().current_scene
			if current != null and current.scene_file_path == EXPLORER_SCENE:
				_apply_toggles()
				_set_phase("waiting_for_load")
		"waiting_for_load":
			if loading_complete_seen:
				_pin_pose()
				_set_phase("warmup")
			elif _phase_elapsed_ms() >= LOAD_TIMEOUT_MS:
				_log("WARN: loading_complete never fired in %d ms; aborting" % LOAD_TIMEOUT_MS)
				_write_error("loading_timeout")
				_async_force_quit(2)
		"warmup":
			_enforce_pinned_pose()
			if _phase_elapsed_ms() >= int(config.get("warmup_seconds", 30)) * 1000:
				# Reset per-state CPU timing + CRDT throughput counters so the
				# sampling-window numbers aren't polluted by load-time spikes.
				Global.scene_runner.reset_state_timing()
				Global.scene_runner.reset_crdt_metrics()
				_set_phase("sampling")
		"sampling":
			_enforce_pinned_pose()
			samples.append(_collect_sample())
			if _phase_elapsed_ms() >= int(config.get("sample_seconds", 30)) * 1000:
				_finish()


func _collect_sample() -> Dictionary:
	return {
		"t_ms": _phase_elapsed_ms(),
		"fps": Performance.get_monitor(Performance.TIME_FPS),
		"frame_time_process_ms": Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0,
		"frame_time_physics_ms": Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0,
		"memory_static_mb": Performance.get_monitor(Performance.MEMORY_STATIC) / 1048576.0,
		"memory_rss_mb": OS.get_static_memory_usage() / 1048576.0,
		"memory_peak_mb": OS.get_static_memory_peak_usage() / 1048576.0,
		"video_mem_mb": Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1048576.0,
		"texture_mem_mb": Performance.get_monitor(Performance.RENDER_TEXTURE_MEM_USED) / 1048576.0,
		"buffer_mem_mb": Performance.get_monitor(Performance.RENDER_BUFFER_MEM_USED) / 1048576.0,
		"object_count": Performance.get_monitor(Performance.OBJECT_COUNT),
		"resource_count": Performance.get_monitor(Performance.OBJECT_RESOURCE_COUNT),
		"node_count": Performance.get_monitor(Performance.OBJECT_NODE_COUNT),
		"orphan_node_count": Performance.get_monitor(Performance.OBJECT_ORPHAN_NODE_COUNT),
		"draw_calls": Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
		"render_objects_in_frame":
		Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME),
		"primitives": Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME),
		"physics_active_objects": Performance.get_monitor(Performance.PHYSICS_3D_ACTIVE_OBJECTS),
		"physics_collision_pairs": Performance.get_monitor(Performance.PHYSICS_3D_COLLISION_PAIRS),
		"physics_island_count": Performance.get_monitor(Performance.PHYSICS_3D_ISLAND_COUNT),
		"loaded_scene_count":
		Global.scene_fetcher.loaded_scenes.size() if Global.scene_fetcher != null else 0,
	}


func _count_node_types() -> Dictionary:
	# Walks the SceneTree once at end of sampling. Tells us how much of the
	# 21k node count is actually render-related (MeshInstance3D, etc) and
	# therefore convertible to RenderingServer-direct, vs UI/logic nodes.
	var counts := {}
	var mesh_resource_ids := {}
	var stack: Array = [get_tree().root]
	while not stack.is_empty():
		var n: Node = stack.pop_back()
		var t := n.get_class()
		counts[t] = counts.get(t, 0) + 1
		if n is MeshInstance3D:
			var mesh := (n as MeshInstance3D).mesh
			if mesh != null:
				var rid := mesh.get_rid()
				mesh_resource_ids[rid] = mesh_resource_ids.get(rid, 0) + 1
		for c in n.get_children():
			stack.push_back(c)
	# How many MeshInstance3D instances share each unique Mesh resource —
	# anything > 1 is a candidate for MultiMesh batching.
	var dup_buckets := {"unique": 0, "dup_2_to_5": 0, "dup_6_to_20": 0, "dup_21_plus": 0}
	for rid in mesh_resource_ids:
		var n: int = mesh_resource_ids[rid]
		if n == 1:
			dup_buckets.unique += 1
		elif n <= 5:
			dup_buckets.dup_2_to_5 += 1
		elif n <= 20:
			dup_buckets.dup_6_to_20 += 1
		else:
			dup_buckets.dup_21_plus += 1
	counts["_unique_meshes"] = mesh_resource_ids.size()
	counts["_mesh_dup_buckets"] = dup_buckets
	return counts


func _finish() -> void:
	_log("sampling done: %d samples" % samples.size())
	_set_phase("done")

	var node_types := _count_node_types()
	# Per-state CPU timing accumulated during the sampling window only.
	# Format: "State=us(count)\n..." (newline-separated). See
	# lib/src/scene_runner/update_scene.rs::drain_state_timing.
	var state_timing: String = Global.scene_runner.drain_state_timing()
	# CRDT cross-boundary throughput during sampling. See
	# lib/src/dcl/js/engine.rs::drain_crdt_metrics.
	var crdt_metrics: String = Global.scene_runner.drain_crdt_metrics()
	# Per-component-id breakdown of dirty entries on the Rust→V8 path.
	# Identifies which SDK7 components dominate the round-trip pressure.
	var crdt_component_breakdown: String = Global.scene_runner.drain_crdt_component_breakdown()

	var result := {
		"tag": config.get("tag", ""),
		"genesis_plaza_commit": config.get("genesis_plaza_commit", ""),
		"toggles": config.get("toggles", {}),
		"node_type_breakdown": node_types,
		"state_timing_us": state_timing,
		"crdt_metrics": crdt_metrics,
		"crdt_component_breakdown": crdt_component_breakdown,
		"warmup_seconds": int(config.get("warmup_seconds", 0)),
		"sample_seconds": int(config.get("sample_seconds", 0)),
		"samples_collected": samples.size(),
		"build_mode": "debug" if OS.is_debug_build() else "release",
		"platform": OS.get_name(),
		"summary": _summarize(samples),
		"samples": samples,
	}

	_write_result(result)
	# Mirror to a public path on Android so it's pullable from a
	# non-debuggable release APK (user:// lives in the app's private sandbox).
	if OS.get_name() == "Android":
		_write_to_public_path(result)
	# Dump summary to stdout (logcat) one field per line — logcat truncates
	# multi-KB lines, and a long single-line JSON loses everything past ~4KB.
	print("[GP Benchmark] BEGIN_RESULT_JSON")
	for key in ["tag", "platform", "build_mode", "genesis_plaza_commit", "samples_collected"]:
		print("[GP Benchmark] %s=%s" % [key, str(result.get(key, ""))])
	for stat_key in result.get("summary", {}).keys():
		var stat: Dictionary = result["summary"][stat_key]
		print(
			(
				"[GP Benchmark] %s mean=%.3f p50=%.3f p95=%.3f min=%.3f max=%.3f"
				% [stat_key, stat.mean, stat.p50, stat.p95, stat.min, stat.max]
			)
		)
	# Top-15 node types — quick read on whether the 21k node_count is dominated
	# by MeshInstance3D (render workload, RS-convertible) or by UI/logic nodes.
	var sorted_types: Array = []
	for k in node_types.keys():
		if str(k).begins_with("_"):
			continue
		sorted_types.append([k, node_types[k]])
	sorted_types.sort_custom(func(a, b): return a[1] > b[1])
	for i in range(min(15, sorted_types.size())):
		print("[GP Benchmark] node_type %s=%d" % [sorted_types[i][0], sorted_types[i][1]])
	print("[GP Benchmark] unique_meshes=%d" % node_types.get("_unique_meshes", 0))
	var dup: Dictionary = node_types.get("_mesh_dup_buckets", {})
	print(
		(
			"[GP Benchmark] mesh_dups unique=%d 2-5=%d 6-20=%d 21+=%d"
			% [
				dup.get("unique", 0),
				dup.get("dup_2_to_5", 0),
				dup.get("dup_6_to_20", 0),
				dup.get("dup_21_plus", 0),
			]
		)
	)
	print("[GP Benchmark] END_RESULT_JSON")
	# Sanity-check screenshot. Compared against the prior run's image by
	# scripts/bench/compare_screenshots.py — if it diverges too far the run
	# is flagged (different scene loaded, character moved, asset failed).
	# Runs LAST so a slow/failed screenshot can't block result delivery, and
	# the PNG encode (~200-500 ms zlib on 1080p) doesn't contaminate the
	# preceding sample window in profiles.
	_save_screenshot()
	_async_force_quit(0)


# get_tree().quit() schedules a clean exit but iOS sometimes ignores it
# (the OS suspends instead of terminating). Hammer with OS.kill as a
# fallback after a short delay to guarantee the client closes between
# matrix runs.
func _async_force_quit(exit_code: int) -> void:
	get_tree().quit(exit_code)
	await get_tree().create_timer(2.0).timeout
	OS.kill(OS.get_process_id())


func _write_to_public_path(result: Dictionary) -> void:
	var public_dir := "/sdcard/Download/gp-benchmark"
	DirAccess.make_dir_recursive_absolute(public_dir)
	var tag: String = config.get("tag", "result")
	var path := "%s/%s.json" % [public_dir, tag]
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		_log("WARN: cannot write %s (storage permission?)" % path)
		return
	f.store_string(JSON.stringify(result, "\t"))
	f.close()
	_log("mirrored result to %s" % path)


func _save_screenshot() -> void:
	var vp := get_viewport()
	if vp == null:
		_log("WARN: no viewport for screenshot")
		return
	var img := vp.get_texture().get_image()
	if img == null:
		_log("WARN: viewport returned null image")
		return
	var tag: String = config.get("tag", "result")
	var user_path := "user://output/gp-benchmark/%s.png" % tag
	DirAccess.make_dir_recursive_absolute(user_path.get_base_dir())
	if img.save_png(user_path) == OK:
		_log("screenshot -> %s" % user_path)
	if OS.get_name() == "Android":
		var public_path := "/sdcard/Download/gp-benchmark/%s.png" % tag
		if img.save_png(public_path) == OK:
			_log("mirrored screenshot to %s" % public_path)


func _summarize(s: Array) -> Dictionary:
	if s.is_empty():
		return {}

	var keys := [
		"fps",
		"frame_time_process_ms",
		"frame_time_physics_ms",
		"memory_static_mb",
		"memory_rss_mb",
		"memory_peak_mb",
		"video_mem_mb",
		"texture_mem_mb",
		"buffer_mem_mb",
		"resource_count",
		"render_objects_in_frame",
		"physics_active_objects",
		"physics_collision_pairs",
		"physics_island_count",
		"loaded_scene_count",
		"node_count",
		"draw_calls",
		"primitives",
	]
	var out := {}
	for k in keys:
		var values: Array = []
		for sample in s:
			values.append(float(sample.get(k, 0.0)))
		values.sort()
		var n: int = values.size()
		var sum: float = 0.0
		for v in values:
			sum += v
		out[k] = {
			"mean": sum / n,
			"min": values[0],
			"p50": values[n / 2],
			"p95": values[int(n * 0.95)],
			"max": values[n - 1],
		}
	return out


func _write_error(reason: String) -> void:
	var path: String = config.get("output_path", "user://output/gp-benchmark/result.json")
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		return
	(
		f
		. store_string(
			(
				JSON
				. stringify(
					{
						"tag": config.get("tag", ""),
						"error": reason,
						"phase": phase,
						"loading_complete_seen": loading_complete_seen,
					},
					"\t"
				)
			)
		)
	)
	f.close()


func _write_result(result: Dictionary) -> void:
	var path: String = config.get("output_path", "user://output/gp-benchmark/result.json")
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		push_error("GP Benchmark: cannot open %s for writing" % path)
		return
	f.store_string(JSON.stringify(result, "\t"))
	f.close()
	_log("wrote %s" % path)


## Pin to a hardcoded canonical pose so warmup+sampling render the exact
## same viewpoint across devices. The scene's `get_global_spawn_position`
## randomizes within the scene.json range, which means two devices spawn
## at different points. For sanity-check screenshots we need bit-for-bit
## identical viewpoints.
func _pin_pose() -> void:
	var player := _get_player_node()
	if player == null:
		_log("WARN: no player node when pinning pose")
		return
	player.global_transform.origin = BENCH_POSE_POSITION
	player.look_at(BENCH_POSE_LOOK_AT)
	pinned_transform = player.global_transform
	if Global.player_camera_node != null:
		pinned_camera_basis = Global.player_camera_node.global_transform.basis
	pose_pinned = true
	_log("pose pinned: pos=%s look=%s" % [str(BENCH_POSE_POSITION), str(BENCH_POSE_LOOK_AT)])


func _enforce_pinned_pose() -> void:
	if not pose_pinned:
		return
	var player := _get_player_node()
	if player != null:
		player.global_transform = pinned_transform
		if "velocity" in player:
			player.velocity = Vector3.ZERO
	if Global.player_camera_node != null:
		var t := Global.player_camera_node.global_transform
		t.basis = pinned_camera_basis
		Global.player_camera_node.global_transform = t


func _get_player_node() -> Node3D:
	var explorer := get_tree().current_scene
	if explorer == null:
		return null
	if "player" in explorer:
		return explorer.player
	return null


func _apply_toggles() -> void:
	if Global.scene_runner == null:
		push_error("GP Benchmark: Global.scene_runner is null when applying toggles")
		return
	Global.scene_runner.bench_disable_tweens = _toggle("disable_tweens")
	Global.scene_runner.bench_disable_transforms = _toggle("disable_transforms")


func _toggle(key: String) -> bool:
	return bool(config.get("toggles", {}).get(key, false))


func _load_config() -> Dictionary:
	var f := FileAccess.open(CONFIG_PATH, FileAccess.READ)
	if f == null:
		return {}
	var text := f.get_as_text()
	f.close()
	var parsed: Variant = JSON.parse_string(text)
	if typeof(parsed) != TYPE_DICTIONARY:
		return {}
	return parsed


## Pull tag and toggle overrides from the deeplink params so the matrix can be
## driven without re-exporting the APK/IPA. Recognized params:
##   bench-tag=<string>
##   bench-disable-tweens=true|false
##   bench-disable-transforms=true|false
##   bench-warmup=<seconds>
##   bench-sample=<seconds>
##   rs-gltf-direct=true|false  -- GLTF→RenderingServer migration toggle
func _apply_deeplink_overrides() -> void:
	if Global.deep_link_obj == null:
		return
	var params = Global.deep_link_obj.params
	var tag: String = params.get("bench-tag", "")
	if not tag.is_empty():
		config["tag"] = tag
		var ext: String = (config.get("output_path", "result.json") as String).get_extension()
		config["output_path"] = "user://output/gp-benchmark/%s.%s" % [tag, ext]

	var toggles: Dictionary = config.get("toggles", {}).duplicate()
	for k in [
		["bench-disable-tweens", "disable_tweens"],
		["bench-disable-transforms", "disable_transforms"]
	]:
		var v: String = params.get(k[0], "")
		if not v.is_empty():
			toggles[k[1]] = v.to_lower() in ["true", "1", "yes"]
	config["toggles"] = toggles

	var warmup: String = params.get("bench-warmup", "")
	if not warmup.is_empty() and warmup.is_valid_int():
		config["warmup_seconds"] = warmup.to_int()
	var sample: String = params.get("bench-sample", "")
	if not sample.is_empty() and sample.is_valid_int():
		config["sample_seconds"] = sample.to_int()

	# RenderingServer migration flags. Writes back to DclCli so the Rust side
	# (mesh_renderer, gltf_container) and any GDScript that reads Global.cli
	# observe the same value. See plan in
	# ~/.claude/plans/https-github-com-decentraland-godot-expl-precious-nest.md
	var rs_gltf_direct: String = params.get("rs-gltf-direct", "")
	if not rs_gltf_direct.is_empty():
		Global.cli.rs_gltf_direct = rs_gltf_direct.to_lower() in ["true", "1", "yes"]


func _set_phase(p: String) -> void:
	phase = p
	phase_started_at_ms = Time.get_ticks_msec()
	_log("phase -> %s" % p)
	# Markers consumed by scripts/bench/profile_android.sh / profile_ios.sh to
	# trigger simpleperf / xctrace recording exactly during the sampling window.
	if p == "sampling":
		_log(
			(
				"PROFILE_WINDOW_BEGIN duration_s=%d pid=%d"
				% [int(config.get("sample_seconds", 30)), OS.get_process_id()]
			)
		)
	elif p == "done":
		_log("PROFILE_WINDOW_END")


func _phase_elapsed_ms() -> int:
	return Time.get_ticks_msec() - phase_started_at_ms


func _log(msg: String) -> void:
	print("[GP Benchmark] %s" % msg)
