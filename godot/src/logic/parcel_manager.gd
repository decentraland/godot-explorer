extends Node

class_name ParcelManager

var scene_runner: SceneManager = null
var realm: Realm = null
var last_parcel: Vector2i = Vector2i(-1000,-1000)
var loaded_scenes: Dictionary = {}

const SCENE_RADIUS = 2

var desired_scene = []
var http_many_requester: HTTPManyRequester

const ACTIVE_ENTITIES_REQUEST = 1
const ENTITY_METADATA_REQUEST = 2
const MAIN_FILE_REQUEST = 3

func _ready():
	scene_runner = get_tree().root.get_node("scene_runner")
	realm = get_tree().root.get_node("realm")
	realm.realm_changed.connect(self._on_realm_changed)
	
	http_many_requester = HTTPManyRequester.new()
	http_many_requester.name = "http_many_requester_parcel"
	http_many_requester.request_completed.connect(self._on_requested_completed)
	add_child(http_many_requester)
	
func _on_requested_completed(reference_id: int, request_id: String, result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray):
	
	match reference_id:
		ACTIVE_ENTITIES_REQUEST:
			_on_active_entities_requested_completed(result, response_code, headers, body)
		ENTITY_METADATA_REQUEST:
			_on_entity_metadata_requested_completed(request_id, result, response_code, headers, body)
		MAIN_FILE_REQUEST:
			_on_main_file_requested_completed(request_id, result, response_code, headers, body)
		_:
			pass
			
func _on_active_entities_requested_completed(result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray):
	if result != OK or response_code < 200 or response_code > 299:
		return
		
	var json_str: String = body.get_string_from_utf8()
	if json_str.is_empty():
		return
	
	var json = JSON.parse_string(json_str)
	if json == null:
		printerr("do_request_json failed because json_string is not a valid json with length ", json_str.length())
		return

	for entity in json:
		if not entity.has("baseUrl"):
			entity["baseUrl"] = realm.content_base_url + "contents/"
		if not entity.has("entityId"):
			entity["entityId"] = entity.get("id", "no-id")
			
		load_scene(entity)
	
func _on_entity_metadata_requested_completed(request_id: String, result: int, response_code: int, _headers: PackedStringArray, body: PackedByteArray):
	if result != OK or response_code < 200 or response_code > 299:
		return
		
	var json_str: String = body.get_string_from_utf8()
	if json_str.is_empty():
		return
	
	var json = JSON.parse_string(json_str)
	if json == null:
		printerr("do_request_json failed because json_string is not a valid json with length ", json_str.length())
		return

	for scene in loaded_scenes:
		if scene.get("entity_metadata_request_id", "") == request_id:
			_on_load_scene_entity(json)
	
func request_active_entities(pointers: Array):
	if realm.realm_about == null:
		return
		
	var body_json = JSON.stringify({"pointers": pointers})
	http_many_requester.request(ACTIVE_ENTITIES_REQUEST, realm.content_base_url + "entities/active",HTTPClient.METHOD_POST, body_json, ["Content-type: application/json"])

func update_position(new_position: Vector2i) -> void:
	if last_parcel == new_position or realm.content_base_url.is_empty():
		return
		
	last_parcel = new_position
	
	if realm.realm_desired_running_scenes.size() > 0:
		return
	
	var pointers: Array[String] = []
	for x in range(new_position.x - SCENE_RADIUS, new_position.x + SCENE_RADIUS + 1):
		for z in range(new_position.y - SCENE_RADIUS, new_position.y + SCENE_RADIUS + 1):
			pointers.push_back(str(x) + "," + str(z))
	request_active_entities(pointers)

func _on_realm_changed():
	print("realm changed ")
	
	for realm_scene in realm.realm_desired_running_scenes:
		load_scene(realm_scene)
		
func load_scene(entity: Dictionary):
	var scene_entity_id: String = entity.get("entityId", "")

	if loaded_scenes.has(scene_entity_id):
		return true
	
	loaded_scenes[scene_entity_id] = {
		"entity": entity,
		"scene_number_id": -1
	}
	
	if entity.get("metadata") == null:
		var scene_entity_url: String = entity.get("baseUrl", "") + entity.get("entityId", "")
		var entity_metadata_request_id = http_many_requester.request(ENTITY_METADATA_REQUEST, scene_entity_url)
		loaded_scenes[scene_entity_id]["entity_metadata_request_id"] = entity_metadata_request_id
	else:
		_on_load_scene_entity(entity)
	
func _on_load_scene_entity(scene_json: Dictionary):
	var scene_entity_id: String = scene_json.get("entityId", "")
		
	if scene_json == null: 
		printerr("Scene ", scene_entity_id, " fail getting the entity.")
		return false
	
	var file_content: Dictionary = {}
	for file_hash in scene_json.get("content", []):
		file_content[file_hash.get("file", "null")] = file_hash.get("hash", "no_hash")
		
	var main_js_file_hash = file_content.get(scene_json.get("metadata", {}).get("main", ""), null)
	if main_js_file_hash == null or main_js_file_hash == "no_hash":
		printerr("Scene ", scene_entity_id, " fail getting the main js file hash.")
		return false
		
	var local_main_js_path = "user://content/" + main_js_file_hash
	var main_js_file_url: String = scene_json.get("baseUrl", "") + main_js_file_hash
	var request_id = http_many_requester.request(MAIN_FILE_REQUEST, main_js_file_url, HTTPClient.METHOD_GET, "", [], 0, local_main_js_path)
	loaded_scenes[scene_entity_id]["main_file_request_id"] = request_id
	loaded_scenes[scene_entity_id]["local_main_js_path"] = local_main_js_path
	loaded_scenes[scene_entity_id]["scene_json"] = scene_json
	loaded_scenes[scene_entity_id]["file_content"] = file_content
	
	
func _on_main_file_requested_completed(request_id: String, result: int, response_code: int, _headers: PackedStringArray, _body: PackedByteArray):
	if result != OK or response_code < 200 or response_code > 299:
		return
	
	for scene in loaded_scenes.values():
		if scene.get("main_file_request_id", "") == request_id:
			_on_try_spawn_scene(scene)
	
func _on_try_spawn_scene(scene):
	var scene_json = scene["scene_json"]
	var local_main_js_path = scene["local_main_js_path"]
	
	if not FileAccess.file_exists(local_main_js_path):
		return false

	var base_parcel = scene_json.get("metadata", {}).get("scene", {}).get("base", "0,0").split_floats(",")
	var offset: Vector3 = 16 * Vector3(base_parcel[0], 0, -base_parcel[1])
	var base_url = scene_json.get("baseUrl", "")
	var content_mapping = ContentMapping.new()
	content_mapping.set_content_mapping(scene["file_content"])
	content_mapping.set_base_url(base_url)
	var scene_number_id: int = scene_runner.start_scene(local_main_js_path, offset, content_mapping)
	scene.scene_number_id = scene_number_id

	return true