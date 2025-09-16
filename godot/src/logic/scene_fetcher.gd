class_name SceneFetcher
extends Node

signal parcels_processed(parcel_filled, empty)
signal report_scene_load(done: bool, is_new_loading: bool, pending: int)
signal notify_pending_loading_scenes(is_pending: bool)

const EMPTY_SCENES = [preload("res://assets/empty-scenes/EmptyScene.tscn")]

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

var desired_portable_experiences_urns: Array[String] = []

# Special-case: one-shot to skip loading screen
var _is_reloading: bool = false

# This counter is to control the async-flow
var _scene_changed_counter: int = 0

var _debugging_js_scene_id: String = ""

var _bypass_loading_check: bool = false


func _ready():
	Global.realm.realm_changed.connect(self._on_realm_changed)

	# Initialize wall manager
	wall_manager = FloatingIslandWalls.new()
	add_child(wall_manager)

	scene_entity_coordinator.set_scene_radius(Global.get_config().scene_radius)
	Global.get_config().param_changed.connect(self._on_config_changed)

	Global.scene_runner.scene_killed.connect(self.on_scene_killed)
	Global.loading_finished.connect(self.on_loading_finished)


func get_current_spawn_point():
	var current_scene_data = get_current_scene_data()
	if current_scene_data == null:
		return null

	return current_scene_data.scene_entity_definition.get_global_spawn_position()


func on_loading_finished():
	var target_position = get_current_spawn_point()
	if target_position != null:
		Global.get_explorer().move_to(target_position, true)


func on_scene_killed(killed_scene_id, _entity_id):
	for scene_entity_id in loaded_scenes.keys():
		var scene: SceneItem = loaded_scenes[scene_entity_id]
		if scene.scene_number_id == killed_scene_id:
			loaded_scenes.erase(scene_entity_id)
			return


func _on_config_changed(param: ConfigData.ConfigParams):
	if param == ConfigData.ConfigParams.SCENE_RADIUS:
		scene_entity_coordinator.set_scene_radius(Global.get_config().scene_radius)


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


