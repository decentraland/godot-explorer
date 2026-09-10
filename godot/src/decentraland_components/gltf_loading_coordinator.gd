extends Node

## GltfLoadingCoordinator — shared, source-deduplicated GLTF loading with a
## two-stage pipeline that decouples network concurrency from main-thread work.
##
## Replaces the per-entity throttle that used to live in gltf_container.gd, where
## every DCL entity was an independent load: N entities referencing the same
## asset each took a slot, each queued separately (~42s median queue wait per
## instance on Genesis Plaza), and each downloaded + loaded the same bytes.
##
## Pipeline (per distinct content hash = one LoadGroup):
##
##  1. DOWNLOAD stage — throttled by MAX_CONCURRENT_DOWNLOADS (network-bound,
##     off-thread). One fetch per hash, shared across every waiter. This is
##     high because the ContentProvider's HttpQueueRequester already caps real
##     HTTP parallelism (12); we just want the fetch stage to not be the wall.
##
##  2. LOAD+REALIZE stage — a single main-thread pump with a per-frame time
##     budget (LOAD_PUMP_BUDGET_USEC; at least one source per frame). Per
##     source it does the one main-thread `ResourceLoader.load` (GPU texture
##     upload — must be on the main thread to avoid the Mali RenderingServer
##     deadlock), then instantiates every waiter (cheap CPU-only clone of the
##     shared PackedScene) and add_child's them in that same frame. Identical
##     meshes compile their render pipeline once; expensive sources still get
##     a frame each, cheap ones are batched so the queue drains fast.
##
## Splitting the two stages is the point: downloads run wide while the heavy,
## unavoidably-serial main-thread load/upload is paced. A load slot is no longer
## held for the whole network round-trip.
##
## The per-entity loading-state contract is unchanged: each container flips its
## own `dcl_gltf_loading_state` to FINISHED/error, which the Rust
## `sync_gltf_loading_state` poll consumes to clear `scene.gltf_loading` and
## dismiss the loading screen.

## Network fetch concurrency. High on purpose: the real HTTP cap lives in the
## ContentProvider's HttpQueueRequester (12). This just bounds how many hashes
## sit in the fetch stage (and thus in-flight download memory).
const MAX_CONCURRENT_DOWNLOADS := 24

## Main-thread budget per frame for the load+realize pump. Sources are pulled
## from the queue until the budget is spent (at least one per frame), so a
## queue of cheap sources drains in a few frames instead of one-per-frame at
## whatever the loading frame rate happens to be (~6-10 fps on Genesis Plaza,
## i.e. ~4-6 s for a 40-deep queue).
const LOAD_PUMP_BUDGET_USEC := 12_000

const STATE_PENDING := 0  # created, waiting for a download slot
const STATE_DOWNLOADING := 1  # holding a download slot, fetching
const STATE_FETCHED := 2  # bytes on disk, queued for main-thread load+realize
const STATE_REALIZING := 3  # being loaded/instantiated/added this frame
const STATE_DONE := 4  # every current waiter realized
const STATE_ERROR := 5  # fetch or load failed; waiters notified


class LoadGroup:
	extends RefCounted
	var hash := ""
	var src := ""
	var scene_id := -1
	var content_mapping = null
	var optimized := false
	var state := 0
	var scene_path := ""
	# Bypass ResourceLoader's cache for this load — see `request(force_fresh)`.
	var force_fresh := false
	var packed_scene: PackedScene = null
	var waiters: Array = []


# hash -> LoadGroup
var _groups := {}
# hashes of PENDING groups waiting for a download slot
var _download_queue: Array = []
# distinct sources currently occupying a download slot
var _downloading_count := 0
# LoadGroups whose bytes are on disk, waiting for the main-thread load+realize
# pump (time-budgeted per frame)
var _load_queue: Array = []
var _load_pump_running := false
# Groups whose .scn is being parsed by ResourceLoader's thread (experiment).
var _threaded_loading: Array = []
var _threaded_slots_cache := -1
var _max_groups_cache := -1

# Cumulative pipeline stats (debug / benchmarking; see get_stats()).
var _stats_groups := 0
var _stats_download_usec := 0
var _stats_download_max_usec := 0
var _stats_loads := 0
var _stats_load_usec := 0
var _stats_load_max_usec := 0
var _stats_instances := 0
var _stats_instantiate_usec := 0
var _stats_pump_frames := 0
var _stats_pump_usec := 0
var _stats_errors := 0


