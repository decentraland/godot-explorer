extends Node
## LoadingProfiler — centralized wall-clock instrumentation for the whole loading
## pipeline: realm resolution (/about) → scene discovery → spawn → per-scene tick /
## GLTF render → loading-screen dismissal, plus navigation events (return to
## discover, teleport, world join).
##
## Every event is one greppable line with a stable prefix so it can be pulled from
## `adb logcat` and parsed by scripts/loadprof_report.py:
##
##   [LOADPROF] v=1 ev=<ev> ep=<id|-> t=<abs_ms> f=<frame> pf=<phys_frame> dt=<ms> gap=<ms> k=v ...
##   [LOADPROF-SUM] v=1 ep=<id> when=<..> realm=<..> total=<ms> frames=<n> pframes=<n> <label=ms ...>
##
## f  = Engine.get_process_frames()  (render frame — the visible frame counter)
## pf = Engine.get_physics_frames()  (physics tick — where the Rust scene runner ticks)
##
## Two independent timelines are tracked:
##   * episode  — a loading-screen-bounded journey (begin_episode → end_episode)
##   * scene    — the current parcel scene reaching render (tick>=10 & GLTFs done),
##                tracked even when no loading screen is shown (walking to new places)
##
## INSTRUMENTATION ONLY — never changes engine/app behaviour. Turn off with ENABLED
## (or remove the autoload + the `LoadingProfiler.*` call sites).

## Master on/off switch for all profiling output.
const ENABLED := true
## Per-event [LOADPROF] print lines. OFF in prod (the Segment funnel event is the signal);
## flip on locally to trace an episode step-by-step. The once-per-episode [LOADPROF-SUM]
## summary and the Segment emission stay active whenever ENABLED.
const VERBOSE_LOG := false
## Output line-grammar version (bump if the fields below change).
const FMT_VERSION := 1

# --- episode state (loading-screen-bounded journey) ---
var _ep_id := 0
var _ep_active := false
var _ep_start_ms := 0
var _ep_start_frame := 0
var _ep_start_pframe := 0
var _ep_when := "-"
var _ep_realm := "-"
var _last_mark_ms := 0
var _ep_marks := {}  # event name -> dt_ms of first occurrence (for the summary)
# --- rich per-episode aggregates (for the Segment funnel event) ---
var _ep_phase_ms := {}  # LoadingPhase name -> first ms since episode begin
var _ep_band_ms := 0  # accumulated ms spent in the 25-30% plateau band (F-7 signal)
var _ep_band_enter_ms := -1  # ms when we entered the band (-1 = not currently in band)
var _ep_res_loaded0 := 0  # count_loaded_resources at episode begin (delta baseline)
var _ep_res_loading0 := 0  # count_loading_resources at episode begin (delta baseline)
var _ep_gltf_failed0 := 0  # coordinator failed-groups at episode begin (delta baseline)
var _ep_peak_pending := 0  # peak pending assets (loading - loaded) during the episode

# --- loading_progress throttling ---
var _last_progress_bucket := -1
var _last_ready := -1

# --- per-scene render tracking (independent of the loading screen) ---
var _tracked_scene_id := -2  # -2 = uninitialized (distinct from SceneManager's -1)
var _tracked_scene_start_ms := 0
var _seen_first_tick := false
var _seen_tick4 := false
var _seen_tick10 := false
var _seen_gltf_done := false
var _seen_rendered := false
var _seen_gltf_activity := false  # progress dipped below 1.0 (GLTFs really started loading)
var _gltf_last_busy_ms := 0  # last time the scene had GLTFs pending
var _seen_gltf_quiet := false  # progress==1.0 sustained 3s (scene stopped streaming GLTFs)

var _connected := false

# --- aggregate resource-download tracking (all assets: gltf, textures, audio) ---
var _dl_active := false
var _dl_last_log_ms := 0


func _ready() -> void:
	if not ENABLED:
		set_process(false)
		return
	# Connection happens lazily in _process so Global.realm / Global.scene_runner are
	# guaranteed to exist (autoload order + first-frame retry).
	_log("profiler.ready", {"fmt": FMT_VERSION})


func _process(_delta: float) -> void:
	if not ENABLED:
		return
	if not _connected:
		_try_connect()
		return
	_poll_current_scene()
	_poll_downloads()


