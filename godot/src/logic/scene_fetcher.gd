extends Node

class_name SceneFetcher

signal parcels_processed(parcel_filled, empty)

const MAIN_JS_FILE_REQUEST = 100
const MAIN_CRDT_FILE_REQUEST = 101
const ADAPTATION_LAYER_JS_FILE_REQUEST = 102

var adaptation_layer_js_request: int = -1
var adaptation_layer_js_local_path: String = "user://sdk-adaptation-layer.js"

var http_requester: RustHttpRequesterWrapper = RustHttpRequesterWrapper.new()

var current_position: Vector2i = Vector2i(-1000, -1000)
var loaded_scenes: Dictionary = {}
var loaded_empty_scenes: Dictionary = {}
var scene_entity_coordinator: SceneEntityCoordinator = SceneEntityCoordinator.new()
var last_version_updated: int = -1


func _ready():
	Global.realm.realm_changed.connect(self._on_realm_changed)
	http_requester.request_completed.connect(self._on_requested_completed)

	scene_entity_coordinator.set_scene_radius(Global.config.scene_radius)
	Global.config.param_changed.connect(self._on_config_changed)

	if FileAccess.file_exists(adaptation_layer_js_local_path):
		DirAccess.remove_absolute(adaptation_layer_js_local_path)


func _on_config_changed(param: ConfigData.ConfigParams):
	if param == ConfigData.ConfigParams.SceneRadius:
		scene_entity_coordinator.set_scene_radius(Global.config.scene_radius)


func get_current_scene_data() -> Dictionary:
	var scene_entity_id = scene_entity_coordinator.get_scene_entity_id(current_position)
	if scene_entity_id == "empty":
		return {}
	else:
		var scene = loaded_scenes.get(scene_entity_id, {})
		return scene


func set_scene_radius(value: int):
	scene_entity_coordinator.set_scene_radius(value)


func _process(_dt):
	http_requester.poll()
	scene_entity_coordinator.update()
	if scene_entity_coordinator.get_version() != last_version_updated:
		_on_desired_scene_changed()
		last_version_updated = scene_entity_coordinator.get_version()


var empty_scenes = [
	preload("res://assets/empty-scenes/EP_01.glb"),
	preload("res://assets/empty-scenes/EP_02.glb"),
	preload("res://assets/empty-scenes/EP_03.glb"),
	preload("res://assets/empty-scenes/EP_04.glb"),
	preload("res://assets/empty-scenes/EP_05.glb"),
	preload("res://assets/empty-scenes/EP_06.glb"),
	preload("res://assets/empty-scenes/EP_07.glb"),
	preload("res://assets/empty-scenes/EP_08.glb"),
	preload("res://assets/empty-scenes/EP_09.glb"),
	preload("res://assets/empty-scenes/EP_10.glb"),
	preload("res://assets/empty-scenes/EP_11.glb"),
	preload("res://assets/empty-scenes/EP_12.glb")
]


func _on_desired_scene_changed():
	var d = scene_entity_coordinator.get_desired_scenes()
	var loadable_scenes = d.get("loadable_scenes", [])
	var keep_alive_scenes = d.get("keep_alive_scenes", [])
	var empty_parcels = d.get("empty_parcels", [])
	for scene_id in loadable_scenes:
		if not loaded_scenes.has(scene_id):
			var dict = scene_entity_coordinator.get_scene_dict(scene_id)
			if dict.size() > 0:
				dict["metadata"] = JSON.parse_string(dict.metadata)
				load_scene(scene_id, dict)
			else:
				printerr("shoud load scene_id ", scene_id, " but data is empty")

	var to_remove: Array[String] = []
	for scene_id in loaded_scenes.keys():
		if not loadable_scenes.has(scene_id) and not keep_alive_scenes.has(scene_id):
			var scene = loaded_scenes[scene_id]
			var scene_number_id: int = scene.get("scene_number_id", -1)
			if scene_number_id != -1:
				Global.scene_runner.kill_scene(scene_number_id)
				to_remove.push_back(scene_id)

	for scene_id in to_remove:
		loaded_scenes.erase(scene_id)

	var empty_parcels_coords = []
	for parcel in empty_parcels:
		var coord = parcel.split(",")
		var x = int(coord[0])
		var z = int(coord[1])
		empty_parcels_coords.push_back(Vector2i(x, z))

		if not loaded_empty_scenes.has(parcel):
			var index = randi_range(0, 11)
			var scene: Node3D = empty_scenes[index].instantiate()
			Global.content_manager._hide_colliders(scene)
			add_child(scene)
			scene.global_position = Vector3(x * 16 + 8, 0, -z * 16 - 8)
			loaded_empty_scenes[parcel] = scene

	var parcel_filled = []
	for scene_id in loaded_scenes:
		parcel_filled.append_array(loaded_scenes[scene_id].parcels)

	parcels_processed.emit(parcel_filled, empty_parcels_coords)