## Cumulative counters of the pipeline since boot (all times in ms).
func get_stats() -> Dictionary:
	return {
		"groups": _stats_groups,
		"download_ms": _stats_download_usec / 1000.0,
		"download_max_ms": _stats_download_max_usec / 1000.0,
		"loads": _stats_loads,
		"load_ms": _stats_load_usec / 1000.0,
		"load_max_ms": _stats_load_max_usec / 1000.0,
		"instances": _stats_instances,
		"instantiate_ms": _stats_instantiate_usec / 1000.0,
		"pump_frames": _stats_pump_frames,
		"pump_ms": _stats_pump_usec / 1000.0,
		"errors": _stats_errors,
		"in_flight_groups": _groups.size(),
	}


#region Public API — called by gltf_container.gd


## Register a container as a waiter on its content hash. The first requester of
## a hash creates the group and enqueues it for download; later requesters just
## attach and are realized when the shared load completes (or immediately queued
## for realize if the bytes are already fetched).
##
## `force_fresh` re-reads the .scn from disk instead of reusing ResourceLoader's
## cached copy. Only preview hot-reload needs it: the preview server hashes file
## *paths*, so an edited model keeps its hash and therefore its .scn path, and a
## cached load would hand back the previous version of the file.
func request(
	container,
	hash: String,
	src: String,
	scene_id: int,
	optimized: bool,
	content_mapping,
	force_fresh: bool = false
) -> void:
	var group: LoadGroup = _groups.get(hash)
	if group != null and force_fresh:
		group.force_fresh = true
		group.packed_scene = null

	if group == null:
		group = LoadGroup.new()
		group.hash = hash
		group.src = src
		group.scene_id = scene_id
		group.content_mapping = content_mapping
		group.optimized = optimized
		group.force_fresh = force_fresh
		group.state = STATE_PENDING
		group.waiters.append(container)
		_groups[hash] = group
		_download_queue.append(hash)
		_pump_downloads()
		return

	if group.waiters.has(container):
		return
	group.waiters.append(container)

	match group.state:
		STATE_PENDING, STATE_DOWNLOADING:
			# Realized in the batch when the shared download + load completes.
			pass
		STATE_FETCHED, STATE_REALIZING:
			# Bytes already on disk — this late waiter attaches to the shared load.
			if not _load_queue.has(group):
				_load_queue.append(group)
			_ensure_load_pump()
		STATE_ERROR:
			container._on_shared_load_error("shared load failed")


## Detach a container (e.g. it left the tree or is reloading a new src).
func unregister(container, hash: String) -> void:
	var group: LoadGroup = _groups.get(hash)
	if group == null:
		return
	group.waiters.erase(container)


#endregion

#region Download stage (network, wide)


func _max_concurrent_downloads() -> int:
	if _max_groups_cache >= 0:
		return _max_groups_cache
	_max_groups_cache = MAX_CONCURRENT_DOWNLOADS
	# `max-groups=<n>` deeplink: benchmark knob for the fetch-stage width.
	if Global.deep_link_obj != null:
		var v := str(Global.deep_link_obj.params.get("max-groups", ""))
		if v.is_valid_int() and v.to_int() > 0:
			_max_groups_cache = v.to_int()
	return _max_groups_cache


func _pump_downloads() -> void:
	while _downloading_count < _max_concurrent_downloads() and not _download_queue.is_empty():
		var hash := _pop_next_download()
		if hash.is_empty():
			break
		var group: LoadGroup = _groups.get(hash)
		if group == null or group.state != STATE_PENDING:
			continue
		_prune_dead_waiters(group)
		if group.waiters.is_empty():
			_groups.erase(hash)
			continue
		group.state = STATE_DOWNLOADING
		_downloading_count += 1
		_async_download_group(group)


## Pop the next hash to download, prioritizing groups with a current-scene
## waiter (mirrors the old per-container queue prioritization).
func _pop_next_download() -> String:
	for i in range(_download_queue.size()):
		var hash: String = _download_queue[i]
		var group: LoadGroup = _groups.get(hash)
		if group != null and _has_current_scene_waiter(group):
			_download_queue.remove_at(i)
			return hash
	while not _download_queue.is_empty():
		var hash: String = _download_queue.pop_front()
		if _groups.has(hash):
			return hash
	return ""


func _has_current_scene_waiter(group: LoadGroup) -> bool:
	for waiter in group.waiters:
		if is_instance_valid(waiter) and waiter.is_current_scene():
			return true
	return false