# ---------------------------------------------------------------------------
# Public API used by the instrumented call sites
# ---------------------------------------------------------------------------


## Log a one-off event with optional key=value metadata.
func mark(event: String, extra: Dictionary = {}) -> void:
	_log(event, extra)


## Open a new loading-screen episode. Auto-closes a previous one if still open.
func begin_episode(when: String, realm: String) -> void:
	if not ENABLED:
		return
	if _ep_active:
		end_episode("superseded")
	_ep_id += 1
	_ep_active = true
	_ep_start_ms = now_ms()
	_ep_start_frame = Engine.get_process_frames()
	_ep_start_pframe = Engine.get_physics_frames()
	_last_mark_ms = _ep_start_ms
	_ep_when = _san(when) if not when.is_empty() else "-"
	_ep_realm = _san(realm) if not realm.is_empty() else "-"
	_ep_marks = {}
	_last_progress_bucket = -1
	_last_ready = -1
	# reset + snapshot the rich aggregates for per-episode deltas
	_ep_phase_ms = {}
	_ep_band_ms = 0
	_ep_band_enter_ms = -1
	_ep_peak_pending = 0
	var counts := _res_counts()
	_ep_res_loaded0 = counts[0]
	_ep_res_loading0 = counts[1]
	_ep_gltf_failed0 = _gltf_failed_total()
	_log("episode.begin", {"when": _ep_when, "realm": _ep_realm})
	_emit_segment_started()


## Close the current episode and emit its timing summary.
func end_episode(reason: String) -> void:
	if not ENABLED or not _ep_active:
		return
	# close an open plateau-band interval so its time is counted
	if _ep_band_enter_ms >= 0:
		_ep_band_ms += now_ms() - _ep_band_enter_ms
		_ep_band_enter_ms = -1
	_log("episode.end", {"reason": reason})
	_emit_summary(reason)
	_emit_segment_completed(reason)
	_ep_active = false


func now_ms() -> int:
	return Time.get_ticks_msec()


# ---------------------------------------------------------------------------
# Signal wiring (Rust SceneManager + GDScript Realm)
# ---------------------------------------------------------------------------


func _try_connect() -> void:
	if _connected:
		return
	var sr = Global.scene_runner
	var realm = Global.realm
	if sr == null or realm == null:
		return  # retried next frame
	sr.loading_started.connect(_on_loading_started)
	sr.loading_phase_changed.connect(_on_phase_changed)
	sr.loading_progress.connect(_on_loading_progress)
	sr.loading_complete.connect(_on_loading_complete)
	sr.loading_timeout.connect(_on_loading_timeout)
	sr.loading_cancelled.connect(_on_loading_cancelled)
	sr.scene_spawned.connect(_on_scene_spawned)
	realm.realm_changing.connect(_on_realm_changing)
	realm.realm_changed.connect(_on_realm_changed)
	realm.realm_change_failed.connect(_on_realm_change_failed)
	_connected = true
	_log("profiler.connected")


func _on_realm_changing() -> void:
	_log("realm.changing")


func _on_realm_changed() -> void:
	_log("realm.changed", {"realm": Global.realm.get_realm_string()})


func _on_realm_change_failed(new_realm: String, reason: String) -> void:
	_log("realm.change_failed", {"realm": new_realm, "reason": reason})
	if is_instance_valid(Global.metrics):
		Global.metrics.track_realm_change_failed(
			JSON.stringify({"realm_bucket": _realm_bucket(new_realm), "reason": _san(reason)})
		)


func _on_loading_started(session_id: int, expected_count: int) -> void:
	# Cover loads that begin without a UI entry point (e.g. walking into a parcel).
	if not _ep_active:
		begin_episode("auto", Global.realm.get_realm_string())
	_log("loading.started", {"session": session_id, "expected": expected_count})


func _on_phase_changed(phase: String) -> void:
	if _ep_active and not _ep_phase_ms.has(phase):
		_ep_phase_ms[phase] = now_ms() - _ep_start_ms
	_log("loading.phase", {"phase": phase})


