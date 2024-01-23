class_name SceneFetcher
extends Node

signal parcels_processed(parcel_filled, empty)
signal report_scene_load(done: bool, is_new_loading: bool)

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

var adaptation_layer_js_request: int = -1
var adaptation_layer_js_local_path: String = "user://sdk-adaptation-layer.js"

var current_position: Vector2i = Vector2i(-1000, -1000)
var loaded_empty_scenes: Dictionary = {}
var loaded_scenes: Dictionary = {}
var scene_entity_coordinator: SceneEntityCoordinator = SceneEntityCoordinator.new()
var last_version_updated: int = -1

var desired_portable_experiences_urns: Array[String] = []


func _ready():
	Global.realm.realm_changed.connect(self._on_realm_changed)

	scene_entity_coordinator.set_scene_radius(Global.config.scene_radius)
	Global.config.param_changed.connect(self._on_config_changed)

	if FileAccess.file_exists(adaptation_layer_js_local_path):
		DirAccess.remove_absolute(adaptation_layer_js_local_path)

	Global.scene_runner.scene_killed.connect(self.on_scene_killed)


func on_scene_killed(killed_scene_id, _entity_id):
	for scene_id in loaded_scenes.keys():
		var scene = loaded_scenes[scene_id]
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if scene_number_id == killed_scene_id:
			loaded_scenes.erase(scene_id)
			return


func _on_config_changed(param: ConfigData.ConfigParams):
	if param == ConfigData.ConfigParams.SCENE_RADIUS:
		scene_entity_coordinator.set_scene_radius(Global.config.scene_radius)


func get_current_scene_data() -> Dictionary:
	var scene_entity_id = scene_entity_coordinator.get_scene_entity_id(current_position)
	if scene_entity_id == "empty":
		return {}

	var scene = loaded_scenes.get(scene_entity_id, {})
	return scene


func set_scene_radius(value: int):
	scene_entity_coordinator.set_scene_radius(value)


# gdlint:ignore = async-function-name
func _process(_dt):
	scene_entity_coordinator.update()
	if scene_entity_coordinator.get_version() != last_version_updated:
		last_version_updated = scene_entity_coordinator.get_version()
		await _async_on_desired_scene_changed()


func is_scene_loaded(x: int, z: int) -> bool:
	var parcel_str = "%d,%d" % [x, z]
	return get_parcel_scene_id(x, z) != -1 or loaded_empty_scenes.has(parcel_str)


func get_parcel_scene_id(x: int, z: int) -> int:
	for scene_id in loaded_scenes.keys():
		var scene = loaded_scenes[scene_id]
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if scene_number_id != -1:
			for pos in scene.get("parcels", []):
				if pos.x == x and pos.y == z:
					return scene_number_id
	return -1


func _async_on_desired_scene_changed():
	var d = scene_entity_coordinator.get_desired_scenes()
	var loadable_scenes = d.get("loadable_scenes", [])
	var keep_alive_scenes = d.get("keep_alive_scenes", [])
	var empty_parcels = d.get("empty_parcels", [])

	# Report new load, when I dont have scenes loaded, and there are a lot of new scenes...
	var new_loading = loaded_scenes.is_empty() and not loadable_scenes.is_empty()

	var loading_promises: Array = []
	for scene_id in loadable_scenes:
		if not loaded_scenes.has(scene_id):
			var dict = scene_entity_coordinator.get_scene_dict(scene_id)
			if dict.size() > 0:
				dict["metadata"] = JSON.parse_string(dict.metadata)
				loading_promises.push_back(async_load_scene.bind(scene_id, dict))
			else:
				printerr("shoud load scene_id ", scene_id, " but data is empty")
		else:
			# When we already have loaded the scene...
			new_loading = false

	report_scene_load.emit(false, new_loading)

	await PromiseUtils.async_all(loading_promises)

	for scene_id in loaded_scenes.keys():
		if not loadable_scenes.has(scene_id) and not keep_alive_scenes.has(scene_id):
			var scene = loaded_scenes[scene_id]
			var scene_number_id: int = scene.get("scene_number_id", -1)
			if scene_number_id != -1:
				Global.scene_runner.kill_scene(scene_number_id)

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
	for scene_id in loaded_scenes:
		parcel_filled.append_array(loaded_scenes[scene_id].parcels)

	report_scene_load.emit(true, new_loading)

	parcels_processed.emit(parcel_filled, empty_parcels_coords)


func _on_realm_changed():
	var should_load_city_pointers = true
	var content_base_url = Global.realm.content_base_url

	Global.config.last_realm_joined = Global.realm.realm_url
	Global.config.save_to_settings_file()

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

	for scene in loaded_scenes.values():
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if not scene.is_global and scene_number_id != -1:
			Global.scene_runner.kill_scene(scene_number_id)

	for parcel in loaded_empty_scenes:
		remove_child(loaded_empty_scenes[parcel])

	loaded_empty_scenes.clear()

	loaded_scenes = {}


func set_portable_experiences_urns(urns: Array[String]) -> void:
	var global_scenes_urns: Array = (
		Global.realm.realm_about.get("configurations", {}).get("globalScenesUrn", []).duplicate()
	)
	prints("set_portable_experiences_urns ", global_scenes_urns, " with ", urns)

	desired_portable_experiences_urns = urns
	global_scenes_urns.append_array(desired_portable_experiences_urns)
	scene_entity_coordinator.set_fixed_desired_entities_global_urns(global_scenes_urns)


