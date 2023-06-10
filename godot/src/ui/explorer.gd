extends Node

var scene_runner: SceneManager = null
var realm: Realm = null
var parcel_manager: ParcelManager = null

@onready var player = $Player
@onready var panel_bottom_left = $UI/Panel_BottomLeft
@onready var label_fps = %Label_FPS
@onready var label_parcel_position = %Label_ParcelPosition
@onready var option_button_realm = $UI/Panel_BottomLeft/OptionButton_Realm
@onready var minimap = $UI/Minimap
@onready var h_slider_scene_radius = $UI/Panel_BottomLeft/HSlider_SceneRadius
@onready var label_scene_radius_value = $UI/Panel_BottomLeft/Label_SceneRadiusValue

var parcel_position: Vector2i
var parcel_position_real: Vector2i
var panel_bottom_left_height: int = 0

func _process(_dt):
	label_fps.set_text(str(Engine.get_frames_per_second()) + " FPS")
	parcel_position_real = Vector2(player.position.x*0.0625, -player.position.z*0.0625)
	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	parcel_manager.update_position(parcel_position)
	label_parcel_position.set_text(str(parcel_position))
	minimap.set_center_position(parcel_position_real)
	
func _ready():
	panel_bottom_left_height = panel_bottom_left.size.y
	
	player.position = 16 * Vector3(72, 0.1, 10) 
	player.look_at(16 * Vector3(73, 0, 9) )
	scene_runner = get_tree().root.get_node("scene_runner")
	scene_runner.set_camera_and_player_node(player, player)
	
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
	realm.set_realm(option_button_realm.get_item_text(index))

const COLLAPSED_SIZE = 28
func _on_button_collapse_pressed():
	if panel_bottom_left.size.y >= panel_bottom_left_height:
		panel_bottom_left.size.y = COLLAPSED_SIZE
		panel_bottom_left.position.y += panel_bottom_left_height - COLLAPSED_SIZE
	else:
		panel_bottom_left.size.y = panel_bottom_left_height
		panel_bottom_left.position.y -= panel_bottom_left_height - COLLAPSED_SIZE


func _on_h_slider_scene_radius_drag_ended(value_changed):
	if value_changed:
		parcel_manager.set_scene_radius(h_slider_scene_radius.value)
		label_scene_radius_value.text = str(h_slider_scene_radius.value)
