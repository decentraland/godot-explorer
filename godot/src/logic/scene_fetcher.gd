class_name SceneFetcher
extends Node

signal parcels_processed(parcel_filled, empty)
signal report_scene_load(done: bool, is_new_loading: bool, pending: int)
signal notify_pending_loading_scenes(is_pending: bool)
signal player_parcel_changed(new_position: Vector2i)

const EMPTY_SCENE = preload("res://assets/empty-scenes/empty_parcel.tscn")
const EMPTY_PARCEL_MATERIAL = preload("res://assets/empty-scenes/empty_parcel_material.tres")
const CLIFF_MATERIAL = preload("res://assets/empty-scenes/cliff_material.tres")

# Maximum number of empty parcels before switching to simple floor mode
# Above this threshold, individual floating islands are replaced with a single floor + cliffs
const MAX_EMPTY_PARCELS_FOR_FLOATING_ISLANDS: int = 100

const ADAPTATION_LAYER_URL: String = "https://renderer-artifacts.decentraland.org/sdk6-adaption-layer/main/index.min.js"
const FIXED_LOCAL_ADAPTATION_LAYER: String = ""
const INVALID_PARCEL := Vector2i(-1000, -1000)


class SceneItem:
	extends RefCounted
	var main_js_req_id: int = -1
	var main_crdt_req_id: int = -1

	var id: String = ""
	var scene_entity_definition: DclSceneEntityDefinition = null
	var scene_number_id: int = -1
	var parcels: Array[Vector2i] = []
	var is_global: bool = false


var current_position: Vector2i = INVALID_PARCEL
var current_scene_entity_id: String = ""

var loaded_empty_scenes: Dictionary = {}
var loaded_scenes: Dictionary = {}
var current_edge_parcels: Array[Vector2i] = []
var wall_manager: FloatingIslandWalls = null
var scene_entity_coordinator: SceneEntityCoordinator = SceneEntityCoordinator.new()
var last_version_updated: int = -1
var last_version_checked: int = -1
var last_scene_group_hash: String = ""

var base_floor_manager: BaseFloorManager = null

var desired_portable_experiences_urns: Array[String] = []

# Dynamic loading mode: enables continuous scene loading/unloading without terrain generation
# Used for deep links to custom realms - provides smooth loading without freezes
var _use_dynamic_loading: bool = false

# Flag to skip loading screen during reload operations (teleports, scene reloads, etc.)
# This is a one-shot flag that gets reset after use
var _is_reloading: bool = false

# When true, the current reload is a hot reload from the preview WebSocket
# Hot reloads skip the loading screen for a smoother development experience
var _is_hot_reloading: bool = false

# Flag to purge cache on first scene load in preview mode
# This prevents the race condition where cached content is used before
# the WebSocket SCENE_UPDATE can trigger a reload
var _purge_cache_on_first_load: bool = false

# This counter is to control the async-flow
var _scene_changed_counter: int = 0

var _debugging_js_scene_id: String = ""

var _bypass_loading_check: bool = false

# Track the target parcel during teleport to ensure correct spawn point
var _teleport_target_parcel: Vector2i = INVALID_PARCEL

# Track if the coordinator has been configured (config() called)
var _coordinator_configured: bool = false

# Async floating islands generation state
var _floating_islands_generating: bool = false
var _floating_islands_generation_data: Dictionary = {}  # Captured data for generation
var _floating_islands_queue: Array = []  # Coordinates to generate
var _floating_islands_total: int = 0
var _floating_islands_created: int = 0

# Simple floor for large scenes (>100 empty parcels)
var _large_scene_floor: Node3D = null

# Preview WebSocket for hot reload
var _preview_ws := WebSocketPeer.new()
var _preview_ws_pending_url: String = ""
var _preview_ws_dirty_connected: bool = false
var _preview_ws_dirty_closed: bool = false


func _ready():
	Global.realm.realm_changed.connect(self._on_realm_changed)

	# Initialize wall manager and base floor manager only for floating islands mode
	if is_using_floating_islands():
		wall_manager = FloatingIslandWalls.new()
		add_child(wall_manager)

		base_floor_manager = BaseFloorManager.new()
		base_floor_manager.name = "BaseFloorManager"
		add_child(base_floor_manager)

		# Parcel data texture will be generated after parcels are loaded

	# Set scene radius based on mode:
	# - Floating islands: radius 5 to load scenes within range (avoids loading all scattered scenes)
	# - City/test mode: radius 0 for precise coordinate-based loading
	var scene_radius = 5 if _use_dynamic_loading else 0
	scene_entity_coordinator.set_scene_radius(scene_radius)

	Global.scene_runner.scene_killed.connect(self.on_scene_killed)
	Global.loading_finished.connect(self.on_loading_finished)


func get_current_spawn_point():
	var current_scene_data = get_current_scene_data()
	if current_scene_data == null:
		return null

	return current_scene_data.scene_entity_definition.get_global_spawn_position()


func on_loading_finished():
	var spawn_parcel = current_position
	if is_using_floating_islands() and _teleport_target_parcel != INVALID_PARCEL:
		spawn_parcel = _teleport_target_parcel
		_teleport_target_parcel = INVALID_PARCEL

	var scene_data = get_scene_data(spawn_parcel)
	if scene_data != null:
		var target_position = scene_data.scene_entity_definition.get_global_spawn_position()
		if target_position != null:
			# Trust the spawn point position, move up if inside a collider
			var valid_position := _find_valid_spawn_position(target_position)
			Global.get_explorer().move_to(valid_position, true, false)  # skip stuck check, position already validated


func on_scene_killed(killed_scene_id, _entity_id):
	for scene_entity_id in loaded_scenes.keys():
		var scene: SceneItem = loaded_scenes[scene_entity_id]
		if scene.scene_number_id == killed_scene_id:
			loaded_scenes.erase(scene_entity_id)
			return


func get_current_scene_data() -> SceneItem:
	return get_scene_data(current_position)


func get_scene_data(coord: Vector2i) -> SceneItem:
	var scene_entity_id := scene_entity_coordinator.get_scene_entity_id(coord)
	if scene_entity_id.is_empty():
		return null

	return loaded_scenes.get(scene_entity_id)


func get_scene_data_by_scene_id(scene_id: int) -> SceneItem:
	for scene: SceneItem in loaded_scenes.values():
		if scene.scene_number_id == scene_id:
			return scene

	return null


func set_scene_radius(value: int):
	scene_entity_coordinator.set_scene_radius(value)


## Enables or disables dynamic loading mode.
## In dynamic loading mode:
## - Scenes are loaded/unloaded continuously as the player moves
## - No loading screen is shown
## - No floating island terrain is generated (just simple grass floors)
## This mode is used for deep links to custom realms to provide smooth loading.
func set_dynamic_loading_mode(enabled: bool) -> void:
	if _use_dynamic_loading == enabled:
		return

	_use_dynamic_loading = enabled
	var scene_radius = 5 if _use_dynamic_loading else 0
	scene_entity_coordinator.set_scene_radius(scene_radius)

	if enabled:
		# Clear floating island state when enabling dynamic mode
		last_scene_group_hash = ""
		for parcel in loaded_empty_scenes:
			var empty_scene = loaded_empty_scenes[parcel]
			remove_child(empty_scene)
			empty_scene.queue_free()
		loaded_empty_scenes.clear()
		if wall_manager:
			wall_manager.clear_walls()
		_cleanup_simple_floor()


