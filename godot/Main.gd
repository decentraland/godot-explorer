extends Node

var scene_runner: SceneRunner
@onready var scene_list = $UI/Panel_Scenes/ItemList_Scenes

func _process(delta):
	%Label_FPS.set_text(str(Engine.get_frames_per_second()) + " FPS")
	
func _ready():
	scene_runner = SceneRunner.new()
	scene_runner.set_camera_node($Camera3D)
	self.add_child(scene_runner)
	print('scene_runner created')
	
	$UI/Panel_SpawnScene/OptionButton_SceneToSpawn.clear()	
	for path in DirAccess.get_directories_at("res://assets/scenes"):
		$UI/Panel_SpawnScene/OptionButton_SceneToSpawn.add_item(path)
	$UI/Panel_SpawnScene/OptionButton_SceneToSpawn.select(0)
func _on_add_button_pressed():
	var path = $UI/Panel_SpawnScene/OptionButton_SceneToSpawn.text
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
		
