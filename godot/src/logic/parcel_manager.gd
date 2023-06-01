extends Node

class_name ParcelManager

var scene_runner: SceneManager = null
var realm: Realm = null
var last_parcel: Vector2i = Vector2i(-1000,-1000)

var loaded_scenes: Dictionary = {}

const SCENE_RADIUS = 2

func update_position(new_position: Vector2i) -> void:
	if last_parcel == new_position or realm.content_base_url.is_empty():
		return
		
	last_parcel = new_position
	
	if realm.realm_desired_running_scenes.size() > 0:
		return
	
	# TODO: reserve (7*(SCENE_RADIUS²))
	var pointers: Array[String] = []
	for x in range(new_position.x - SCENE_RADIUS, new_position.x + SCENE_RADIUS + 1):
		for z in range(new_position.y - SCENE_RADIUS, new_position.y + SCENE_RADIUS + 1):
			pointers.push_back(str(x) + "," + str(z))
#	print(pointers)
	var entities = await realm.get_active_entities(pointers)
	if entities != null:
		for entity in entities:
			if not entity.has("baseUrl"):
				entity["baseUrl"] = realm.content_base_url + "contents/"
			if not entity.has("entityId"):
				entity["entityId"] = entity.get("id", "no-id")
				
			load_scene(entity)
		
func _ready():
	scene_runner = get_tree().root.get_node("scene_runner")
	realm = get_tree().root.get_node("realm")
	realm.realm_changed.connect(self._on_realm_changed)

	
func _on_realm_changed():
	print("realm changed ")
	
	for realm_scene in realm.realm_desired_running_scenes:
		load_scene(realm_scene)
		
func load_scene(entity: Dictionary) -> bool:
	var scene_entity_id: String = entity.get("entityId", "")

	if loaded_scenes.has(scene_entity_id):
		return true
	
	loaded_scenes[scene_entity_id] = {
		"entity": entity,
		"scene_number_id": -1
	}
	
	var scene_json 
	
	if entity.get("metadata") == null:
		var scene_entity_url: String = entity.get("baseUrl", "") + entity.get("entityId", "")
		scene_json = await realm.requester.do_request_json(scene_entity_url, HTTPClient.METHOD_GET)
	else:
		scene_json = entity
		
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
	if not FileAccess.file_exists(local_main_js_path):
		var main_js_file_url: String = entity.get("baseUrl", "") + main_js_file_hash
		var main_js_file = await realm.requester.do_request_file(main_js_file_url, main_js_file_hash)
		
		if main_js_file == null:
			printerr("Scene ", scene_entity_id, " fail getting the main js file.")
			return false
	
	var base_parcel = scene_json.get("metadata", {}).get("scene", {}).get("base", "0,0").split_floats(",")
	var offset: Vector3 = 16 * Vector3(base_parcel[0], 0, -base_parcel[1])
	var base_url = scene_json.get("baseUrl", "")
	var content_mapping = ContentMapping.new()
	content_mapping.set_content_mapping(file_content)
	content_mapping.set_base_url(base_url)
	var scene_number_id: int = scene_runner.start_scene(local_main_js_path, offset, content_mapping)
	loaded_scenes[scene_entity_id].scene_number_id = scene_number_id
	
	return true