func is_dynamic_loading_mode() -> bool:
	return _use_dynamic_loading


# gdlint:ignore = async-function-name
func _process(_dt):
	_process_preview_ws()

	# Process async floating islands generation (2 parcels per frame)
	_process_floating_islands_batch()

	scene_entity_coordinator.update()

	var version := scene_entity_coordinator.get_version()

	# Use continuous loading when:
	# - Not using floating islands (test/renderer mode)
	# - OR dynamic loading mode is enabled (deep link to custom realm)
	var use_continuous_loading = not is_using_floating_islands() or _use_dynamic_loading
	if use_continuous_loading:
		if version != last_version_updated:
			last_version_updated = scene_entity_coordinator.get_version()
			await _async_on_desired_scene_changed()
		return

	# Once we're here, we need the logic of selected time to process the desired change
	var scene_entity_id := scene_entity_coordinator.get_scene_entity_id(current_position)

	# Skip processing when at invalid initial position to avoid spam
	var is_invalid_position = current_position == INVALID_PARCEL

	# Check if there's an actual change (not just empty -> empty)
	var has_actual_change = (
		scene_entity_id != current_scene_entity_id
		and not (scene_entity_id.is_empty() and current_scene_entity_id.is_empty())
	)

	if _bypass_loading_check or (has_actual_change and not is_invalid_position):
		current_scene_entity_id = scene_entity_id
		_bypass_loading_check = false

		if version != last_version_updated:
			last_version_updated = scene_entity_coordinator.get_version()
			notify_pending_loading_scenes.emit(false)
			await _async_on_desired_scene_changed()
	elif version != last_version_updated:
		# Version changed but we're in the same scene - still need to update for dynamic loading
		last_version_updated = scene_entity_coordinator.get_version()
		await _async_on_desired_scene_changed()
	elif version != last_version_checked:
		last_version_checked = version
		if _is_there_any_new_scene_to_load():
			notify_pending_loading_scenes.emit(true)


func is_scene_loaded(x: int, z: int) -> bool:
	var parcel_str = "%d,%d" % [x, z]
	return get_parcel_scene_id(x, z) != -1 or loaded_empty_scenes.has(parcel_str)


func get_parcel_scene_id(x: int, z: int) -> int:
	for scene_id in loaded_scenes.keys():
		var scene: SceneItem = loaded_scenes[scene_id]
		if scene.scene_number_id != -1:
			for pos in scene.parcels:
				if pos.x == x and pos.y == z:
					return scene.scene_number_id
	return -1


func _is_there_any_new_scene_to_load() -> bool:
	var d = scene_entity_coordinator.get_desired_scenes()
	var loadable_scenes = d.get("loadable_scenes", [])

	for scene_id in loadable_scenes:
		if not loaded_scenes.has(scene_id):
			var scene_definition = scene_entity_coordinator.get_scene_definition(scene_id)
			if scene_definition != null:
				return true
	return false