func _on_realm_changed():
	var should_load_city_pointers = true
	var content_base_url = Global.realm.content_base_url

	if not Global.realm.realm_city_loader_content_base_url.is_empty():
		content_base_url = Global.realm.realm_city_loader_content_base_url

	if Global.realm.realm_scene_urns.size() > 0 and Global.realm.realm_city_loader_content_base_url.is_empty():
		should_load_city_pointers = false

	scene_entity_coordinator.config(
		content_base_url + "entities/active", content_base_url, should_load_city_pointers
	)
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)
	var scenes_urns: Array = Global.realm.realm_about.get("configurations", {}).get("scenesUrn", [])
	scene_entity_coordinator.set_fixed_desired_entities_urns(scenes_urns)
	scene_entity_coordinator.set_fixed_desired_entities_global_urns(["urn:decentraland:entity:bafkreigl5uuwemnv6xmatgd4z7kb4rxirdf7v6wzdze6a5fv4rk2vetlu4?=&baseUrl=https://sdilauro.github.io/dae-unit-tests/"])

	for scene in loaded_scenes.values():
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if scene_number_id != -1:
			Global.scene_runner.kill_scene(scene_number_id)

	for parcel in loaded_empty_scenes:
		remove_child(loaded_empty_scenes[parcel])

	loaded_empty_scenes.clear()

	loaded_scenes = {}


func _on_requested_completed(response: RequestResponse):
	match response.reference_id():
		MAIN_JS_FILE_REQUEST:
			_on_main_js_file_requested_completed(response)
		MAIN_CRDT_FILE_REQUEST:
			_on_main_crdt_file_requested_completed(response)
		ADAPTATION_LAYER_JS_FILE_REQUEST:
			_on_adaptation_layer_js_file_requested_completed(response)
		_:
			pass


func get_scene_by_req_id(request_id: int):
	for scene in loaded_scenes.values():
		var req = scene.get("req", {})
		if (
			req.get("js_request_id", -1) == request_id
			or req.get("crdt_request_id", -1) == request_id
		):
			return scene

	return null


func _on_main_crdt_file_requested_completed(response: RequestResponse):
	var scene = get_scene_by_req_id(response.id())

	# Probably the scene was unloaded
	if scene == null:
		return

	scene.req.crdt_request_completed = true
	if scene.req.js_request_completed and scene.req.crdt_request_completed:
		_on_try_spawn_scene(scene)


func _on_adaptation_layer_js_file_requested_completed(response: RequestResponse):
	var request_id = response.id()
	for scene in loaded_scenes.values():
		var req = scene.get("req", {})
		if req.get("js_request_id", -1) == request_id:
			scene.req.js_request_completed = true
			_on_try_spawn_scene(scene)


func _on_main_js_file_requested_completed(response: RequestResponse):
	var scene = get_scene_by_req_id(response.id())

	# Probably the scene was unloaded
	if scene == null:
		return

	scene.req.js_request_completed = true
	if scene.req.js_request_completed and scene.req.crdt_request_completed:
		_on_try_spawn_scene(scene)


func update_position(new_position: Vector2i) -> void:
	if current_position == new_position:
		return

	current_position = new_position
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)


