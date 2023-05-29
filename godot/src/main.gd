extends Node

var scene_runner: SceneRunner = null
var realm: Realm = null
var parcel_manager: ParcelManager = null

@onready var scene_list = $UI/Panel_Scenes/ItemList_Scenes
@onready var option_button_scene_to_spawn = $UI/Panel_SpawnScene/OptionButton_SceneToSpawn
@onready var free_camera = $Camera3D_FreeCamera

var parcel_position: Vector2i
func _process(_dt):
	%Label_FPS.set_text(str(Engine.get_frames_per_second()) + " FPS")
	
	parcel_position = Vector2i(floori(free_camera.position.x*0.0625), floori(free_camera.position.z*0.0625))
	parcel_manager.update_position(parcel_position)
	
	%Label_ParcelPosition.set_text(str(parcel_position) )
	$UI/Map.set_center_position(parcel_position)
	
func _ready():
	free_camera.position = 16 * Vector3(72, 0.5, -9) 
	scene_runner = SceneRunner.new()
	add_child(scene_runner)
	
	scene_runner.set_camera_node(free_camera)
	
	realm = Realm.new()
	add_child(realm)
	
#	realm.set_realm("mannakia.dcl.eth")
	realm.set_realm("https://sdk-test-scenes.decentraland.zone")
	realm.realm_changed.connect(self._on_realm_changed)
	
	parcel_manager = ParcelManager.new()
	parcel_manager.init(scene_runner, realm)
	
	option_button_scene_to_spawn.clear()	
	for path in DirAccess.get_directories_at("res://assets/scenes"):
		option_button_scene_to_spawn.add_item(path)
	option_button_scene_to_spawn.select(0)
	
		
	
func _on_add_button_pressed():
	var path = "res://assets/scenes/" + option_button_scene_to_spawn.text + "/index.js"
	var scene_id = scene_runner.start_scene(path, Vector3(float($UI/Panel_SpawnScene/X.value) * 16, 0, float($UI/Panel_SpawnScene/Z.value) * 16))
	var item = scene_list.add_item(path)
	scene_list.set_item_metadata(item, scene_id)


func _on_button_delete_scene_pressed():
	var selected = scene_list.get_selected_items()
	if selected.size() > 0:
		var scene_id: int = scene_list.get_item_metadata(selected[0])
		if scene_runner.kill_scene(scene_id):
			print(scene_id, " scene deleted")
			scene_list.remove_item(selected[0])
			if scene_list.item_count > 0:
				scene_list.select(0)
		else:
			print(scene_id, " couldn't delete scene")
		
func _on_realm_changed(): 
	pass
