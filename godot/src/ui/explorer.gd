extends Node

var scene_runner: SceneManager = null
var realm: Realm = null
var parcel_manager: ParcelManager = null

@onready var player = $Player

var parcel_position: Vector2i
func _process(_dt):
	%Label_FPS.set_text(str(Engine.get_frames_per_second()) + " FPS")
	
	parcel_position = Vector2i(floori(player.position.x*0.0625), floori(-player.position.z*0.0625))
	parcel_manager.update_position(parcel_position)
	
	%Label_ParcelPosition.set_text(str(parcel_position) )
	$UI/Map.set_center_position(parcel_position)
	
func _ready():	
	player.position = 16 * Vector3(72, 0.1, 10) 
	player.look_at(16 * Vector3(73, 0, 9) )
	scene_runner = get_tree().root.get_node("scene_runner")
	scene_runner.set_camera_node(player)
	
	realm = get_tree().root.get_node("realm")
	
	parcel_manager = ParcelManager.new()
	add_child(parcel_manager)

func _on_check_button_toggled(button_pressed):
	scene_runner.set_pause(button_pressed)


func _on_ui_gui_input(event):
	if event is InputEventMouseButton and event.pressed:
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func _on_option_button_item_selected(index):
	realm.set_realm($UI/OptionButton.get_item_text(index))
