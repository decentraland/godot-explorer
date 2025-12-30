class_name SceneFetcher
extends Node

signal parcels_processed(parcel_filled, empty)
signal report_scene_load(done: bool, is_new_loading: bool, pending: int)
signal notify_pending_loading_scenes(is_pending: bool)
signal player_parcel_changed(new_position: Vector2i)

const EMPTY_SCENE = preload("res://assets/empty-scenes/empty_parcel.tscn")

const ADAPTATION_LAYER_URL: String = "https://renderer-artifacts.decentraland.org/sdk6-adaption-layer/main/index.min.js"
const FIXED_LOCAL_ADAPTATION_LAYER: String = ""


class SceneItem:
	extends RefCounted
	var main_js_req_id: int = -1
	var main_crdt_req_id: int = -1

	var id: String = ""
	var scene_entity_definition: DclSceneEntityDefinition = null
	var scene_number_id: int = -1
	var parcels: Array[Vector2i] = []
	var is_global: bool = false


var current_position: Vector2i = Vector2i(-1000, -1000)
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

# This counter is to control the async-flow
var _scene_changed_counter: int = 0

var _debugging_js_scene_id: String = ""

var _bypass_loading_check: bool = false

# Track the target parcel during teleport to ensure correct spawn point
var _teleport_target_parcel: Vector2i = Vector2i(-1000, -1000)


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
	var scene_radius = 5 if is_using_floating_islands() else 0
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
	if is_using_floating_islands() and _teleport_target_parcel != Vector2i(-1000, -1000):
		spawn_parcel = _teleport_target_parcel
		_teleport_target_parcel = Vector2i(-1000, -1000)

	var scene_data = get_scene_data(spawn_parcel)
	if scene_data != null:
		var target_position = scene_data.scene_entity_definition.get_global_spawn_position()
		if target_position != null:
			Global.get_explorer().move_to(target_position, true)


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


func is_dynamic_loading_mode() -> bool:
	return _use_dynamic_loading


# gdlint:ignore = async-function-name
func _process(_dt):
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
	var is_invalid_position = current_position == Vector2i(-1000, -1000)

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

	# Determine if we should show a loading screen
	# Show loading screen ONLY when:
	# - We have no scenes loaded AND there are scenes to load (initial load)
	# - We're in floating islands mode (not dynamic loading)
	# - We're NOT in dynamic loading mode
	# - We're NOT reloading (teleport, reload_scene, etc.)
	var new_loading = (
		loaded_scenes.is_empty()
		and not loadable_scenes.is_empty()
		and is_using_floating_islands()
		and not _use_dynamic_loading
	)

	# Skip loading screen for any reload scenario (teleports, scene reloads, etc.)
	if new_loading and _is_reloading:
		_is_reloading = false  # Reset the flag after use (one-shot)
		new_loading = false

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
				loading_promises.push_back(async_load_scene.bind(scene_id, scene_definition))
			else:
				printerr("should load scene_id ", scene_id, " but data is empty")

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

	if use_floating_islands:
		_regenerate_floating_islands()

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
	var global_scenes_urns: Array = Global.realm.realm_about.get("configurations", {}).get(
		"globalScenesUrn", []
	)

	scene_entity_coordinator.config(
		content_base_url + "entities/active", content_base_url, should_load_city_pointers
	)
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
	# Collect parcels from ALL loaded scenes (not just player's current scene)
	var all_scene_parcels = []
	for scene_id in loaded_scenes.keys():
		var scene: SceneItem = loaded_scenes[scene_id]

		# Include parcels from all loaded non-global scenes
		if not scene.is_global and scene.scene_number_id != -1:
			for parcel in scene.parcels:
				all_scene_parcels.append(parcel)

	if all_scene_parcels.is_empty():
		return

	var current_scene_group_hash: String = str(all_scene_parcels.hash())

	# Skip if same scene configuration
	if !last_scene_group_hash.is_empty() and last_scene_group_hash == current_scene_group_hash:
		return

	last_scene_group_hash = current_scene_group_hash

	# Clear old empty parcels
	for parcel in loaded_empty_scenes:
		var empty_scene = loaded_empty_scenes[parcel]
		remove_child(empty_scene)
		empty_scene.queue_free()
	loaded_empty_scenes.clear()
	current_edge_parcels.clear()
	if wall_manager:
		wall_manager.clear_walls()

	# Create floating island platform considering all loaded scenes
	_create_floating_island_for_cluster(all_scene_parcels)


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

		# Always skip loading screen for teleports (both same and different positions)
		# Teleports are user-initiated and should feel seamless
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

	var content_mapping := scene_entity_definition.get_content_mapping()

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
			return PromiseUtils.resolved(false)

	var scene_hash_zip: String = "%s-mobile.zip" % scene_entity_id
	var asset_url: String = (
		"%s/%s-mobile.zip" % [Global.content_provider.get_optimized_base_url(), scene_entity_id]
	)

	# Check if optimized zip already exists to avoid re-download hang
	var zip_file_path = "user://content/" + scene_hash_zip
	var download_res = null
	if FileAccess.file_exists(zip_file_path):
		download_res = true  # Pretend success since file exists
	else:
		var download_promise: Promise = Global.content_provider.fetch_file_by_url(
			scene_hash_zip, asset_url
		)
		download_res = await PromiseUtils.async_awaiter(download_promise)

	if Global.is_xr() or Global.get_testing_scene_mode():
		pass  # Scene optimization skipped (XR/testing mode)
	elif download_res is PromiseError:
		printerr("Scene ", scene_entity_id, " is not optimized, failed to download zip.")
	else:
		var ok = ProjectSettings.load_resource_pack("user://content/" + scene_hash_zip, false)
		if not ok:
			printerr("Scene ", scene_entity_id, " failed to load optimized scene, error #1")
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

	# the scene was removed while it was loading...
	if not loaded_scenes.has(scene_entity_id):
		printerr("Scene was removed while loading:", scene_entity_id)
		return PromiseUtils.resolved(false)

	var scene_in_dict = loaded_scenes[scene_entity_id]
	_on_try_spawn_scene(scene_in_dict, local_main_js_path, local_main_crdt_path)
	return PromiseUtils.resolved(true)


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
					scene.global_position = Vector3(x * 16 + 8, 0, -z * 16 - 8)

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
