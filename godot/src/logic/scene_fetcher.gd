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

class SceneItem extends RefCounted:
	var main_js_req_id: int = -1
	var main_crdt_req_id: int = -1
	
	var id: String = ""
	var scene_entity_definition: DclSceneEntityDefinition = null
	var scene_number_id: int = -1
	var parcels: Array[Vector2i] = []
	var is_global: bool = false

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
	Global.loading_finished.connect(self.on_loading_finished)


func get_target_position(target_position: Variant) -> float:
	if target_position is Array:
		if target_position.is_empty():
			return 0.0

		if target_position.size() == 1:
			return float(target_position[0])

		# size >= 2
		return randf_range(target_position[0], target_position[1])
	return float(target_position)


func get_current_spawn_point():
	var current_scene_data = get_current_scene_data()
	#if current_scene_data.is_empty():
		#return null
	
	return null
	
	# TODO
	#var spawn_points = current_scene_data.get("entity", {}).get("metadata", {}).get(
		#"spawnPoints", []
	#)
	#if not spawn_points is Array:
		#return null
#
	#var some_spawn_point = null
	#for spawn_point in spawn_points:
		#if not spawn_point is Dictionary:
			#continue
#
		#if some_spawn_point == null:
			#some_spawn_point = spawn_point
#
		#if spawn_point.get("default", false):
			#some_spawn_point = spawn_point
#
	#if some_spawn_point == null:
		#return null
#
	#var target_position = some_spawn_point.get("position")
	## TODO Camera target
	## var target_camera_position = some_spawn_point.get("cameraTarget")
#
	#if not target_position is Dictionary:
		#return null
#
	#var target_position_x = get_target_position(target_position.get("x"))
	#var target_position_y = get_target_position(target_position.get("y"))
	#var target_position_z = get_target_position(target_position.get("z"))
#
	#var base_parcel = (
		#current_scene_data
		#. entity
		#. get("metadata", {})
		#. get("scene", {})
		#. get("base", "0,0")
		#. split_floats(",")
	#)
	#target_position_x = base_parcel[0] * 16.0 + target_position_x
	#target_position_z = -(base_parcel[1] * 16.0 + target_position_z)
	#return Vector3(target_position_x, target_position_y, target_position_z)


func on_loading_finished():
	var target_position = get_current_spawn_point()
	if target_position != null:
		Global.get_explorer().move_to(target_position)


func on_scene_killed(killed_scene_id, _entity_id):
	for scene_id in loaded_scenes.keys():
		var scene: SceneItem = loaded_scenes[scene_id]
		if scene.scene_number_id == killed_scene_id:
			loaded_scenes.erase(scene_id)
			return


func _on_config_changed(param: ConfigData.ConfigParams):
	if param == ConfigData.ConfigParams.SCENE_RADIUS:
		scene_entity_coordinator.set_scene_radius(Global.config.scene_radius)


func get_current_scene_data() -> SceneItem:
	var scene_entity_id = scene_entity_coordinator.get_scene_entity_id(current_position)
	if scene_entity_id == "empty":
		return null

	return loaded_scenes.get(scene_entity_id)


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
		var scene: SceneItem = loaded_scenes[scene_id]
		if scene.scene_number_id != -1:
			for pos in scene.parcels:
				if pos.x == x and pos.y == z:
					return scene.scene_number_id
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
			var scene_definition = scene_entity_coordinator.get_scene_definition(scene_id)
			
			if scene_definition != null:
				# TODO: update metadata?
				# dict["metadata"] = JSON.parse_string(dict.metadata)
				loading_promises.push_back(async_load_scene.bind(scene_id, scene_definition))
			else:
				printerr("shoud load scene_id ", scene_id, " but data is empty")
		else:
			# When we already have loaded the scene...
			new_loading = false

	report_scene_load.emit(false, new_loading)

	await PromiseUtils.async_all(loading_promises)

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

	for scene: SceneItem in loaded_scenes.values():
		if not scene.is_global and scene.scene_number_id != -1:
			Global.scene_runner.kill_scene(scene.scene_number_id)

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
	for scene: SceneItem in loaded_scenes.values():
		if (
			scene.main_js_req_id == request_id or
			scene.main_crdt_req_id == request_id
		):
			return scene

	return null


func update_position(new_position: Vector2i) -> void:
	if current_position == new_position:
		return

	current_position = new_position
	scene_entity_coordinator.set_current_position(current_position.x, current_position.y)

	
func async_load_scene(scene_entity_id: String, scene_entity_definition: DclSceneEntityDefinition) -> Promise:
	var parcels := scene_entity_definition.get_parcels()

	var scene_item: SceneItem = SceneItem.new()
	scene_item.id = scene_entity_id
	scene_item.scene_entity_definition = scene_entity_definition
	scene_item.scene_number_id = -1
	scene_item.parcels = parcels
	scene_item.is_global = scene_entity_definition.is_global()
	
	loaded_scenes[scene_entity_id] = scene_item

	var local_main_js_path: String = ""

	if scene_entity_definition.is_sdk7():
		var main_js_file_hash := scene_entity_definition.get_main_js_hash()
		if not main_js_file_hash.is_empty():
			local_main_js_path = "user://content/" + main_js_file_hash
			if (
				not FileAccess.file_exists(local_main_js_path)
				or main_js_file_hash.begins_with("b64")
			):
				var main_js_file_url: String = scene_entity_definition.get_base_url() + main_js_file_hash
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

	var main_crdt_file_hash := scene_entity_definition.get_main_crdt_hash()
	var local_main_crdt_path: String = String()
	if not main_crdt_file_hash.is_empty():
		local_main_crdt_path = "user://content/" + main_crdt_file_hash
		var main_crdt_file_url: String = scene_entity_definition.get_base_url() + main_crdt_file_hash
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


func _on_try_spawn_scene(scene_item: SceneItem, local_main_js_path: String, local_main_crdt_path: String):
	if not local_main_js_path.is_empty() and not FileAccess.file_exists(local_main_js_path):
		printerr("Couldn't get main.js file")
		local_main_js_path = ""

	if not local_main_crdt_path.is_empty() and not FileAccess.file_exists(local_main_crdt_path):
		printerr("Couldn't get main.crdt file")
		local_main_crdt_path = ""

	if local_main_crdt_path.is_empty() and local_main_js_path.is_empty():
		printerr("Couldn't spawn the scene ", scene_item.id)
		return false


	var scene_number_id: int = Global.scene_runner.start_scene(
		local_main_js_path,
		local_main_crdt_path,
		scene_item.scene_entity_definition
	)
	scene_item.scene_number_id = scene_number_id

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