func get_scene_by_req_id(request_id: int):
	for scene in loaded_scenes.values():
		var req = scene.get("req", {})
		if (
			req.get("js_request_id", -1) == request_id
			or req.get("crdt_request_id", -1) == request_id
		):
			return scene

	return null


func update_position(new_position: Vector2i) -> void:
	if current_position == new_position:
		return

	current_position = new_position
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)


func async_load_scene(scene_entity_id: String, entity: Dictionary) -> Promise:
	var metadata = entity.get("metadata", {})
	var is_global = entity.get("is_global", false)

	var parcels_str = metadata.get("scene", {}).get("parcels", [])
	var parcels = []
	for parcel in parcels_str:
		var p = parcel.split_floats(",")
		parcels.push_back(Vector2i(int(p[0]), int(p[1])))

	loaded_scenes[scene_entity_id] = {
		"id": scene_entity_id,
		"entity": entity,
		"scene_number_id": -1,
		"parcels": parcels,
		"is_global": is_global,
	}

	var is_sdk7 = metadata.get("runtimeVersion", null) == "7"
	var local_main_js_path = ""

	if is_sdk7:
		var main_js_file_hash = entity.get("content", {}).get(metadata.get("main", ""), null)
		if main_js_file_hash != null:
			local_main_js_path = "user://content/" + main_js_file_hash
			if (
				not FileAccess.file_exists(local_main_js_path)
				or main_js_file_hash.begins_with("b64")
			):
				var main_js_file_url: String = entity.baseUrl + main_js_file_hash
				var promise: Promise = Global.http_requester.request_file(
					main_js_file_url, local_main_js_path.replace("user:/", OS.get_user_data_dir())
				)

				var res = await PromiseUtils.async_awaiter(promise)
				if res is PromiseError:
					printerr(
						"Scene ",
						scene_entity_id,
						" fail getting the script code content, error message: ",
						res.get_error()
					)
					return PromiseUtils.resolved(false)
	else:
		local_main_js_path = String(adaptation_layer_js_local_path)
		if not FileAccess.file_exists(local_main_js_path):
			var promise: Promise = Global.http_requester.request_file(
				"https://renderer-artifacts.decentraland.org/sdk7-adaption-layer/dev/index.min.js",
				local_main_js_path.replace("user:/", OS.get_user_data_dir())
			)
			var res = await PromiseUtils.async_awaiter(promise)
			if res is PromiseError:
				printerr(
					"Scene ",
					scene_entity_id,
					" fail getting the adaptation layer content, error message: ",
					res.get_error()
				)
				return PromiseUtils.resolved(false)

	var main_crdt_file_hash = entity.get("content", {}).get("main.crdt", null)
	var local_main_crdt_path = ""
	if main_crdt_file_hash != null:
		local_main_crdt_path = "user://content/" + main_crdt_file_hash
		var main_crdt_file_url: String = entity.baseUrl + main_crdt_file_hash
		var promise: Promise = Global.http_requester.request_file(
			main_crdt_file_url, local_main_crdt_path.replace("user:/", OS.get_user_data_dir())
		)

		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			printerr(
				"Scene ",
				scene_entity_id,
				" fail getting the main crdt content, error message: ",
				res.get_error()
			)
			return PromiseUtils.resolved(false)

	# the scene was removed while it was loading...
	if not loaded_scenes.has(scene_entity_id):
		return PromiseUtils.resolved(false)

	_on_try_spawn_scene(loaded_scenes[scene_entity_id], local_main_js_path, local_main_crdt_path)
	return PromiseUtils.resolved(true)


func _on_try_spawn_scene(scene, local_main_js_path, local_main_crdt_path):
	if not local_main_js_path.is_empty() and not FileAccess.file_exists(local_main_js_path):
		printerr("Couldn't get main.js file")
		local_main_js_path = ""

	if not local_main_crdt_path.is_empty() and not FileAccess.file_exists(local_main_crdt_path):
		printerr("Couldn't get main.crdt file")
		local_main_crdt_path = ""

	if local_main_crdt_path.is_empty() and local_main_js_path.is_empty():
		printerr("Couldn't spawn the scene ", scene.id)
		return false

	var base_parcel = (
		scene.entity.get("metadata", {}).get("scene", {}).get("base", "0,0").split_floats(",")
	)
	var title = scene.entity.get("metadata", {}).get("display", {}).get("title", "No title")
	var scene_definition: Dictionary = {
		"base": Vector2i(base_parcel[0], base_parcel[1]),
		"is_global": scene.is_global,
		"path": local_main_js_path,
		"main_crdt_path": local_main_crdt_path,
		"visible": true,
		"parcels": scene.parcels,
		"title": title,
		"entity_id": scene.id,
		"metadata": scene.entity.metadata
	}

	var dcl_content_mapping = DclContentMappingAndUrl.new()
	dcl_content_mapping.initialize(scene.entity.baseUrl, scene.entity["content"])
	var scene_number_id: int = Global.scene_runner.start_scene(
		scene_definition, dcl_content_mapping
	)
	scene.scene_number_id = scene_number_id

	return true


func reload_scene(scene_id: String) -> void:
	var scene = loaded_scenes.get(scene_id)
	if scene != null:
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if scene_number_id != -1:
			Global.scene_runner.kill_scene(scene_number_id)

		loaded_scenes.erase(scene_id)
		scene_entity_coordinator.reload_scene_data(scene_id)

	var dict = scene_entity_coordinator.get_scene_dict(scene_id)
	if dict.size() > 0:
		var content_dict: Dictionary = dict.get("content", {})
		for file_hash in content_dict.values():
			print("todo clean file hash ", file_hash)
			# TODO: clean file hash cached