# gdlint:ignore = async-function-name
func _process(_dt):
	scene_entity_coordinator.update()

	var version := scene_entity_coordinator.get_version()

	# When the loading-check is disable, early process this and return
	if not Global.get_config().loading_scene_arround_only_when_you_pass:
		if version != last_version_updated:
			last_version_updated = scene_entity_coordinator.get_version()
			await _async_on_desired_scene_changed()
		return

	# Once we're here, we need the logic of selected time to process the desired change
	var scene_entity_id := scene_entity_coordinator.get_scene_entity_id(current_position)

	if (
		_bypass_loading_check
		or scene_entity_id != current_scene_entity_id
		or scene_entity_id.is_empty()
	):
		current_scene_entity_id = scene_entity_id
		_bypass_loading_check = false

		if version != last_version_updated:
			last_version_updated = scene_entity_coordinator.get_version()
			notify_pending_loading_scenes.emit(false)
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
	var d = scene_entity_coordinator.get_desired_scenes()
	var loadable_scenes = d.get("loadable_scenes", [])
	var keep_alive_scenes = d.get("keep_alive_scenes", [])
	var empty_parcels = d.get("empty_parcels", [])

	_scene_changed_counter += 1
	var counter_this_call := _scene_changed_counter

	# Report new load, when I dont have scenes loaded, and there are a lot of new scenes...
	var new_loading = loaded_scenes.is_empty() and not loadable_scenes.is_empty()
	if new_loading and _is_reloading:
		_is_reloading = false
		new_loading = false

	var loading_promises: Array = []
	for scene_id in loadable_scenes:
		if not loaded_scenes.has(scene_id):
			var scene_definition = scene_entity_coordinator.get_scene_definition(scene_id)
			if scene_definition != null:
				loading_promises.push_back(async_load_scene.bind(scene_id, scene_definition))
			else:
				printerr("shoud load scene_id ", scene_id, " but data is empty")
		else:
			# When we already have loaded the scene...
			new_loading = false

	report_scene_load.emit(false, new_loading, loadable_scenes.size())

	await PromiseUtils.async_all(loading_promises)

	# If there is other calls processing the scene, early return
	# 	the next block of code will be executed by the last request
	if counter_this_call != _scene_changed_counter:
		return

	# Clean up old scenes that are no longer needed
	var scenes_to_remove = []
	for scene_id in loaded_scenes.keys():
		var should_keep = loadable_scenes.has(scene_id) or keep_alive_scenes.has(scene_id)

		if not should_keep:
			var scene: SceneItem = loaded_scenes[scene_id]
			# Don't kill global scenes
			if not scene.is_global and scene.scene_number_id != -1:
				print("Unloading scene: %s (was at parcels: %s)" % [scene.id, scene.parcels])
				Global.scene_runner.kill_scene(scene.scene_number_id)
				scenes_to_remove.append(scene_id)
			elif scene.is_global:
				print("Keeping global scene: %s" % scene.id)

	# Remove killed scenes from loaded_scenes dictionary
	for scene_id in scenes_to_remove:
		loaded_scenes.erase(scene_id)

	# Calculate bounds of all loaded scenes and create 2-parcel padding
	var all_scene_parcels = []
	for scene: SceneItem in loaded_scenes.values():
		if not scene.is_global:  # Only consider local scenes for bounds
			all_scene_parcels.append_array(scene.parcels)

	# Filter out distant parcels to prevent huge padding areas
	# This happens when there's a scene at (0,0) mixed with scenes far away
	if all_scene_parcels.size() > 1:
		all_scene_parcels = _filter_clustered_parcels(all_scene_parcels)

	# Only recreate floating island if scene parcels have changed
	var current_scene_hash = str(all_scene_parcels.hash())
	if (
		not all_scene_parcels.is_empty()
		and (not has_meta("last_scene_hash") or get_meta("last_scene_hash") != current_scene_hash)
	):
		print("Scene layout changed, recreating floating island...")
		print("Cleaning up %d old empty parcels" % loaded_empty_scenes.size())
		for parcel in loaded_empty_scenes:
			var empty_scene = loaded_empty_scenes[parcel]
			remove_child(empty_scene)
			empty_scene.queue_free()
		loaded_empty_scenes.clear()
		current_edge_parcels.clear()
		wall_manager.clear_walls()

		# Store the hash to avoid recreating unnecessarily
		set_meta("last_scene_hash", current_scene_hash)

		# Create floating island with empty parcels
		var empty_parcels_coords = []
		# Calculate bounding box of all loaded scene parcels
		var min_x = all_scene_parcels[0].x
		var max_x = all_scene_parcels[0].x
		var min_z = all_scene_parcels[0].y
		var max_z = all_scene_parcels[0].y

		for parcel in all_scene_parcels:
			min_x = min(min_x, parcel.x)
			max_x = max(max_x, parcel.x)
			min_z = min(min_z, parcel.y)
			max_z = max(max_z, parcel.y)

		# Create a set of scene parcels for quick lookup
		var scene_parcel_set = {}
		for parcel in all_scene_parcels:
			scene_parcel_set[Vector2i(parcel.x, parcel.y)] = true

		# Create 2-parcel padding around the bounds
		var padding = 2
		for x in range(min_x - padding, max_x + padding + 1):
			for z in range(min_z - padding, max_z + padding + 1):
				var coord = Vector2i(x, z)
				# Only add empty parcels if they're not occupied by actual scenes
				if not scene_parcel_set.has(coord):
					empty_parcels_coords.push_back(coord)
					var parcel_string = "%d,%d" % [x, z]

					if not loaded_empty_scenes.has(parcel_string):
						var index = randi_range(0, EMPTY_SCENES.size() - 1)
						var scene: Node3D = EMPTY_SCENES[index].instantiate()
						var temp := (
							"EP_%s_%s_%s"
							% [index, str(x).replace("-", "m"), str(z).replace("-", "m")]
						)
						scene.name = temp
						add_child(scene)
						scene.global_position = Vector3(x * 16 + 8, 0, -z * 16 - 8)

						# Set cliff direction based on position relative to scene bounds
						var cliff_direction = _calculate_cliff_direction(
							x, z, min_x, max_x, min_z, max_z, padding
						)
						if scene.has_method("set_cliff_direction"):
							scene.set_cliff_direction(cliff_direction)

						loaded_empty_scenes[parcel_string] = scene

		# Calculate edge parcels (outermost perimeter)
		var edge_parcels: Array[Vector2i] = []
		var padding_bounds_min_x = min_x - padding
		var padding_bounds_max_x = max_x + padding
		var padding_bounds_min_z = min_z - padding
		var padding_bounds_max_z = max_z + padding

		for coord in empty_parcels_coords:
			var x = coord.x
			var z = coord.y
			# A parcel is on the edge if it's on the boundary of the padded area
			if (
				x == padding_bounds_min_x
				or x == padding_bounds_max_x
				or z == padding_bounds_min_z
				or z == padding_bounds_max_z
			):
				edge_parcels.append(coord)

		# Store edge parcels for external access and add edge indicators
		current_edge_parcels = edge_parcels

		# Add white cube indicators to edge parcels
		for edge_coord in edge_parcels:
			var parcel_string = "%d,%d" % [edge_coord.x, edge_coord.y]
			if loaded_empty_scenes.has(parcel_string):
				var empty_scene = loaded_empty_scenes[parcel_string]
				if empty_scene.has_method("add_edge_indicator"):
					empty_scene.add_edge_indicator()

		print(
			(
				"Created floating island: bounds (%d,%d) to (%d,%d) with %d empty padding parcels, %d edge parcels"
				% [min_x, min_z, max_x, max_z, empty_parcels_coords.size(), edge_parcels.size()]
			)
		)
		print("Edge parcels: %s" % edge_parcels)

		# Create invisible walls around the floating island
		wall_manager.create_walls_for_bounds(min_x, max_x, min_z, max_z, padding)

	var empty_parcels_coords = []
	if has_meta("last_scene_hash"):
		# Use existing empty parcels for coordinate processing
		for parcel_string in loaded_empty_scenes.keys():
			var coord = parcel_string.split(",")
			var x = int(coord[0])
			var z = int(coord[1])
			empty_parcels_coords.push_back(Vector2i(x, z))

	# Process original empty parcels from coordinator (if any) - but only if we didn't just create floating island
	if not has_meta("last_scene_hash") or get_meta("last_scene_hash") == "":
		for parcel in empty_parcels:
			var coord = parcel.split(",")
			var x = int(coord[0])
			var z = int(coord[1])
			if not empty_parcels_coords.has(Vector2i(x, z)):  # Avoid duplicates
				empty_parcels_coords.push_back(Vector2i(x, z))

	var parcel_filled = []
	for scene: SceneItem in loaded_scenes.values():
		parcel_filled.append_array(scene.parcels)

		# Calculate and print grid size for each loaded scene
		var grid_size = _calculate_scene_grid_size(scene.parcels)
		var scene_type = "GLOBAL" if scene.is_global else "LOCAL"
		var current_pos = Global.scene_fetcher.current_position
		var contains_current = scene.parcels.has(current_pos)
		print(
			(
				"Scene '%s' (%s) loaded with grid size: %dx%d (total parcels: %d) - Contains current pos (%d,%d): %s"
				% [
					scene.id,
					scene_type,
					grid_size.x,
					grid_size.y,
					scene.parcels.size(),
					current_pos.x,
					current_pos.y,
					contains_current
				]
			)
		)

	report_scene_load.emit(true, new_loading, loadable_scenes.size())

	parcels_processed.emit(parcel_filled, empty_parcels_coords)


