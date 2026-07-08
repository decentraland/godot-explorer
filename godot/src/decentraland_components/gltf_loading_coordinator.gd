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
##  2. LOAD+REALIZE stage — a single main-thread pump, paced ONE source-group
##     per frame. Per source it does the one main-thread `ResourceLoader.load`
##     (GPU texture upload — must be on the main thread to avoid the Mali
##     RenderingServer deadlock), then instantiates every waiter (cheap CPU-only
##     clone of the shared PackedScene) and add_child's them in that one frame.
##     Identical meshes compile their render pipeline once; distinct sources are
##     spread across frames so at most one new-pipeline stall lands per frame.
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
	var packed_scene: PackedScene = null
	var waiters: Array = []


# hash -> LoadGroup
var _groups := {}
# hashes of PENDING groups waiting for a download slot
var _download_queue: Array = []
# distinct sources currently occupying a download slot
var _downloading_count := 0
# LoadGroups whose bytes are on disk, waiting for the main-thread load+realize
# pump (one processed per frame)
var _load_queue: Array = []
var _load_pump_running := false

#region Public API — called by gltf_container.gd


## Register a container as a waiter on its content hash. The first requester of
## a hash creates the group and enqueues it for download; later requesters just
## attach and are realized when the shared load completes (or immediately queued
## for realize if the bytes are already fetched).
func request(
	container, hash: String, src: String, scene_id: int, optimized: bool, content_mapping
) -> void:
	var group: LoadGroup = _groups.get(hash)
	if group == null:
		group = LoadGroup.new()
		group.hash = hash
		group.src = src
		group.scene_id = scene_id
		group.content_mapping = content_mapping
		group.optimized = optimized
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
			# Bytes already on disk — this late waiter "downloaded" instantly.
			_mark_download(container, hash, optimized)
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


func _pump_downloads() -> void:
	while _downloading_count < MAX_CONCURRENT_DOWNLOADS and not _download_queue.is_empty():
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
	_mark_all_downloads_begin(group)

	var scene_path := ""
	if group.optimized:
		var promise = Global.content_provider.fetch_optimized_asset_with_dependencies(group.hash)
		var result = await PromiseUtils.async_awaiter(promise)
		if result is PromiseError:
			_fail_download(group, "failed to download optimized asset dependencies")
			return
		scene_path = "res://glbs/" + group.hash + ".scn"
		if not ResourceLoader.exists(scene_path):
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

	_mark_all_downloads_done(group)
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
	while not _load_queue.is_empty():
		var group: LoadGroup = _load_queue.pop_front()
		await _load_and_realize_group(group)
	_load_pump_running = false


## For ONE source in ONE frame: do the single main-thread ResourceLoader.load
## (first pass only), then instantiate + add every not-yet-realized waiter.
## Mark them FINISHED after the render frame — measuring the source's whole
## first-frame main-thread + GPU stall once.
# gdlint:ignore = async-function-name
func _load_and_realize_group(group: LoadGroup) -> void:
	if not is_instance_valid(group):
		return

	var main_t0 := Time.get_ticks_usec()

	if group.packed_scene == null:
		# Synchronous main-thread load — ONCE per source. The optimized .scn
		# embeds ETC2 texture atlases + mesh textures that upload to the GPU
		# during load; doing that on the WorkerThreadPool raced the render
		# thread over the RenderingServer command lock and DEADLOCKED on Mali.
		var resource := ResourceLoader.load(group.scene_path)
		if resource == null or not resource is PackedScene:
			_notify_group_error(group, "loaded resource is null")
			return
		group.packed_scene = resource

	group.state = STATE_REALIZING
	var batch: Array = []
	for waiter in group.waiters:
		if is_instance_valid(waiter) and waiter._needs_realize():
			batch.append(waiter)
	if batch.is_empty():
		_retire_group(group)
		return

	for waiter in batch:
		waiter._instantiate_and_add(group.packed_scene)

	await get_tree().process_frame

	# Wall time spanning load → all add_childs → next frame: the source's whole
	# main-thread + first-render pipeline stall, shared across its instances.
	var gpu_ms := (Time.get_ticks_usec() - main_t0) / 1000.0
	for waiter in batch:
		if is_instance_valid(waiter):
			waiter._complete_shared_load(gpu_ms)

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
	for waiter in group.waiters:
		if is_instance_valid(waiter):
			waiter._on_shared_load_error(reason)
	# Drop the group so a later request for this hash can retry (a transient
	# failure must not permanently blacklist the asset).
	_groups.erase(group.hash)


#endregion

#region Instrumentation helpers


func _mark_all_downloads_begin(group: LoadGroup) -> void:
	for waiter in group.waiters:
		if is_instance_valid(waiter):
			_mark_download_begin(waiter, group.hash)


func _mark_all_downloads_done(group: LoadGroup) -> void:
	for waiter in group.waiters:
		if is_instance_valid(waiter):
			_mark_download_done(waiter, group.hash, group.optimized)


func _mark_download(container, hash: String, optimized: bool) -> void:
	_mark_download_begin(container, hash)
	_mark_download_done(container, hash, optimized)


func _mark_download_begin(container, hash: String) -> void:
	LoadingProfiler.mark(
		"asset.gltf_download_begin",
		{"scene_id": container.dcl_scene_id, "entity": container.dcl_entity_id, "hash": hash}
	)


func _mark_download_done(container, hash: String, optimized: bool) -> void:
	(
		LoadingProfiler
		. mark(
			"asset.gltf_downloaded",
			{
				"scene_id": container.dcl_scene_id,
				"entity": container.dcl_entity_id,
				"hash": hash,
				"opt": optimized,
			}
		)
	)


func _prune_dead_waiters(group: LoadGroup) -> void:
	var alive: Array = []
	for waiter in group.waiters:
		if is_instance_valid(waiter):
			alive.append(waiter)
	group.waiters = alive

#endregion
