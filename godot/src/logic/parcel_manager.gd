extends Node

class_name ParcelManager

const MAIN_FILE_REQUEST = 100

var scene_runner: SceneManager = null
var realm: Realm = null

var http_requester: RustHttpRequesterWrapper = RustHttpRequesterWrapper.new()

var current_position: Vector2i = Vector2i(-1000,-1000)
var loaded_scenes: Dictionary = {}
var scene_entity_coordinator: SceneEntityCoordinator = SceneEntityCoordinator.new()
var last_version_updated: int = -1

func _ready():
	scene_runner = get_tree().root.get_node("scene_runner")
	realm = get_tree().root.get_node("realm")

	realm.realm_changed.connect(self._on_realm_changed)
	http_requester.request_completed.connect(self._on_requested_completed)
	
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
	
func _on_realm_changed():
	var should_load_city_pointers = true
	var content_base_url = realm.content_base_url

	if not realm.realm_city_loader_content_base_url.is_empty():
		content_base_url = realm.realm_city_loader_content_base_url

	if realm.realm_scene_urns.size() > 0 and realm.realm_city_loader_content_base_url.is_empty():
		should_load_city_pointers = false
		
	print(content_base_url , " & ", should_load_city_pointers)
	scene_entity_coordinator.config(content_base_url + "entities/active", content_base_url, should_load_city_pointers)
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
		MAIN_FILE_REQUEST:
			_on_main_file_requested_completed(response)
		_:
			pass
	
func _on_main_file_requested_completed(response: RequestResponse):
	var status_code = response.status_code()
	if response.is_error() or status_code < 200 or status_code > 299:
		return null
		
	var request_id = response.id()
	for scene in loaded_scenes.values():
		if scene.get("main_file_request_id", "") == request_id:
			_on_try_spawn_scene(scene)

func update_position(new_position: Vector2i) -> void:
	if current_position == new_position:
		return
		
	current_position = new_position
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)

func load_scene(scene_entity_id: String, entity: Dictionary):
	loaded_scenes[scene_entity_id] = {
		"entity": entity,
		"scene_number_id": -1
	}
	
	var main_crdt_file_hash = entity.get("content", {}).get("main.crdt", null)
	if main_crdt_file_hash != null:
		pass
		# TODO: load main.crdt
	
	var main_js_file_hash = entity.get("content", {}).get(entity.get("metadata", {}).get("main", ""), null)
	if main_js_file_hash == null or main_js_file_hash == "no_hash":
		printerr("Scene ", scene_entity_id, " fail getting the main js file hash.")
		return false
		
	var local_main_js_path = "user://content/" + main_js_file_hash
	var main_js_file_url: String = entity.baseUrl + main_js_file_hash
	var request_id = http_requester._requester.request_file(MAIN_FILE_REQUEST, main_js_file_url, local_main_js_path.replace("user:/", OS.get_user_data_dir()))
	loaded_scenes[scene_entity_id]["main_file_request_id"] = request_id
	loaded_scenes[scene_entity_id]["local_main_js_path"] = local_main_js_path

func _on_try_spawn_scene(scene):
	var local_main_js_path = scene["local_main_js_path"]
	
	if not FileAccess.file_exists(local_main_js_path):
		return false

	var base_parcel = scene.entity.get("metadata", {}).get("scene", {}).get("base", "0,0").split_floats(",")
	var offset: Vector3 = 16 * Vector3(base_parcel[0], 0, -base_parcel[1])
	var base_url = scene.entity.get("baseUrl", "")
	var content_mapping = ContentMapping.new()
	content_mapping.set_content_mapping(scene.entity["content"])
	content_mapping.set_base_url(scene.entity.baseUrl)
	var scene_number_id: int = scene_runner.start_scene(local_main_js_path, offset, content_mapping)
	scene.scene_number_id = scene_number_id
	
	return true