# gdlint:ignore = async-function-name
func _async_download_group(group: LoadGroup) -> void:
	var scene_path := ""
	var t0 := Time.get_ticks_usec()
	_stats_groups += 1
	if group.optimized:
		var promise = Global.content_provider.fetch_optimized_asset_with_dependencies(group.hash)
		var result = await PromiseUtils.async_awaiter(promise)
		if result is PromiseError:
			_fail_download(group, "failed to download optimized asset dependencies")
			return
		scene_path = Global.content_provider.get_optimized_scene_path(group.hash)
		if not FileAccess.file_exists(scene_path):
			_fail_download(group, "optimized scene not found: " + scene_path)
			return
	else:
		var promise = Global.content_provider.load_scene_gltf(group.src, group.content_mapping)
		if promise == null:
			_fail_download(group, "failed to start loading")
			return
		await PromiseUtils.async_awaiter(promise)
		if promise.is_rejected():
			var error = promise.get_data()
			var reason: String = error.get_error() if error is PromiseError else "promise rejected"
			_fail_download(group, reason)
			return
		var data = promise.get_data()
		if not data is String or (data as String).is_empty():
			_fail_download(group, "invalid scene path")
			return
		scene_path = data

	var dl_usec := Time.get_ticks_usec() - t0
	_stats_download_usec += dl_usec
	_stats_download_max_usec = maxi(_stats_download_max_usec, dl_usec)
	group.scene_path = scene_path
	group.state = STATE_FETCHED
	_release_download_slot()
	if not _load_queue.has(group):
		_load_queue.append(group)
	_ensure_load_pump()


func _fail_download(group: LoadGroup, reason: String) -> void:
	_release_download_slot()
	_notify_group_error(group, reason)


func _release_download_slot() -> void:
	_downloading_count = maxi(0, _downloading_count - 1)
	_pump_downloads()


#endregion

#region Load + realize stage (main thread, one source per frame)


func _ensure_load_pump() -> void:
	if _load_pump_running:
		return
	_load_pump_running = true
	_run_load_pump()


# gdlint:ignore = async-function-name
func _run_load_pump() -> void:
	while not _load_queue.is_empty() or not _threaded_loading.is_empty():
		var frame_t0 := Time.get_ticks_usec()
		var realized: Array = []  # [group, batch] pairs realized this frame

		# EXPERIMENT (`threaded-load=<n>` deeplink): hand the .scn parse to
		# Godot's loader thread and only instantiate on the main thread. Poll
		# statuses first, so a load that finished during the last frame is
		# realized this frame.
		if _threaded_load_slots() > 0:
			var still_loading: Array = []
			for group in _threaded_loading:
				var status := ResourceLoader.load_threaded_get_status(group.scene_path)
				if status == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
					still_loading.append(group)
					continue
				if status == ResourceLoader.THREAD_LOAD_LOADED:
					var resource := ResourceLoader.load_threaded_get(group.scene_path)
					if resource is PackedScene:
						group.packed_scene = resource
				if group.packed_scene == null:
					_notify_group_error(group, "threaded load failed (status %d)" % status)
					continue
				_stats_loads += 1
				var batch := _load_and_realize_group(group)
				if not batch.is_empty():
					realized.append([group, batch])
			_threaded_loading = still_loading
			while not _load_queue.is_empty() and _threaded_loading.size() < _threaded_load_slots():
				var group: LoadGroup = _load_queue.pop_front()
				if not is_instance_valid(group) or _threaded_loading.has(group):
					continue
				if group.packed_scene != null:
					var batch := _load_and_realize_group(group)
					if not batch.is_empty():
						realized.append([group, batch])
					continue
				var err := ResourceLoader.load_threaded_request(group.scene_path, "", false)
				if err != OK:
					_notify_group_error(group, "load_threaded_request failed (%d)" % err)
					continue
				_threaded_loading.append(group)

		while not _load_queue.is_empty() and _threaded_load_slots() == 0:
			var group: LoadGroup = _load_queue.pop_front()
			var batch := _load_and_realize_group(group)
			if not batch.is_empty():
				realized.append([group, batch])
			if Time.get_ticks_usec() - frame_t0 >= LOAD_PUMP_BUDGET_USEC:
				break
		_stats_pump_frames += 1
		_stats_pump_usec += Time.get_ticks_usec() - frame_t0

		await get_tree().process_frame

		for pair in realized:
			_complete_realized_group(pair[0], pair[1])
	_load_pump_running = false


