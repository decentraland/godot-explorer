extends Node

class_name ParcelManager

const MAIN_JS_FILE_REQUEST = 100
const MAIN_CRDT_FILE_REQUEST = 101

var scene_runner: SceneManager = null
var realm: Realm = null

var http_requester: RustHttpRequesterWrapper = RustHttpRequesterWrapper.new()

var current_position: Vector2i = Vector2i(-1000, -1000)
var loaded_scenes: Dictionary = {}
var scene_entity_coordinator: SceneEntityCoordinator = SceneEntityCoordinator.new()
var last_version_updated: int = -1


func _ready():
	scene_runner = get_tree().root.get_node("scene_runner")
	realm = get_tree().root.get_node("realm")

	realm.realm_changed.connect(self._on_realm_changed)
	http_requester.request_completed.connect(self._on_requested_completed)

	scene_entity_coordinator.set_scene_radius(1)


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
		_on_desired_parsel_manager_update()
		last_version_updated = scene_entity_coordinator.get_version()


func _on_desired_parsel_manager_update():
	var d = scene_entity_coordinator.get_desired_scenes()
	var loadable_scenes = d.get("loadable_scenes", [])
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
		if not loadable_scenes.has(scene_id):
			var scene = loaded_scenes[scene_id]
			var scene_number_id: int = scene.get("scene_number_id", -1)
			if scene_number_id != -1:
				scene_runner.kill_scene(scene_number_id)
				to_remove.push_back(scene_id)

	for scene_id in to_remove:
		loaded_scenes.erase(scene_id)


func _on_realm_changed():
	var should_load_city_pointers = true
	var content_base_url = realm.content_base_url

	if not realm.realm_city_loader_content_base_url.is_empty():
		content_base_url = realm.realm_city_loader_content_base_url

	if realm.realm_scene_urns.size() > 0 and realm.realm_city_loader_content_base_url.is_empty():
		should_load_city_pointers = false

	scene_entity_coordinator.config(
		content_base_url + "entities/active", content_base_url, should_load_city_pointers
	)
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)
	var scenes_urns: Array = realm.realm_about.get("configurations", {}).get("scenesUrn", [])
	scene_entity_coordinator.set_fixed_desired_entities_urns(scenes_urns)

	for scene in loaded_scenes.values():
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if scene_number_id != -1:
			scene_runner.kill_scene(scene_number_id)

	loaded_scenes = {}


func _on_requested_completed(response: RequestResponse):
	match response.reference_id():
		MAIN_JS_FILE_REQUEST:
			_on_main_js_file_requested_completed(response)
		MAIN_CRDT_FILE_REQUEST:
			_on_main_crdt_file_requested_completed(response)
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
	loaded_scenes[scene_entity_id] = {
		"id": scene_entity_id, "entity": entity, "scene_number_id": -1
	}

	var main_js_file_hash = entity.get("content", {}).get(
		entity.get("metadata", {}).get("main", ""), null
	)
	if main_js_file_hash == null:
		printerr("Scene ", scene_entity_id, " fail getting the main js file hash.")
		return false

	var local_main_js_path = "user://content/" + main_js_file_hash
	var main_js_file_url: String = entity.baseUrl + main_js_file_hash
	var main_js_request_id = http_requester._requester.request_file(
		MAIN_JS_FILE_REQUEST,
		main_js_file_url,
		local_main_js_path.replace("user:/", OS.get_user_data_dir())
	)

	var req = {
		"js_request_completed": false,
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
	var parcels_str = scene.entity.get("metadata", {}).get("scene", {}).get("parcels", [])
	var title = scene.entity.get("metadata", {}).get("display", {}).get("title", "No title")
	var parcels = []
	for parcel in parcels_str:
		var p = parcel.split_floats(",")
		parcels.push_back(Vector2i(int(p[0]), int(p[1])))

	var content_mapping: Dictionary = {
		"base_url": scene.entity.baseUrl, "content": scene.entity["content"]
	}

	var scene_definition: Dictionary = {
		"base": Vector2i(base_parcel[0], base_parcel[1]),
		"is_global": false,
		"path": local_main_js_path,
		"main_crdt_path": local_main_crdt_path,
		"visible": true,
		"parcels": parcels,
		"title": title
	}

	var scene_number_id: int = scene_runner.start_scene(scene_definition, content_mapping)
	scene.scene_number_id = scene_number_id

	return true


func reload_scene(scene_id: String) -> void:
	var scene = loaded_scenes.get(scene_id)
	if scene != null:
		var scene_number_id: int = scene.get("scene_number_id", -1)
		if scene_number_id != -1:
			scene_runner.kill_scene(scene_number_id)

		loaded_scenes.erase(scene_id)
		scene_entity_coordinator.reload_scene_data(scene_id)