func _async_on_desired_scene_changed():
	var desired_scenes = scene_entity_coordinator.get_desired_scenes()
	var loadable_scenes = desired_scenes.get("loadable_scenes", [])

	_scene_changed_counter += 1
	var counter_this_call := _scene_changed_counter

	# Capture reloading flags - only consume them when there are scenes to load
	# This ensures we don't lose the flag if coordinator hasn't discovered scenes yet
	var is_reloading_now := _is_reloading
	var is_hot_reloading_now := _is_hot_reloading
	if not loadable_scenes.is_empty():
		_is_reloading = false
		_is_hot_reloading = false

	# Determine if we should show a loading screen
	# Show loading screen when:
	# - We have no scenes loaded AND there are scenes to load
	# - We're in floating islands mode
	# - Either NOT in dynamic loading mode, OR this is a teleport (user expects to wait)
	# - NOT a hot reload (preview WebSocket) — hot reloads skip loading screen
	var new_loading = (
		loaded_scenes.is_empty()
		and not loadable_scenes.is_empty()
		and is_using_floating_islands()
		and (not _use_dynamic_loading or is_reloading_now)
		and not is_hot_reloading_now
	)

	# Start a new loading session if we need to load scenes
	# The loading session tracks progress through the Rust-based LoadingSession
	var scenes_to_load: PackedStringArray = PackedStringArray()
	var loading_promises: Array = []

	for scene_id in loadable_scenes:
		var should_load = false
		if not loaded_scenes.has(scene_id):
			should_load = true
		else:
			# Check if scene is actually loaded or still loading (scene_number_id == -1)
			var scene: SceneItem = loaded_scenes[scene_id]
			if scene.scene_number_id != -1:
				# Scene is fully loaded
				new_loading = false

		if should_load:
			var scene_definition = scene_entity_coordinator.get_scene_definition(scene_id)
			if scene_definition != null:
				scenes_to_load.append(scene_id)
				loading_promises.push_back(async_load_scene.bind(scene_id, scene_definition))
			else:
				printerr("should load scene_id ", scene_id, " but data is empty")
				# Report as fetched (with null definition) so loading session can progress
				Global.scene_runner.report_scene_fetched(scene_id)

	# Start a loading session for the new scenes (cancels any existing session)
	var loading_session_started := false
	if scenes_to_load.size() > 0 and new_loading:
		Global.scene_runner.start_loading_session(scenes_to_load)
		loading_session_started = true

	# Keep the old signal for backwards compatibility
	report_scene_load.emit(false, new_loading, loadable_scenes.size())

	await PromiseUtils.async_all(loading_promises)

	# If there is other calls processing the scene, early return
	# 	the next block of code will be executed by the last request
	if counter_this_call != _scene_changed_counter:
		return

	# Get current loadable/keep_alive scenes (they may have changed while loading)
	var current_desired = scene_entity_coordinator.get_desired_scenes()
	var current_loadable = current_desired.get("loadable_scenes", [])
	var current_keep_alive = current_desired.get("keep_alive_scenes", [])

	# Clean up old scenes that are no longer needed
	var scenes_to_remove = []
	for scene_id in loaded_scenes.keys():
		var should_keep = current_loadable.has(scene_id) or current_keep_alive.has(scene_id)

		if not should_keep:
			var scene: SceneItem = loaded_scenes[scene_id]
			# Don't kill or remove scenes that are still loading (scene_number_id == -1)
			# Don't kill or remove global scenes
			if not scene.is_global and scene.scene_number_id != -1:
				Global.scene_runner.kill_scene(scene.scene_number_id)
				if base_floor_manager:
					base_floor_manager.remove_scene_floors(scene.id)
				scenes_to_remove.append(scene_id)

	for scene_id in scenes_to_remove:
		loaded_scenes.erase(scene_id)

	# Skip floating island generation in test/renderer modes or dynamic loading mode
	# Dynamic loading mode uses simple grass floors without complex terrain
	var use_floating_islands = is_using_floating_islands() and not _use_dynamic_loading

	# Clear floating island state when switching to dynamic loading
	if not use_floating_islands and !last_scene_group_hash.is_empty():
		last_scene_group_hash = ""
		# Clean up any existing floating island empty parcels
		for parcel in loaded_empty_scenes:
			var empty_scene = loaded_empty_scenes[parcel]
			remove_child(empty_scene)
			empty_scene.queue_free()
		loaded_empty_scenes.clear()
		_cleanup_simple_floor()

	if use_floating_islands:
		# Wait for coordinator to finish fetching all scene metadata before generating islands
		# This ensures we know all occupied parcels before creating the terrain
		var max_wait_frames := 300  # ~5 seconds at 60fps
		var wait_frames := 0
		var coordinator_was_busy := scene_entity_coordinator.is_busy()

		while scene_entity_coordinator.is_busy() and wait_frames < max_wait_frames:
			coordinator_was_busy = true
			await get_tree().process_frame
			wait_frames += 1
			# Check if a new call to this function was made while waiting
			if counter_this_call != _scene_changed_counter:
				return

		# If coordinator was never busy and we're reloading, wait a bit for it to start
		# This handles the case where realm just changed and coordinator hasn't queued requests yet
		if not coordinator_was_busy and is_reloading_now:
			var startup_wait := 0
			while not scene_entity_coordinator.is_busy() and startup_wait < 30:  # ~0.5s at 60fps
				await get_tree().process_frame
				startup_wait += 1
				if counter_this_call != _scene_changed_counter:
					return

			# Now wait for it to finish
			while scene_entity_coordinator.is_busy() and wait_frames < max_wait_frames:
				coordinator_was_busy = true
				await get_tree().process_frame
				wait_frames += 1
				if counter_this_call != _scene_changed_counter:
					return

		if wait_frames >= max_wait_frames:
			printerr(
				"WARNING: Timed out waiting for scene_entity_coordinator to finish fetching scenes"
			)

		# After waiting, check if new scenes were discovered and load them
		var final_desired = scene_entity_coordinator.get_desired_scenes()
		var final_loadable = final_desired.get("loadable_scenes", [])
		var new_scenes_to_load: PackedStringArray = PackedStringArray()
		var new_loading_promises: Array = []

		for scene_id in final_loadable:
			if not loaded_scenes.has(scene_id):
				var scene_definition = scene_entity_coordinator.get_scene_definition(scene_id)
				if scene_definition != null:
					new_scenes_to_load.append(scene_id)
					new_loading_promises.push_back(
						async_load_scene.bind(scene_id, scene_definition)
					)

		# Start a loading session for newly discovered scenes
		if new_scenes_to_load.size() > 0:
			Global.scene_runner.start_loading_session(new_scenes_to_load)
			loading_session_started = true

		if new_loading_promises.size() > 0:
			await PromiseUtils.async_all(new_loading_promises)
			# Check again if a new call was made while loading
			if counter_this_call != _scene_changed_counter:
				return

		_regenerate_floating_islands()

		# If no loading session was started but we're in floating islands mode,
		# emit loading_complete to hide the loading screen (e.g., teleporting to empty parcel)
		# Only emit if:
		# - Coordinator was busy at some point (meaning it fetched and found no scenes), OR
		# - We're not in a reloading state (normal update, not teleport/realm change)
		if not loading_session_started and (coordinator_was_busy or not is_reloading_now):
			Global.scene_runner.loading_complete.emit(-1)

	var empty_parcels_coords = []
	if use_floating_islands and !last_scene_group_hash.is_empty():
		# Use floating island empty parcels
		for parcel_string in loaded_empty_scenes.keys():
			var coord = parcel_string.split(",")
			var x = int(coord[0])
			var z = int(coord[1])
			empty_parcels_coords.push_back(Vector2i(x, z))
	# Note: In test/renderer mode (use_floating_islands = false), we don't render empty parcels at all
	# This prevents grass/terrain from appearing in scene snapshots

	var parcel_filled = []
	for scene: SceneItem in loaded_scenes.values():
		parcel_filled.append_array(scene.parcels)

	# Always emit new_loading=false when done=true to properly dismiss the loading screen
	report_scene_load.emit(true, false, loadable_scenes.size())
	parcels_processed.emit(parcel_filled, empty_parcels_coords)


func _on_realm_changed():
	var content_base_url = Global.realm.content_base_url

	Global.get_config().last_realm_joined = Global.realm.realm_url
	Global.get_config().save_to_settings_file()

	# In preview mode, purge cache on first scene load to avoid using stale cached content
	# This prevents the race condition where cached content is used before
	# the WebSocket SCENE_UPDATE can trigger a reload
	var is_preview_mode = Global.cli.preview_mode or not Global.deep_link_obj.preview.is_empty()
	if is_preview_mode:
		_purge_cache_on_first_load = true

	# Check if we should use dynamic loading mode
	# Dynamic loading is enabled via deep link parameter: &dynamic-scene-loading=true
	# This works for any realm including Genesis City
	var deep_link_dynamic = (
		Global.deep_link_obj.dynamic_scene_loading if Global.deep_link_obj else false
	)
	var should_use_dynamic = deep_link_dynamic and is_using_floating_islands()
	set_dynamic_loading_mode(should_use_dynamic)

	var scenes_urns: Array = Global.realm.realm_about.get("configurations", {}).get("scenesUrn", [])

	# Force floating island recreation on realm change
	last_scene_group_hash = ""

	if not Global.realm.realm_city_loader_content_base_url.is_empty():
		content_base_url = Global.realm.realm_city_loader_content_base_url

	# Use floating islands mode (single scene) by default
	# Only use dynamic city mode (radius-based) in test/renderer modes
	var should_load_city_pointers = not is_using_floating_islands()

	scene_entity_coordinator.config(
		content_base_url + "entities/active", content_base_url, should_load_city_pointers
	)
	_coordinator_configured = true
	scene_entity_coordinator.set_fixed_desired_entities_urns(scenes_urns)
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)

	set_portable_experiences_urns(self.desired_portable_experiences_urns)

	for scene: SceneItem in loaded_scenes.values():
		if not scene.is_global and scene.scene_number_id != -1:
			Global.scene_runner.kill_scene(scene.scene_number_id)

	for parcel in loaded_empty_scenes:
		var empty_parcel = loaded_empty_scenes[parcel]
		remove_child(empty_parcel)
		empty_parcel.queue_free()

	loaded_empty_scenes.clear()
	if wall_manager:
		wall_manager.clear_walls()
	_cleanup_simple_floor()

	loaded_scenes = {}


func set_portable_experiences_urns(urns: Array[String]) -> void:
	var global_scenes_urns: Array = (
		Global.realm.realm_about.get("configurations", {}).get("globalScenesUrn", []).duplicate()
	)

	desired_portable_experiences_urns = urns
	global_scenes_urns.append_array(desired_portable_experiences_urns)
	scene_entity_coordinator.set_fixed_desired_entities_global_urns(global_scenes_urns)