func _on_realm_changed():
	var should_load_city_pointers = true
	var content_base_url = Global.realm.content_base_url

	Global.get_config().last_realm_joined = Global.realm.realm_url
	Global.get_config().save_to_settings_file()

	# Force floating island recreation on realm change
	set_meta("last_scene_hash", "")

	if not Global.realm.realm_city_loader_content_base_url.is_empty():
		content_base_url = Global.realm.realm_city_loader_content_base_url

	if (
		Global.realm.realm_scene_urns.size() > 0
		and Global.realm.realm_city_loader_content_base_url.is_empty()
	):
		should_load_city_pointers = false

	scene_entity_coordinator.config(
		content_base_url + "entities/active", content_base_url, should_load_city_pointers
	)
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)
	var scenes_urns: Array = Global.realm.realm_about.get("configurations", {}).get("scenesUrn", [])
	scene_entity_coordinator.set_fixed_desired_entities_urns(scenes_urns)

	set_portable_experiences_urns(self.desired_portable_experiences_urns)

	for scene: SceneItem in loaded_scenes.values():
		if not scene.is_global and scene.scene_number_id != -1:
			Global.scene_runner.kill_scene(scene.scene_number_id)

	for parcel in loaded_empty_scenes:
		var empty_parcel = loaded_empty_scenes[parcel]
		remove_child(empty_parcel)
		empty_parcel.queue_free()

	loaded_empty_scenes.clear()
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