func _on_loading_progress(percent: float, ready: int, total: int) -> void:
	# Accumulate wall-clock time the bar sits in the 25-30% plateau band (F-7 signal).
	if _ep_active:
		var in_band := percent >= 25.0 and percent < 30.0
		if in_band and _ep_band_enter_ms < 0:
			_ep_band_enter_ms = now_ms()
		elif not in_band and _ep_band_enter_ms >= 0:
			_ep_band_ms += now_ms() - _ep_band_enter_ms
			_ep_band_enter_ms = -1
	# First scene counted ready — a key milestone for the summary.
	if ready > 0 and _ep_active and not _ep_marks.has("loading.progress.ready"):
		_log("loading.progress.ready", {"ready": ready, "total": total})
	# Throttle the noisy progress stream to 10% buckets / ready-count changes.
	var bucket := int(percent) / 10
	if bucket != _last_progress_bucket or ready != _last_ready:
		_last_progress_bucket = bucket
		_last_ready = ready
		_log("loading.progress", {"pct": int(percent), "ready": ready, "total": total})


func _on_loading_complete(session_id: int) -> void:
	_log("loading.complete", {"session": session_id})


func _on_loading_timeout(session_id: int) -> void:
	_log("loading.timeout", {"session": session_id})


func _on_loading_cancelled(session_id: int) -> void:
	_log("loading.cancelled", {"session": session_id})


func _on_scene_spawned(scene_id: int, entity_id: String) -> void:
	_log("scene.spawned", {"scene_id": scene_id, "entity": entity_id})


# ---------------------------------------------------------------------------
# Per-scene render polling
# ---------------------------------------------------------------------------


func _poll_current_scene() -> void:
	var sr = Global.scene_runner
	if sr == null:
		return
	var sid: int = sr.get_current_parcel_scene_id()

	if sid != _tracked_scene_id:
		_tracked_scene_id = sid
		_tracked_scene_start_ms = now_ms()
		_seen_first_tick = false
		_seen_tick4 = false
		_seen_tick10 = false
		_seen_gltf_done = false
		_seen_rendered = false
		_seen_gltf_activity = false
		_gltf_last_busy_ms = _tracked_scene_start_ms
		_seen_gltf_quiet = false
		if sid >= 0:
			_log("scene.current", {"scene_id": sid, "title": _safe_title(sid)})
		return

	if sid < 0 or _seen_gltf_quiet:
		return

	var node := _find_scene_node(sid)
	if node == null:
		return
	var tick: int = node.get_last_tick_number()
	var gltf: float = node.get_gltf_loading_progress()
	var ms := now_ms() - _tracked_scene_start_ms

	if not _seen_first_tick and tick >= 0:
		_seen_first_tick = true
		_log("scene.tick_first", {"scene_id": sid, "ms": ms})
	if not _seen_tick4 and tick >= 4:
		_seen_tick4 = true
		_log("scene.tick4_sdk_ready", {"scene_id": sid, "ms": ms})
	if not _seen_tick10 and tick >= 10:
		_seen_tick10 = true
		_log("scene.tick10_ready", {"scene_id": sid, "ms": ms})
	# progress==1.0 with max==0 means "no GLTFs yet", NOT done — only trust
	# completion once we've actually seen GLTFs load (progress dipped below 1.0).
	if gltf < 1.0:
		_seen_gltf_activity = true
		_gltf_last_busy_ms = now_ms()
	if not _seen_gltf_done and _seen_gltf_activity and gltf >= 1.0:
		_seen_gltf_done = true
		_log("scene.gltf_done", {"scene_id": sid, "ms": ms})
	if not _seen_rendered and _seen_tick10 and _seen_gltf_done:
		_seen_rendered = true
		_log("scene.rendered", {"scene_id": sid, "ms": ms})
	# "Quiet" = GLTFs stopped streaming for 3s — the honest "scene settled" signal
	# (scene.rendered is only the FIRST transient gap between GLTF waves).
	if (
		not _seen_gltf_quiet
		and _seen_gltf_activity
		and gltf >= 1.0
		and now_ms() - _gltf_last_busy_ms >= 3000
	):
		_seen_gltf_quiet = true
		_log("scene.gltf_quiet", {"scene_id": sid, "ms": ms})


func _find_scene_node(scene_id: int) -> DclSceneNode:
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode and child.get_scene_id() == scene_id:
			return child
	return null


func _safe_title(scene_id: int) -> String:
	var title: String = Global.scene_runner.get_scene_title(scene_id)
	return title if not title.is_empty() else "?"


# ---------------------------------------------------------------------------
# Aggregate resource-download polling (covers every asset kind: GLTF, texture,
# audio, video — everything routed through ContentProvider). Per-GLTF detail is
# emitted separately from gltf_container.gd (asset.gltf_*).
# ---------------------------------------------------------------------------