func get_scene_by_req_id(request_id: int):
	for scene: SceneItem in loaded_scenes.values():
		if scene.main_js_req_id == request_id or scene.main_crdt_req_id == request_id:
			return scene

	return null


func is_using_floating_islands() -> bool:
	return not (
		Global.cli.scene_test_mode or Global.cli.scene_renderer_mode or Global.cli.preview_mode
	)


func _unload_scenes_except_current(current_scene_id: int) -> void:
	var scenes_to_remove = []

	for scene_id in loaded_scenes:
		var scene: SceneItem = loaded_scenes[scene_id]
		if scene.scene_number_id != current_scene_id and scene.scene_number_id != -1:
			# Kill the scene
			Global.scene_runner.kill_scene(scene.scene_number_id)
			# Remove base floors
			if base_floor_manager:
				base_floor_manager.remove_scene_floors(scene.id)
			scenes_to_remove.append(scene_id)

	# Remove from loaded_scenes
	for scene_id in scenes_to_remove:
		loaded_scenes.erase(scene_id)


func _regenerate_floating_islands() -> void:
	# Guard against overlapping generation
	if _floating_islands_generating:
		return

	# Collect parcels from ALL loaded scenes (not just player's current scene)
	var all_scene_parcels = []
	for scene_id in loaded_scenes.keys():
		var scene: SceneItem = loaded_scenes[scene_id]

		# Include parcels from all non-global scenes (including those still loading)
		# We know their parcels from metadata even before they're fully spawned
		if not scene.is_global:
			for parcel in scene.parcels:
				all_scene_parcels.append(parcel)

	var is_empty_parcel_mode := all_scene_parcels.is_empty()
	var empty_parcel_center := INVALID_PARCEL

	if is_empty_parcel_mode:
		# No scenes loaded - use current position as center of the floating island
		var target_parcel = current_position
		if _teleport_target_parcel != INVALID_PARCEL:
			target_parcel = _teleport_target_parcel

		if target_parcel == INVALID_PARCEL:
			return

		empty_parcel_center = target_parcel
		# Treat current position as if it were a "scene" to generate the surrounding island
		all_scene_parcels = [target_parcel]

	var current_scene_group_hash: String = str(all_scene_parcels.hash())

	# Skip if same scene configuration
	if !last_scene_group_hash.is_empty() and last_scene_group_hash == current_scene_group_hash:
		return

	last_scene_group_hash = current_scene_group_hash

	# Calculate bounds
	var min_x = all_scene_parcels[0].x
	var max_x = all_scene_parcels[0].x
	var min_z = all_scene_parcels[0].y
	var max_z = all_scene_parcels[0].y

	for parcel in all_scene_parcels:
		min_x = min(min_x, parcel.x)
		max_x = max(max_x, parcel.x)
		min_z = min(min_z, parcel.y)
		max_z = max(max_z, parcel.y)

	# Create a set of scene parcels for quick lookup (captured data)
	var scene_parcel_set = {}
	for parcel in all_scene_parcels:
		scene_parcel_set[Vector2i(parcel.x, parcel.y)] = true

	var padding = 2

	# Build queue of coordinates to generate
	_floating_islands_queue = []
	for x in range(min_x - padding, max_x + padding + 1):
		for z in range(min_z - padding, max_z + padding + 1):
			var coord = Vector2i(x, z)
			# Only add empty parcels if they're not occupied by actual scenes
			if not scene_parcel_set.has(coord):
				_floating_islands_queue.append(coord)

	_floating_islands_total = _floating_islands_queue.size()
	_floating_islands_created = 0

	# For large scenes, use simple floor instead of individual floating islands
	if _floating_islands_total > MAX_EMPTY_PARCELS_FOR_FLOATING_ISLANDS:
		_create_simple_floor_with_cliffs(min_x, max_x, min_z, max_z, padding)
		_floating_islands_generating = false
		_floating_islands_queue.clear()
		Global.scene_runner.start_floating_islands(0)
		Global.scene_runner.finish_floating_islands()
		return

	# Capture all needed data for async generation
	_floating_islands_generation_data = {
		"scene_parcel_set": scene_parcel_set,
		"min_x": min_x,
		"max_x": max_x,
		"min_z": min_z,
		"max_z": max_z,
		"padding": padding,
		"is_empty_parcel_mode": is_empty_parcel_mode,
		"empty_parcel_center": empty_parcel_center
	}

	# Signal floating islands generation start with count
	Global.scene_runner.start_floating_islands(_floating_islands_total)

	# Clear old empty parcels
	for parcel in loaded_empty_scenes:
		var empty_scene = loaded_empty_scenes[parcel]
		remove_child(empty_scene)
		empty_scene.queue_free()
	loaded_empty_scenes.clear()
	current_edge_parcels.clear()
	if wall_manager:
		wall_manager.clear_walls()

	# Start async generation
	_floating_islands_generating = true


## Process async floating islands generation - 2 parcels per frame
func _process_floating_islands_batch() -> void:
	if not _floating_islands_generating:
		return

	# Process 2 parcels per frame
	var parcels_this_frame = mini(2, _floating_islands_queue.size())
	for i in parcels_this_frame:
		var coord = _floating_islands_queue.pop_front()
		_create_single_empty_parcel(coord)
		_floating_islands_created += 1

	# Report progress
	Global.scene_runner.report_floating_islands_progress(
		_floating_islands_created, _floating_islands_total
	)

	# Check if generation is complete
	if _floating_islands_queue.is_empty():
		_finish_floating_islands_generation()


## Create a single empty parcel at the given coordinate
func _create_single_empty_parcel(coord: Vector2i) -> void:
	var x = coord.x
	var z = coord.y
	var parcel_string = "%d,%d" % [x, z]

	if loaded_empty_scenes.has(parcel_string):
		return  # Already exists

	var data = _floating_islands_generation_data
	var scene_parcel_set = data.get("scene_parcel_set", {})
	var min_x = data.get("min_x", 0)
	var max_x = data.get("max_x", 0)
	var min_z = data.get("min_z", 0)
	var max_z = data.get("max_z", 0)
	var padding = data.get("padding", 2)

	var scene: Node3D = EMPTY_SCENE.instantiate()
	var temp := "EP_%s_%s" % [str(x).replace("-", "m"), str(z).replace("-", "m")]
	scene.name = temp
	add_child(scene)
	scene.global_position = Vector3(
		x * EmptyParcel.PARCEL_SIZE + EmptyParcel.PARCEL_HALF_SIZE,
		0,
		-z * EmptyParcel.PARCEL_SIZE - EmptyParcel.PARCEL_HALF_SIZE
	)

	var config = _calculate_parcel_adjacency(
		x, z, min_x - padding, max_x + padding, min_z - padding, max_z + padding, scene_parcel_set
	)
	scene.set_corner_configuration.call_deferred(config)

	loaded_empty_scenes[parcel_string] = scene