func update_position(new_position: Vector2i) -> void:
	if current_position == new_position:
		return

	current_position = new_position
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)


func async_load_scene(
	scene_entity_id: String, scene_entity_definition: DclSceneEntityDefinition
) -> Promise:
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

	var download_promise: Promise = Global.content_provider.fetch_file_by_url(
		scene_hash_zip, asset_url
	)
	var download_res = await PromiseUtils.async_awaiter(download_promise)
	if Global.is_xr() or Global.get_testing_scene_mode():
		print("Scene optimization skipped")
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
		printerr("the scene was removed while was loading ", scene_entity_id)
		return PromiseUtils.resolved(false)

	_on_try_spawn_scene(loaded_scenes[scene_entity_id], local_main_js_path, local_main_crdt_path)
	return PromiseUtils.resolved(true)


func _on_try_spawn_scene(
	scene_item: SceneItem, local_main_js_path: String, local_main_crdt_path: String
):
	if not local_main_js_path.is_empty() and not FileAccess.file_exists(local_main_js_path):
		printerr("Couldn't get main.js file")
		local_main_js_path = ""

	if not local_main_crdt_path.is_empty() and not FileAccess.file_exists(local_main_crdt_path):
		printerr("Couldn't get main.crdt file")
		local_main_crdt_path = ""

	if local_main_crdt_path.is_empty() and local_main_js_path.is_empty():
		printerr("Couldn't spawn the scene ", scene_item.id)
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

	return true


func reload_scene(scene_id: String) -> void:
	var scene = loaded_scenes.get(scene_id)
	if scene != null:
		var scene_number_id: int = scene.scene_number_id
		if scene_number_id != -1:
			Global.scene_runner.kill_scene(scene_number_id)

		var scene_entity_definition: DclSceneEntityDefinition = scene.scene_entity_definition
		var local_main_js_path: String = (
			"user://content/" + scene_entity_definition.get_main_js_hash()
		)
		if not local_main_js_path.is_empty() and FileAccess.file_exists(local_main_js_path):
			DirAccess.remove_absolute(local_main_js_path)

		loaded_scenes.erase(scene_id)
		scene_entity_coordinator.reload_scene_data(scene_id)
		_is_reloading = true

	# TODO: clean file hash cached
	# var dict = scene_entity_coordinator.get_scene_dict(scene_id)
	# if dict.size() > 0:
	# 	var content_dict: Dictionary = dict.get("content", {})
	# 	for file_hash in content_dict.values():
	# 		print("todo clean file hash ", file_hash)


func set_debugging_js_scene_id(id: String) -> void:
	_debugging_js_scene_id = id


func get_edge_parcels() -> Array[Vector2i]:
	return current_edge_parcels