func _poll_downloads() -> void:
	var cp = Global.content_provider
	if cp == null:
		return
	var loading: int = cp.count_loading_resources()
	var loaded: int = cp.count_loaded_resources()
	var pending := loading - loaded
	if _ep_active and pending > _ep_peak_pending:
		_ep_peak_pending = pending
	if pending > 0 and not _dl_active:
		_dl_active = true
		_dl_last_log_ms = now_ms()
		_log(
			"asset.download_wave_start", {"loading": loading, "loaded": loaded, "pending": pending}
		)
	elif pending > 0 and _dl_active:
		if now_ms() - _dl_last_log_ms >= 500:
			_dl_last_log_ms = now_ms()
			_log(
				"asset.download_progress",
				{
					"loaded": loaded,
					"loading": loading,
					"pending": pending,
					"mbps": cp.get_download_speed_mbs(),
				}
			)
	elif pending <= 0 and _dl_active:
		_dl_active = false
		_log("asset.download_wave_end", {"total_loaded": loaded})


# ---------------------------------------------------------------------------
# Formatting
# ---------------------------------------------------------------------------


func _log(event: String, extra: Dictionary = {}) -> void:
	if not ENABLED:
		return
	var t := now_ms()
	var dt_s := "-"
	var gap_s := "-"
	var ep_s := "-"
	if _ep_active:
		dt_s = str(t - _ep_start_ms)
		gap_s = str(t - _last_mark_ms)
		ep_s = str(_ep_id)
		_last_mark_ms = t
		if not _ep_marks.has(event):
			_ep_marks[event] = t - _ep_start_ms
	var line := (
		"[LOADPROF] v=%d ev=%s ep=%s t=%d f=%d pf=%d dt=%s gap=%s"
		% [
			FMT_VERSION,
			event,
			ep_s,
			t,
			Engine.get_process_frames(),
			Engine.get_physics_frames(),
			dt_s,
			gap_s,
		]
	)
	for k in extra:
		line += " %s=%s" % [str(k), _san(str(extra[k]))]
	if VERBOSE_LOG:
		print(line)


func _emit_summary(reason: String) -> void:
	var total := now_ms() - _ep_start_ms
	var frames := Engine.get_process_frames() - _ep_start_frame
	var pframes := Engine.get_physics_frames() - _ep_start_pframe
	var line := (
		"[LOADPROF-SUM] v=%d ep=%d when=%s realm=%s reason=%s total=%d frames=%d pframes=%d"
		% [FMT_VERSION, _ep_id, _ep_when, _ep_realm, _san(reason), total, frames, pframes]
	)
	# milestone label -> event key (ms since episode begin; -1 = never happened)
	var milestones := [
		["realm_changing", "realm.changing"],
		["about_start", "realm.about_fetch_start"],
		["about_end", "realm.about_fetch_end"],
		["realm_changed", "realm.changed"],
		["discovery", "discovery.desired_changed"],
		["loading_started", "loading.started"],
		["first_spawn", "scene.spawned"],
		["first_ready", "loading.progress.ready"],
		["complete", "loading.complete"],
		["scene_rendered", "scene.rendered"],
		["place_data", "screen.place_data"],
		["hidden", "screen.hidden"],
	]
	for m in milestones:
		line += " %s=%s" % [m[0], str(_ep_marks.get(m[1], -1))]
	print(line)


## Make a value safe for the `k=v` space-separated grammar.
func _san(s: String) -> String:
	return s.replace(" ", "_").replace("=", ":").strip_edges()


# ---------------------------------------------------------------------------
# Segment analytics — the loading funnel as field-measurable events (#2477 / F-16).
# GDScript owns the schema; the Rust Metrics.track_scene_loading_* #[func]s just forward the
# JSON. All values are bucketed/rounded — no PII, no parcel coords, no URLs.
# ---------------------------------------------------------------------------


func _res_counts() -> Array:
	var cp = Global.content_provider
	if cp == null:
		return [0, 0]
	return [int(cp.count_loaded_resources()), int(cp.count_loading_resources())]


func _gltf_failed_total() -> int:
	if (
		typeof(GltfLoadingCoordinator) == TYPE_OBJECT
		and GltfLoadingCoordinator.has_method("get_failed_groups_total")
	):
		return GltfLoadingCoordinator.get_failed_groups_total()
	return 0


