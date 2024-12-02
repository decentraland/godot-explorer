class_name SceneFetcher
extends Node

signal parcels_processed(parcel_filled, empty)
signal report_scene_load(done: bool, is_new_loading: bool, pending: int)
signal notify_pending_loading_scenes(is_pending: bool)

const EMPTY_SCENES = [
	preload("res://assets/empty-scenes/EP_0.tscn"),
	#preload("res://assets/empty-scenes/EP_1.tscn"), # it looks dark
	preload("res://assets/empty-scenes/EP_2.tscn"),
	preload("res://assets/empty-scenes/EP_3.tscn"),
	preload("res://assets/empty-scenes/EP_4.tscn"),
	preload("res://assets/empty-scenes/EP_5.tscn"),
	preload("res://assets/empty-scenes/EP_6.tscn"),
	preload("res://assets/empty-scenes/EP_7.tscn"),
	preload("res://assets/empty-scenes/EP_8.tscn"),
	preload("res://assets/empty-scenes/EP_9.tscn"),
	preload("res://assets/empty-scenes/EP_10.tscn"),
	preload("res://assets/empty-scenes/EP_11.tscn")
]

const ASSET_OPTIMIZED_BASE_URL: String = "https://psquad.kuruk.net/localstack/optimized-assets"
#const ASSET_OPTIMIZED_BASE_URL: String = "http://localhost:3232"

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
	var keep_alive_scenes = d.get("keep_alive_scenes", [])
	var empty_parcels = d.get("empty_parcels", [])

	var loading_promises: Array = []
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

	for scene_id in loaded_scenes.keys():
		if not loadable_scenes.has(scene_id) and not keep_alive_scenes.has(scene_id):
			var scene: SceneItem = loaded_scenes[scene_id]
			if scene.scene_number_id != -1:
				Global.scene_runner.kill_scene(scene.scene_number_id)

	var empty_parcels_coords = []
	for parcel in empty_parcels:
		var coord = parcel.split(",")
		var x = int(coord[0])
		var z = int(coord[1])
		empty_parcels_coords.push_back(Vector2i(x, z))

		if not loaded_empty_scenes.has(parcel):
			var index = randi_range(0, EMPTY_SCENES.size() - 1)
			var scene: Node3D = EMPTY_SCENES[index].instantiate()
			var temp := "EP_%s_%s_%s" % [index, str(x).replace("-", "m"), str(-z).replace("-", "m")]
			scene.name = temp
			add_child(scene)
			scene.global_position = Vector3(x * 16 + 8, 0, -z * 16 - 8)
			loaded_empty_scenes[parcel] = scene

	var parcel_filled = []
	for scene: SceneItem in loaded_scenes.values():
		parcel_filled.append_array(scene.parcels)

	report_scene_load.emit(true, new_loading, loadable_scenes.size())

	parcels_processed.emit(parcel_filled, empty_parcels_coords)


func _on_realm_changed():
	var should_load_city_pointers = true
	var content_base_url = Global.realm.content_base_url

	Global.get_config().last_realm_joined = Global.realm.realm_url
	Global.get_config().save_to_settings_file()

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

	var scene_hash_zip: String = "%s.zip" % scene_entity_id
	var asset_url: String = "%s/%s.zip" % [ASSET_OPTIMIZED_BASE_URL, scene_entity_id]
	var download_promise: Promise = Global.content_provider.fetch_file_by_url(
		scene_hash_zip, asset_url
	)
	var download_res = await PromiseUtils.async_awaiter(download_promise)
	if download_res is PromiseError:
		printerr("Scene ", scene_entity_id, " is not optimized, failed to download zip.")
	else:
		var ok = ProjectSettings.load_resource_pack("user://content/" + scene_hash_zip, false)
		if not ok:
			printerr("Scene ", scene_entity_id, " failed to load optimized scene")
		else:
			print("Scene ", scene_entity_id, " zip loaded successfully.")

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