func _filter_clustered_parcels(parcels: Array) -> Array:
	# Find the largest cluster of parcels to avoid huge padding from distant outliers
	if parcels.size() <= 2:
		return parcels

	# Calculate center point based on current player position
	var player_pos = current_position
	var filtered_parcels = []
	var max_distance_from_player = 10  # Only include parcels within 10 units of player

	for parcel in parcels:
		var distance = abs(parcel.x - player_pos.x) + abs(parcel.y - player_pos.y)
		if distance <= max_distance_from_player:
			filtered_parcels.append(parcel)

	# If filtering removed too many parcels, fall back to original
	if filtered_parcels.size() == 0:
		print("Warning: Filtering removed all parcels, using original set")
		return parcels

	print(
		(
			"Filtered parcels from %d to %d (removed distant outliers)"
			% [parcels.size(), filtered_parcels.size()]
		)
	)
	return filtered_parcels


func _calculate_cliff_direction(
	x: int,
	z: int,
	scene_min_x: int,
	scene_max_x: int,
	scene_min_z: int,
	scene_max_z: int,
	padding: int
):
	# Load the EmptyParcel class to access the enum
	var empty_parcel_script = preload("res://src/ui/components/empty_parcel.gd")
	var CliffDirection = empty_parcel_script.CliffDirection

	var padding_bounds_min_x = scene_min_x - padding
	var padding_bounds_max_x = scene_max_x + padding
	var padding_bounds_min_z = scene_min_z - padding
	var padding_bounds_max_z = scene_max_z + padding

	# Check if this parcel is on the edge boundary
	var is_on_west_edge = x == padding_bounds_min_x
	var is_on_east_edge = x == padding_bounds_max_x
	var is_on_north_edge = z == padding_bounds_min_z
	var is_on_south_edge = z == padding_bounds_max_z

	# Check for corner positions first (corners take priority over straight edges)
	if is_on_north_edge and is_on_west_edge:
		return CliffDirection.NORTHWEST
	elif is_on_north_edge and is_on_east_edge:
		return CliffDirection.NORTHEAST
	elif is_on_south_edge and is_on_west_edge:
		return CliffDirection.SOUTHWEST
	elif is_on_south_edge and is_on_east_edge:
		return CliffDirection.SOUTHEAST
	# Then check for straight edges
	elif is_on_west_edge:
		return CliffDirection.WEST
	elif is_on_east_edge:
		return CliffDirection.EAST
	elif is_on_north_edge:
		return CliffDirection.NORTH
	elif is_on_south_edge:
		return CliffDirection.SOUTH
	else:
		return CliffDirection.NONE


func _calculate_scene_grid_size(parcels: Array[Vector2i]) -> Vector2i:
	if parcels.is_empty():
		return Vector2i.ZERO

	if parcels.size() == 1:
		return Vector2i.ONE

	# For scenes with non-adjacent parcels, we can't represent as a simple grid
	# Instead, let's find the tightest grid that would contain all parcels
	var min_x = parcels[0].x
	var max_x = parcels[0].x
	var min_y = parcels[0].y
	var max_y = parcels[0].y

	for parcel in parcels:
		min_x = min(min_x, parcel.x)
		max_x = max(max_x, parcel.x)
		min_y = min(min_y, parcel.y)
		max_y = max(max_y, parcel.y)

	# If parcels are spread far apart, this indicates scattered parcels
	var width = max_x - min_x + 1
	var height = max_y - min_y + 1

	# Check if parcels form a contiguous block or are scattered
	var expected_parcels = width * height
	if parcels.size() == expected_parcels:
		# Contiguous block
		return Vector2i(width, height)
	else:
		# Scattered parcels - return the number of parcels as "1x{count}" or "{count}x1"
		if parcels.size() <= 10:
			return Vector2i(parcels.size(), 1)  # Display as 1x{count} for small scattered scenes
		else:
			return Vector2i(
				int(sqrt(parcels.size())), int(ceil(float(parcels.size()) / sqrt(parcels.size())))
			)

	return Vector2i(width, height)