## Coarse realm class (no full URL / PII): world | genesis | other | unknown.
func _realm_bucket(realm: String) -> String:
	if realm.is_empty() or realm == "-":
		return "unknown"
	if Realm.is_dcl_ens(realm):
		return "world"
	var low := realm.to_lower()
	if low.contains("genesis") or low.contains("peer.decentraland") or low == "main":
		return "genesis"
	return "other"


## How the loading screen ended. Key signal: dismissed but never reached ready == the
## wall-clock modal fired == infinite loading.
func _classify_dismissal(reason: String, reached_ready: bool) -> String:
	var r := reason.to_lower()
	if r.contains("supersed"):
		return "superseded"
	if r.contains("cancel"):
		return "cancelled"
	if _ep_marks.has("realm.change_failed") or r.contains("error") or r.contains("fail"):
		return "error"
	if r.contains("timeout"):
		return "wall_clock_timeout"
	if reached_ready:
		return "completion"
	return "wall_clock_timeout"


## Entry ms of each loading phase since episode begin (-1 = never reached). Analysts diff
## consecutive phases for per-phase duration.
func _phase_breakdown() -> Dictionary:
	var out := {}
	for p in ["metadata", "spawning", "assets", "ready", "floating_islands", "done"]:
		out[p] = _ep_phase_ms.get(p, -1)
	return out


func _emit_segment_started() -> void:
	if not is_instance_valid(Global.metrics):
		return
	var cp = Global.content_provider
	var props := {
		"schema": FMT_VERSION,
		"realm_bucket": _realm_bucket(_ep_realm),
		"when": _ep_when,
		"is_world": Realm.is_dcl_ens(_ep_realm),
		"cold_cache": _ep_res_loaded0 == 0,
		"network_estimate_mbs": cp.get_download_speed_mbs() if cp != null else 0.0,
		"cpu_count": OS.get_processor_count(),
		"platform": OS.get_name(),
	}
	Global.metrics.track_scene_loading_started(JSON.stringify(props))


func _emit_segment_completed(reason: String) -> void:
	if not is_instance_valid(Global.metrics):
		return
	var total := now_ms() - _ep_start_ms
	var first_ready: int = _ep_marks.get("loading.progress.ready", -1)
	var reached_ready := first_ready >= 0
	var counts := _res_counts()
	var loaded_now: int = int(counts[0])
	var loading_now: int = int(counts[1])
	var props := {
		"schema": FMT_VERSION,
		"realm_bucket": _realm_bucket(_ep_realm),
		"when": _ep_when,
		"duration_ms": total,
		"reached_ready": reached_ready,
		"dismissed_by": _classify_dismissal(reason, reached_ready),
		"first_spawn_ms": _ep_marks.get("scene.spawned", -1),
		"first_ready_ms": first_ready,
		"complete_ms": _ep_marks.get("loading.complete", -1),
		"scene_rendered_ms": _ep_marks.get("scene.rendered", -1),
		"about_end_ms": _ep_marks.get("realm.about_fetch_end", -1),
		"discovery_ms": _ep_marks.get("discovery.desired_changed", -1),
		"time_in_25_30_band_ms": _ep_band_ms,
		"phase_breakdown": _phase_breakdown(),
		"assets_expected": maxi(0, loading_now - _ep_res_loading0),
		"assets_loaded": maxi(0, loaded_now - _ep_res_loaded0),
		"assets_unfinished": maxi(0, loading_now - loaded_now),
		"assets_errored": maxi(0, _gltf_failed_total() - _ep_gltf_failed0),
		"peak_pending_assets": _ep_peak_pending,
		"frames": Engine.get_process_frames() - _ep_start_frame,
	}
	Global.metrics.track_scene_loading_completed(JSON.stringify(props))
	# Aggregated per-episode asset-failure diagnostic (coarse reason for v1).
	var errored: int = props["assets_errored"]
	if errored > 0:
		(
			Global
			. metrics
			. track_scene_loading_asset_failure(
				(
					JSON
					. stringify(
						{
							"realm_bucket": props["realm_bucket"],
							"reason": "gltf_group_failed",
							"count": errored,
						}
					)
				)
			)
		)
	Global.metrics.flush.call_deferred()