## Finish floating islands generation (create walls, handle empty parcel mode, signal completion)
func _finish_floating_islands_generation() -> void:
	var data = _floating_islands_generation_data
	var min_x: int = data.get("min_x", 0)
	var max_x: int = data.get("max_x", 0)
	var min_z: int = data.get("min_z", 0)
	var max_z: int = data.get("max_z", 0)
	var padding: int = data.get("padding", 2)
	var is_empty_parcel_mode: bool = data.get("is_empty_parcel_mode", false)
	var empty_parcel_center: Vector2i = data.get("empty_parcel_center", INVALID_PARCEL)

	# Create walls
	if wall_manager:
		wall_manager.create_walls_for_bounds(min_x, max_x, min_z, max_z, padding)

	# For empty parcel mode, also create an empty parcel at the center
	if is_empty_parcel_mode and empty_parcel_center != INVALID_PARCEL:
		var x := empty_parcel_center.x
		var z := empty_parcel_center.y
		var parcel_string := "%d,%d" % [x, z]

		var scene: Node3D = EMPTY_SCENE.instantiate()
		var temp := "EP_%s_%s" % [str(x).replace("-", "m"), str(z).replace("-", "m")]
		scene.name = temp
		add_child(scene)
		scene.global_position = Vector3(
			x * EmptyParcel.PARCEL_SIZE + EmptyParcel.PARCEL_HALF_SIZE,
			0,
			-z * EmptyParcel.PARCEL_SIZE - EmptyParcel.PARCEL_HALF_SIZE
		)

		# Center parcel has all neighbors as LOADED (flat terrain with edge strips)
		var config := CornerConfiguration.new()
		config.north = CornerConfiguration.ParcelState.LOADED
		config.south = CornerConfiguration.ParcelState.LOADED
		config.east = CornerConfiguration.ParcelState.LOADED
		config.west = CornerConfiguration.ParcelState.LOADED
		config.northwest = CornerConfiguration.ParcelState.LOADED
		config.northeast = CornerConfiguration.ParcelState.LOADED
		config.southwest = CornerConfiguration.ParcelState.LOADED
		config.southeast = CornerConfiguration.ParcelState.LOADED
		scene.set_corner_configuration.call_deferred(config)

		loaded_empty_scenes[parcel_string] = scene

		# Spawn player after terrain is generated
		var terrain_gen = scene.get_node("TerrainGenerator")
		terrain_gen.terrain_generated.connect(
			_async_spawn_on_empty_parcel.bind(empty_parcel_center), CONNECT_ONE_SHOT
		)

	# Clear generation state
	_floating_islands_generating = false
	_floating_islands_generation_data = {}
	_floating_islands_queue = []

	# Signal floating islands generation complete (100%)
	Global.scene_runner.finish_floating_islands()


## Clean up the simple floor used for large scenes
func _cleanup_simple_floor() -> void:
	if _large_scene_floor != null:
		remove_child(_large_scene_floor)
		_large_scene_floor.queue_free()
		_large_scene_floor = null


## Create a simple floor with fixed cliffs for large scenes (>100 empty parcels)
## This reduces memory from ~450MB (individual floating islands) to ~10MB (single floor + 4 cliffs)
func _create_simple_floor_with_cliffs(
	min_x: int, max_x: int, min_z: int, max_z: int, padding: int
) -> void:
	# Clean up any previous simple floor
	_cleanup_simple_floor()

	# Clean up existing floating islands
	for parcel in loaded_empty_scenes:
		var empty_scene = loaded_empty_scenes[parcel]
		remove_child(empty_scene)
		empty_scene.queue_free()
	loaded_empty_scenes.clear()
	current_edge_parcels.clear()
	if wall_manager:
		wall_manager.clear_walls()

	_large_scene_floor = Node3D.new()
	_large_scene_floor.name = "LargeSceneFloor"
	add_child(_large_scene_floor)

	# Calculate world-space bounds
	var world_min_x = (min_x - padding) * EmptyParcel.PARCEL_SIZE
	var world_max_x = (max_x + padding + 1) * EmptyParcel.PARCEL_SIZE
	var world_min_z = -(max_z + padding + 1) * EmptyParcel.PARCEL_SIZE
	var world_max_z = -(min_z - padding) * EmptyParcel.PARCEL_SIZE

	var width = world_max_x - world_min_x
	var height = world_max_z - world_min_z
	var center_x = (world_min_x + world_max_x) / 2.0
	var center_z = (world_min_z + world_max_z) / 2.0

	# Create floor mesh
	_create_floor_mesh(width, height, center_x, center_z)

	# Create collision
	_create_floor_collision(width, height, center_x, center_z)

	# Create cliffs on all 4 sides
	var cliff_height = 30.0
	# North cliff (facing +Z toward center)
	_create_cliff(
		Vector3(center_x, -cliff_height / 2.0, world_min_z),
		Vector2(width, cliff_height),
		Vector3(0, PI, 0)
	)
	# South cliff (facing -Z toward center)
	_create_cliff(
		Vector3(center_x, -cliff_height / 2.0, world_max_z),
		Vector2(width, cliff_height),
		Vector3.ZERO
	)
	# West cliff (facing +X toward center)
	_create_cliff(
		Vector3(world_min_x, -cliff_height / 2.0, center_z),
		Vector2(height, cliff_height),
		Vector3(0, -PI / 2.0, 0)
	)
	# East cliff (facing -X toward center)
	_create_cliff(
		Vector3(world_max_x, -cliff_height / 2.0, center_z),
		Vector2(height, cliff_height),
		Vector3(0, PI / 2.0, 0)
	)

	# Create walls
	if wall_manager:
		wall_manager.create_walls_for_bounds(min_x, max_x, min_z, max_z, padding)


## Create the floor mesh for simple floor mode
func _create_floor_mesh(width: float, height: float, center_x: float, center_z: float) -> void:
	var mesh_instance := MeshInstance3D.new()
	mesh_instance.name = "FloorMesh"

	var plane_mesh := PlaneMesh.new()
	plane_mesh.size = Vector2(width, height)
	mesh_instance.mesh = plane_mesh

	# Use the same material as floating islands (grass texture)
	# The default grass_rect in the material already points to the bottom-left quadrant
	mesh_instance.material_override = EMPTY_PARCEL_MATERIAL

	_large_scene_floor.add_child(mesh_instance)
	mesh_instance.global_position = Vector3(center_x, -0.05, center_z)


## Create the floor collision for simple floor mode
func _create_floor_collision(width: float, height: float, center_x: float, center_z: float) -> void:
	var static_body := StaticBody3D.new()
	static_body.name = "FloorCollision"
	static_body.collision_layer = EmptyParcel.OBSTACLE_COLLISION_LAYER

	var collision_shape := CollisionShape3D.new()
	var box_shape := BoxShape3D.new()
	box_shape.size = Vector3(width, 0.1, height)
	collision_shape.shape = box_shape

	static_body.add_child(collision_shape)

	_large_scene_floor.add_child(static_body)
	static_body.global_position = Vector3(center_x, -0.05, center_z)


## Create a cliff plane on one side of the simple floor
func _create_cliff(position: Vector3, size: Vector2, rotation: Vector3) -> void:
	var mesh_instance := MeshInstance3D.new()
	mesh_instance.name = "Cliff"

	var plane_mesh := PlaneMesh.new()
	plane_mesh.size = size
	plane_mesh.orientation = PlaneMesh.FACE_Z
	mesh_instance.mesh = plane_mesh

	mesh_instance.material_override = CLIFF_MATERIAL

	_large_scene_floor.add_child(mesh_instance)
	mesh_instance.global_position = position
	mesh_instance.rotation = rotation