## Number of concurrent background .scn loads (0 = synchronous main-thread
## load, the default). Read from the `threaded-load=<n>` deeplink param.
func _threaded_load_slots() -> int:
	if _threaded_slots_cache >= 0:
		return _threaded_slots_cache
	_threaded_slots_cache = 0
	if Global.deep_link_obj != null:
		var v := str(Global.deep_link_obj.params.get("threaded-load", ""))
		if v.is_valid_int():
			_threaded_slots_cache = maxi(0, v.to_int())
	return _threaded_slots_cache


## For ONE source: do the single main-thread ResourceLoader.load (first pass
## only), then instantiate + add every not-yet-realized waiter. Returns the
## batch of waiters realized (empty when nothing was added). The waiters are
## marked FINISHED by `_complete_realized_group` after the next render frame.
func _load_and_realize_group(group: LoadGroup) -> Array:
	if not is_instance_valid(group):
		return []

	if group.packed_scene == null:
		# Synchronous main-thread load — ONCE per source. The optimized .scn
		# embeds ETC2 texture atlases + mesh textures that upload to the GPU
		# during load; doing that on the WorkerThreadPool raced the render
		# thread over the RenderingServer command lock and DEADLOCKED on Mali.
		var cache_mode := (
			ResourceLoader.CACHE_MODE_REPLACE
			if group.force_fresh
			else ResourceLoader.CACHE_MODE_REUSE
		)
		var t0 := Time.get_ticks_usec()
		var resource := ResourceLoader.load(group.scene_path, "", cache_mode)
		var load_usec := Time.get_ticks_usec() - t0
		_stats_loads += 1
		_stats_load_usec += load_usec
		_stats_load_max_usec = maxi(_stats_load_max_usec, load_usec)
		if resource == null or not resource is PackedScene:
			_notify_group_error(group, "loaded resource is null")
			return []
		group.packed_scene = resource

	group.state = STATE_REALIZING
	var batch: Array = []
	for waiter in group.waiters:
		if is_instance_valid(waiter) and waiter._needs_realize():
			batch.append(waiter)
	if batch.is_empty():
		_retire_group(group)
		return []

	var t1 := Time.get_ticks_usec()
	for waiter in batch:
		waiter._instantiate_and_add(group.packed_scene)
	_stats_instances += batch.size()
	_stats_instantiate_usec += Time.get_ticks_usec() - t1
	return batch


## After the render frame following a realize pass: mark the batch FINISHED and
## retire (or re-queue) the group.
func _complete_realized_group(group: LoadGroup, batch: Array) -> void:
	for waiter in batch:
		if is_instance_valid(waiter):
			waiter._complete_shared_load()

	# A waiter may have attached during the render frame — realize it next pass.
	if _has_unrealized_waiter(group):
		group.state = STATE_FETCHED
		if not _load_queue.has(group):
			_load_queue.append(group)
		return

	_retire_group(group)


func _has_unrealized_waiter(group: LoadGroup) -> bool:
	for waiter in group.waiters:
		if is_instance_valid(waiter) and waiter._needs_realize():
			return true
	return false


## Drop the shared PackedScene and the group once every waiter is realized, so it
## is NOT pinned for the app's lifetime. The old per-container path relied on
## ResourceLoader's cache releasing the resource once nothing referenced it;
## retaining it here would leak memory on OOM-sensitive mobile. A later entity
## referencing this hash simply creates a fresh group and reloads.
func _retire_group(group: LoadGroup) -> void:
	group.state = STATE_DONE
	group.packed_scene = null
	_groups.erase(group.hash)


#endregion

#region Errors


func _notify_group_error(group: LoadGroup, reason: String) -> void:
	group.state = STATE_ERROR
	_stats_errors += 1
	# Feed the loading funnel (Rust) so the load's `assets_errored` reflects this failure.
	Global.scene_runner.loading_note_asset_failure(1)
	for waiter in group.waiters:
		if is_instance_valid(waiter):
			waiter._on_shared_load_error(reason)
	# Drop the group so a later request for this hash can retry (a transient
	# failure must not permanently blacklist the asset).
	_groups.erase(group.hash)


#endregion

#region Waiter maintenance


func _prune_dead_waiters(group: LoadGroup) -> void:
	var alive: Array = []
	for waiter in group.waiters:
		if is_instance_valid(waiter):
			alive.append(waiter)
	group.waiters = alive

#endregion