func load_scene(scene_entity_id: String, entity: Dictionary):
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
		"is_global": is_global
	}

	var is_sdk7 = metadata.get("runtimeVersion", null) == "7"
	var main_js_request_id := -1
	var local_main_js_path = ""
	var js_request_completed = true

	if is_sdk7:
		var main_js_file_hash = entity.get("content", {}).get(metadata.get("main", ""), null)
		if main_js_file_hash == null:
			printerr("Scene ", scene_entity_id, " fail getting the main js file hash.")
			return false

		local_main_js_path = "user://content/" + main_js_file_hash
		if not FileAccess.file_exists(local_main_js_path) or main_js_file_hash.begins_with("b64"):
			js_request_completed = false
			var main_js_file_url: String = entity.baseUrl + main_js_file_hash
			main_js_request_id = http_requester._requester.request_file(
				MAIN_JS_FILE_REQUEST,
				main_js_file_url,
				local_main_js_path.replace("user:/", OS.get_user_data_dir())
			)
	else:
		local_main_js_path = String(adaptation_layer_js_local_path)
		if not FileAccess.file_exists(local_main_js_path):
			js_request_completed = false
			if adaptation_layer_js_request == -1:
				adaptation_layer_js_request = (
					http_requester
					. _requester
					. request_file(
						ADAPTATION_LAYER_JS_FILE_REQUEST,
						"https://renderer-artifacts.decentraland.org/sdk7-adaption-layer/main/index.min.js",
						local_main_js_path.replace("user:/", OS.get_user_data_dir())
					)
				)
			main_js_request_id = adaptation_layer_js_request

	var req = {
		"js_request_completed": js_request_completed,
		"js_request_id": main_js_request_id,
		"js_path": local_main_js_path,
		"crdt_request_completed": true,
		"crdt_request_id": -1,
		"crdt_path": "",
	}

	var main_crdt_file_hash = entity.get("content", {}).get("main.crdt", null)
	if main_crdt_file_hash != null:
		var local_main_crdt_path = "user://content/" + main_crdt_file_hash
		var main_crdt_file_url: String = entity.baseUrl + main_crdt_file_hash
		var main_crdt_request_id = http_requester._requester.request_file(
			MAIN_CRDT_FILE_REQUEST,
			main_crdt_file_url,
			local_main_crdt_path.replace("user:/", OS.get_user_data_dir())
		)
		req["crdt_request_completed"] = false
		req["crdt_request_id"] = main_crdt_request_id
		req["crdt_path"] = local_main_crdt_path

	loaded_scenes[scene_entity_id]["req"] = req

	if is_sdk7:
		if req.crdt_request_completed and req.js_request_completed:
			_on_try_spawn_scene(loaded_scenes[scene_entity_id])
	else:
		# SDK6 scenes don't have crdt file, and if they'd have, there is no mechanism to make a clean spawn of both
		if req.js_request_completed:
			_on_try_spawn_scene(loaded_scenes[scene_entity_id])


func _on_try_spawn_scene(scene):
	var local_main_js_path = scene.req.js_path
	var local_main_crdt_path = scene.req.crdt_path

	if not FileAccess.file_exists(local_main_js_path):
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

	var content_mapping: Dictionary = {
		"base_url": scene.entity.baseUrl, "content": scene.entity["content"]
	}

	for key in content_mapping.content.keys():
		var key_lower = key.to_lower()
		if not content_mapping.content.has(key_lower):
			content_mapping.content[key_lower] = content_mapping.content[key]
			content_mapping.content.erase(key)

	var scene_definition: Dictionary = {
		"base": Vector2i(base_parcel[0], base_parcel[1]),
		"is_global": scene.is_global,
		"path": local_main_js_path,
		"main_crdt_path": local_main_crdt_path,
		"visible": true,
		"parcels": scene.parcels,
		"title": title
	}

	var scene_number_id: int = Global.scene_runner.start_scene(scene_definition, content_mapping)
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
#			Global.content_manager.remove_file_hash(file_hash)
		