func _async_spawn_on_empty_parcel(parcel: Vector2i) -> void:
	# Wait one more physics frame to ensure collision shape is ready
	await get_tree().physics_frame
	var parcel_center := Vector3(
		parcel.x * EmptyParcel.PARCEL_SIZE + EmptyParcel.PARCEL_HALF_SIZE,
		0,
		-parcel.y * EmptyParcel.PARCEL_SIZE - EmptyParcel.PARCEL_HALF_SIZE
	)
	# Trust the spawn point position, move up if inside a collider
	var valid_position := _find_valid_spawn_position(parcel_center)
	Global.get_explorer().move_to(valid_position, true, false)  # skip stuck check, position already validated


## Finds a valid spawn position by trusting the spawn point and moving up if inside a collider.
## This avoids the issue where raycasting from above causes players to spawn on top of tall objects.
func _find_valid_spawn_position(spawn_position: Vector3) -> Vector3:
	var space_state := get_tree().root.get_world_3d().direct_space_state
	var check_position := spawn_position

	# Create a small sphere query to check for collisions at player position
	var query := PhysicsShapeQueryParameters3D.new()
	var sphere := SphereShape3D.new()
	sphere.radius = 0.3  # Small check radius
	query.shape = sphere
	# Layer 1 = scene geometry, Layer 2 = empty parcel terrain
	query.collision_mask = 3
	query.collide_with_bodies = true
	query.collide_with_areas = false

	# Move up in small increments until we find a clear position
	var max_iterations := 50  # Prevent infinite loop (50m max)
	for i in range(max_iterations):
		query.transform = Transform3D(Basis.IDENTITY, check_position)
		var results := space_state.intersect_shape(query, 1)

		if results.is_empty():
			return check_position  # Found clear position

		check_position.y += 1.0  # Move up 1 meter

	# Fallback: return original position if nothing found
	return spawn_position


func update_position(new_position: Vector2i, is_teleport: bool) -> void:
	# Skip only if not teleporting and position hasn't changed
	# Always process teleports, even to the same location (to force scene reload)
	if current_position == new_position and !is_teleport:
		return

	# For teleports to the same position, we still want to reload the scene
	var position_changed = current_position != new_position
	current_position = new_position

	if is_teleport:
		_teleport_target_parcel = new_position

		# Mark as reloading to show loading screen even in dynamic loading mode
		# This ensures scenes are properly loaded before user interaction
		_is_reloading = true

		# If teleporting to the same position, force scene processing even if we're in the same scene entity
		if not position_changed:
			if is_using_floating_islands():
				_bypass_loading_check = true

		for scene_id in loaded_scenes.keys():
			var scene: SceneItem = loaded_scenes[scene_id]
			if not scene.is_global and scene.scene_number_id != -1:
				Global.scene_runner.kill_scene(scene.scene_number_id)
				if base_floor_manager:
					base_floor_manager.remove_scene_floors(scene.id)

		loaded_scenes.clear()

		# Clear floating island hash to force regeneration on next load
		last_scene_group_hash = ""

	# Update coordinator position when:
	# - Not using floating islands (test/renderer mode)
	# - OR is a teleport
	# - OR dynamic loading mode is enabled (need continuous position updates)
	# Only if coordinator has been configured (to avoid errors during initialization)
	if _coordinator_configured:
		if not is_using_floating_islands() or is_teleport or _use_dynamic_loading:
			scene_entity_coordinator.set_current_position(current_position.x, current_position.y)

	player_parcel_changed.emit(new_position)


func async_load_scene(
	scene_entity_id: String, scene_entity_definition: DclSceneEntityDefinition
) -> Promise:
	# Check if scene is already in loaded_scenes
	if loaded_scenes.has(scene_entity_id):
		var existing_scene: SceneItem = loaded_scenes[scene_entity_id]
		printerr(
			"WARNING: Scene already in loaded_scenes! scene_number_id:",
			existing_scene.scene_number_id
		)

	var parcels := scene_entity_definition.get_parcels()

	var scene_item: SceneItem = SceneItem.new()
	scene_item.id = scene_entity_id
	scene_item.scene_entity_definition = scene_entity_definition
	scene_item.scene_number_id = -1
	scene_item.parcels = parcels
	scene_item.is_global = scene_entity_definition.is_global()

	loaded_scenes[scene_entity_id] = scene_item

	# Warn if floating islands were already generated before this scene was discovered
	# This indicates a timing issue where scene_entity_coordinator discovered scenes late
	if is_using_floating_islands() and not _use_dynamic_loading:
		if not last_scene_group_hash.is_empty() and not scene_item.is_global:
			printerr(
				"WARNING: Scene ",
				scene_entity_id,
				" discovered after floating islands were generated. ",
				"Parcels: ",
				parcels,
				". Islands will be regenerated."
			)

	var content_mapping := scene_entity_definition.get_content_mapping()

	# In preview mode, purge cached files on first load to ensure fresh content
	# This prevents the race condition where cached content is used before
	# the WebSocket SCENE_UPDATE can trigger a reload
	if _purge_cache_on_first_load:
		_purge_cache_on_first_load = false
		var files: PackedStringArray = content_mapping.get_files()
		var purge_promises: Array = []
		for file_path in files:
			var file_hash = content_mapping.get_hash(file_path)
			purge_promises.push_back(Global.content_provider.purge_file(file_hash))
		# Await all purge operations to complete before fetching
		await PromiseUtils.async_all(purge_promises)

	var local_main_js_path: String = ""
	var script_promise: Promise = null
	if scene_entity_definition.is_sdk7():
		var script_path := scene_entity_definition.get_main_js_path()
		script_promise = Global.content_provider.fetch_file(script_path, content_mapping)
		local_main_js_path = "user://content/" + scene_entity_definition.get_main_js_hash()
	else:
		if (
			not FIXED_LOCAL_ADAPTATION_LAYER.is_empty()
			and FileAccess.file_exists(FIXED_LOCAL_ADAPTATION_LAYER)
		):
			local_main_js_path = String(FIXED_LOCAL_ADAPTATION_LAYER)
		else:
			var script_hash = "sdk-adaptation-layer.js"
			script_promise = Global.content_provider.fetch_file_by_url(
				script_hash, ADAPTATION_LAYER_URL
			)
			local_main_js_path = "user://content/" + script_hash

	if script_promise != null:
		var script_res = await PromiseUtils.async_awaiter(script_promise)
		if script_res is PromiseError:
			printerr(
				"Scene ",
				scene_entity_id,
				" fail getting the script code content, error message: ",
				script_res.get_error()
			)
			# Still report as fetched (with error) so loading session can progress
			Global.scene_runner.report_scene_fetched(scene_entity_id)

			send_scene_failed_metrics(
				scene_entity_id, "script_fetch_failed", script_res.get_error()
			)

			return PromiseUtils.resolved(false)

	var main_crdt_file_hash := scene_entity_definition.get_main_crdt_hash()
	var local_main_crdt_path: String = String()
	if not main_crdt_file_hash.is_empty():
		local_main_crdt_path = "user://content/" + main_crdt_file_hash
		var promise: Promise = Global.content_provider.fetch_file("main.crdt", content_mapping)

		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			printerr(
				"Scene ",
				scene_entity_id,
				" fail getting the main crdt content, error message: ",
				res.get_error()
			)
			# Still report as fetched (with error) so loading session can progress
			Global.scene_runner.report_scene_fetched(scene_entity_id)

			send_scene_failed_metrics(scene_entity_id, "crdt_fetch_failed", res.get_error())

			return PromiseUtils.resolved(false)

	var scene_hash_zip: String = "%s-mobile.zip" % scene_entity_id
	var asset_url: String = (
		"%s/%s-mobile.zip" % [Global.content_provider.get_optimized_base_url(), scene_entity_id]
	)

	# Skip optimized zip download when:
	# - XR mode (handled separately)
	# - Testing scene mode (handled separately)
	# - --only-no-optimized flag (explicitly loading non-optimized scenes)
	var skip_optimized = (
		Global.is_xr() or Global.get_testing_scene_mode() or Global.cli.only_no_optimized
	)

	var download_success := false
	var download_error: PromiseError = null
	var file_not_found_remotely := false
	if not skip_optimized:
		# Check if optimized zip already exists to avoid re-download hang
		var zip_file_path = "user://content/" + scene_hash_zip
		if FileAccess.file_exists(zip_file_path):
			download_success = true
		else:
			# First check if the file exists remotely (HEAD request)
			# This avoids treating 404s as errors - scenes without optimized versions are expected
			var exists_promise = Global.content_provider.check_remote_file_exists(asset_url)
			var exists_res = await PromiseUtils.async_awaiter(exists_promise)

			if exists_res is PromiseError or exists_res == false:
				# File doesn't exist remotely or check failed - this is expected for non-optimized scenes
				file_not_found_remotely = true
			else:
				# File exists remotely, proceed with download
				var download_promise: Promise = Global.content_provider.fetch_file_by_url(
					scene_hash_zip, asset_url
				)
				var download_res = await PromiseUtils.async_awaiter(download_promise)
				if download_res is PromiseError:
					download_error = download_res
				else:
					download_success = true

	if skip_optimized:
		pass  # Scene optimization skipped (XR, testing, or --only-no-optimized)
	elif file_not_found_remotely:
		# Optimized version not available - expected for non-optimized scenes
		# --only-optimized: Skip scene if it's not optimized
		if Global.cli.only_optimized:
			printerr("Scene ", scene_entity_id, " skipped (--only-optimized flag set)")
			# Still report as fetched so loading session can progress
			Global.scene_runner.report_scene_fetched(scene_entity_id)
			loaded_scenes.erase(scene_entity_id)
			return PromiseUtils.resolved(false)
	elif download_error != null or not download_success:
		printerr(
			"Scene ", scene_entity_id, " failed to download optimized zip asset_url=", asset_url
		)

		send_scene_failed_metrics(scene_entity_id, "zip_download_failed")

		# --only-optimized: Skip scene if download failed
		if Global.cli.only_optimized:
			printerr("Scene ", scene_entity_id, " skipped (--only-optimized flag set)")
			# Still report as fetched so loading session can progress
			Global.scene_runner.report_scene_fetched(scene_entity_id)
			loaded_scenes.erase(scene_entity_id)
			return PromiseUtils.resolved(false)
	else:
		var ok = ProjectSettings.load_resource_pack("user://content/" + scene_hash_zip, false)
		if not ok:
			printerr("Scene ", scene_entity_id, " failed to load optimized scene, error #1")

			send_scene_failed_metrics(scene_entity_id, "optimized_scene_load_failed")
		else:
			var optimized_metadata_path = "res://" + scene_entity_id + "-optimized.json"
			var file = FileAccess.open(optimized_metadata_path, FileAccess.READ)
			if file:
				# Read the file's content as a string
				var json_string = file.get_as_text()
				var add_promise = Global.content_provider.load_optimized_assets_metadata(
					json_string
				)
				file.close()
				await PromiseUtils.async_awaiter(add_promise)
				print("Scene ", scene_entity_id, " optimized assets metadata loaded successfully.")
			else:
				printerr("Scene ", scene_entity_id, " failed to load optimized scene, error #2")
				send_scene_failed_metrics(
					scene_entity_id,
					"optimized_scene_json_load_failed",
					error_string(FileAccess.get_open_error())
				)

	# the scene was removed while it was loading...
	if not loaded_scenes.has(scene_entity_id):
		printerr("Scene was removed while loading:", scene_entity_id)
		# Still report as fetched so loading session can progress
		Global.scene_runner.report_scene_fetched(scene_entity_id)
		send_scene_failed_metrics(scene_entity_id, "scene_removed_while_loading")
		return PromiseUtils.resolved(false)

	# Report that this scene's metadata/content has been fetched
	Global.scene_runner.report_scene_fetched(scene_entity_id)

	var scene_in_dict = loaded_scenes[scene_entity_id]
	_on_try_spawn_scene(scene_in_dict, local_main_js_path, local_main_crdt_path)
	return PromiseUtils.resolved(true)


## Sends metrics when the scene load fails
func send_scene_failed_metrics(
	scene_entity_id: String, error_str: String, error_message: String = ""
) -> void:
	# LOADING_END (Failed) metric
	var error_data = {
		"scene_id": scene_entity_id,
		"position": "%d,%d" % [current_position.x, current_position.y],
		"status": "Failed",
		"error": error_str
	}
	if error_message != "":
		error_data["error_message"] = error_message
	Global.metrics.track_screen_viewed("LOADING_END", JSON.stringify(error_data))


func _on_try_spawn_scene(
	scene_item: SceneItem, local_main_js_path: String, local_main_crdt_path: String
):
	if not local_main_js_path.is_empty() and not FileAccess.file_exists(local_main_js_path):
		printerr("Couldn't get main.js file:", local_main_js_path)
		local_main_js_path = ""

	if not local_main_crdt_path.is_empty() and not FileAccess.file_exists(local_main_crdt_path):
		printerr("Couldn't get main.crdt file:", local_main_crdt_path)
		local_main_crdt_path = ""

	if local_main_crdt_path.is_empty() and local_main_js_path.is_empty():
		printerr("Couldn't spawn the scene (no js/crdt):", scene_item.id)
		return false

	var enable_js_inspector: bool = false
	if Global.has_javascript_debugger and _debugging_js_scene_id == scene_item.id:
		enable_js_inspector = true

	var scene_number_id: int = Global.scene_runner.start_scene(
		local_main_js_path,
		local_main_crdt_path,
		scene_item.scene_entity_definition,
		enable_js_inspector
	)
	scene_item.scene_number_id = scene_number_id

	# Add base floors for this scene's parcels
	if base_floor_manager:
		base_floor_manager.add_scene_floors(scene_item.id, scene_item.parcels)

	# Regenerate floating islands after scene spawns (deferred to ensure scene is fully initialized)
	# Skip in dynamic loading mode - we use simple base floors instead
	if is_using_floating_islands() and not _use_dynamic_loading:
		_regenerate_floating_islands.call_deferred()

	return true


func reload_scene(scene_id: String) -> void:
	var scene = loaded_scenes.get(scene_id)
	if scene != null:
		var scene_number_id: int = scene.scene_number_id
		if scene_number_id != -1:
			Global.scene_runner.kill_scene(scene_number_id)
			if base_floor_manager:
				base_floor_manager.remove_scene_floors(scene_id)

		var content_mapping: DclContentMappingAndUrl = (
			scene.scene_entity_definition.get_content_mapping()
		)
		var files: PackedStringArray = content_mapping.get_files()
		if files.size() > 0:
			for file_path in files:
				var file_hash = content_mapping.get_hash(file_path)
				Global.content_provider.purge_file(file_hash)

		loaded_scenes.erase(scene_id)
		scene_entity_coordinator.reload_scene_data(scene_id)
		_is_reloading = true


func set_preview_url(url: String) -> void:
	_preview_ws_pending_url = (url.to_lower().replace("http://", "ws://").replace(
		"https://", "wss://"
	))


func _process_preview_ws():
	_preview_ws.poll()

	var state = _preview_ws.get_ready_state()
	if state == WebSocketPeer.STATE_OPEN:
		if not _preview_ws_pending_url.is_empty():
			_preview_ws.close()

		if _preview_ws_dirty_connected:
			_preview_ws_dirty_connected = false
			_preview_ws_dirty_closed = true

		while _preview_ws.get_available_packet_count():
			var packet = _preview_ws.get_packet().get_string_from_utf8()
			var json = JSON.parse_string(packet)
			if json != null and json is Dictionary:
				var msg_type = json.get("type", "")
				match msg_type:
					"SCENE_UPDATE":
						var scene_id = json.get("payload", {}).get("sceneId", "unknown")
						_is_hot_reloading = true
						reload_scene(scene_id)
					_:
						printerr("preview-ws > unknown message type ", msg_type)

	elif state == WebSocketPeer.STATE_CLOSING:
		_preview_ws_dirty_closed = true
	elif state == WebSocketPeer.STATE_CLOSED:
		if _preview_ws_dirty_closed:
			_preview_ws_dirty_closed = false

		if not _preview_ws_pending_url.is_empty():
			_preview_ws.connect_to_url(_preview_ws_pending_url)
			_preview_ws_pending_url = ""
			_preview_ws_dirty_connected = true


func set_debugging_js_scene_id(id: String) -> void:
	_debugging_js_scene_id = id


func get_edge_parcels() -> Array[Vector2i]:
	return current_edge_parcels


func _find_cluster_containing_parcel(clusters: Array, parcel: Vector2i):
	for cluster in clusters:
		for cluster_parcel in cluster:
			if cluster_parcel.x == parcel.x and cluster_parcel.y == parcel.y:
				return cluster
	return null


func _cluster_parcels(parcels: Array) -> Array:
	if parcels.is_empty():
		return []

	var clusters = []
	var max_cluster_distance = 10  # Parcels within 10 units are considered part of the same island

	for parcel in parcels:
		var added_to_cluster = false

		# Try to add to an existing cluster
		for cluster in clusters:
			# Check if this parcel is close to any parcel in the cluster
			for cluster_parcel in cluster:
				var distance = abs(parcel.x - cluster_parcel.x) + abs(parcel.y - cluster_parcel.y)
				if distance <= max_cluster_distance:
					cluster.append(parcel)
					added_to_cluster = true
					break
			if added_to_cluster:
				break

		# If not added to any cluster, create a new one
		if not added_to_cluster:
			clusters.append([parcel])

	return clusters


func _create_floating_island_for_cluster(cluster: Array):
	if cluster.is_empty():
		return

	var min_x = cluster[0].x
	var max_x = cluster[0].x
	var min_z = cluster[0].y
	var max_z = cluster[0].y

	for parcel in cluster:
		min_x = min(min_x, parcel.x)
		max_x = max(max_x, parcel.x)
		min_z = min(min_z, parcel.y)
		max_z = max(max_z, parcel.y)

	# Create a set of scene parcels for quick lookup
	var scene_parcel_set = {}
	for parcel in cluster:
		scene_parcel_set[Vector2i(parcel.x, parcel.y)] = true

	# Create 2-parcel padding around the bounds
	var padding = 2
	for x in range(min_x - padding, max_x + padding + 1):
		for z in range(min_z - padding, max_z + padding + 1):
			var coord = Vector2i(x, z)
			# Only add empty parcels if they're not occupied by actual scenes
			if not scene_parcel_set.has(coord):
				var parcel_string = "%d,%d" % [x, z]

				if not loaded_empty_scenes.has(parcel_string):
					var scene: Node3D = EMPTY_SCENE.instantiate()
					var temp := "EP_%s_%s" % [str(x).replace("-", "m"), str(z).replace("-", "m")]
					scene.name = temp
					add_child(scene)
					scene.global_position = Vector3(
						x * EmptyParcel.PARCEL_SIZE + EmptyParcel.PARCEL_HALF_SIZE,
						0,
						-z * EmptyParcel.PARCEL_SIZE - EmptyParcel.PARCEL_HALF_SIZE
					)

					var config = _calculate_parcel_adjacency(
						x,
						z,
						min_x - padding,
						max_x + padding,
						min_z - padding,
						max_z + padding,
						scene_parcel_set
					)
					scene.set_corner_configuration.call_deferred(config)

					loaded_empty_scenes[parcel_string] = scene

	if wall_manager:
		wall_manager.create_walls_for_bounds(min_x, max_x, min_z, max_z, padding)


func _calculate_parcel_adjacency(
	x: int,
	z: int,
	bounds_min_x: int,
	bounds_max_x: int,
	bounds_min_z: int,
	bounds_max_z: int,
	loaded_parcels: Dictionary
) -> CornerConfiguration:
	var config = CornerConfiguration.new()

	# Helper to determine the state of an adjacent parcel
	var get_parcel_state = func(px: int, pz: int) -> CornerConfiguration.ParcelState:
		# Out of bounds
		if px < bounds_min_x or px > bounds_max_x or pz < bounds_min_z or pz > bounds_max_z:
			return CornerConfiguration.ParcelState.NOTHING
		# Loaded scene
		if loaded_parcels.has(Vector2i(px, pz)):
			return CornerConfiguration.ParcelState.LOADED
		# Empty parcel
		return CornerConfiguration.ParcelState.EMPTY

	# Check all 8 adjacent positions
	config.north = get_parcel_state.call(x, z + 1)
	config.south = get_parcel_state.call(x, z - 1)
	config.east = get_parcel_state.call(x + 1, z)
	config.west = get_parcel_state.call(x - 1, z)
	config.northwest = get_parcel_state.call(x - 1, z + 1)
	config.northeast = get_parcel_state.call(x + 1, z + 1)
	config.southwest = get_parcel_state.call(x - 1, z - 1)
	config.southeast = get_parcel_state.call(x + 1, z - 1)

	return config
