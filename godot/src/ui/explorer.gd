extends Node

var scene_runner: SceneManager = null
var realm: Realm = null
var parcel_manager: ParcelManager = null

#@onready var player = $Player
@onready var label_fps = %Label_FPS
@onready var label_ram = %Label_RAM
@onready var control_menu = $UI/Control_Menu
@onready var control_minimap = $UI/Control_Minimap
@onready var panel_bottom_left = $UI/Panel_BottomLeft
@onready var player = $Player

var parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0

func _process(_dt):
	parcel_position_real = Vector2(player.position.x*0.0625, -player.position.z*0.0625)
	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	parcel_manager.update_position(parcel_position)
	control_minimap.set_center_position(parcel_position_real)
	
func _ready():
	player.position = 16 * Vector3(78, 0.1, 6) 
	player.look_at(16 * Vector3(73, 0, 9) )

	scene_runner = get_tree().root.get_node("scene_runner")
	scene_runner.set_camera_and_player_node(player, player, self.panel_bottom_left._on_console_add)
	
	realm = get_tree().root.get_node("realm")
	
	parcel_manager = ParcelManager.new()
	add_child(parcel_manager)

func _on_check_button_toggled(button_pressed):
	scene_runner.set_pause(button_pressed)

func _on_ui_gui_input(event):
	if event is InputEventMouseButton and event.pressed:
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	
func _unhandled_input(event):
	if event is InputEventKey:
		if event.pressed and event.keycode == KEY_TAB:
			if not control_menu.visible:
				control_menu.show()
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				
		if event.pressed and event.keycode == KEY_M:
			if control_menu.visible:
				control_menu.hide()
			else:
				control_menu.show_map()
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				
		if event.pressed and event.keycode == KEY_ESCAPE:
			if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				
func _toggle_ram_usage(visibility:bool):
	if visibility:
		label_ram.show()
	else:
		label_ram.hide()

func _on_control_minimap_request_open_map():
	control_menu.show_map()
	
func _on_control_menu_jump_to(parcel:Vector2i):
	player.set_position(Vector3i(parcel.x*16, 3, -parcel.y*16))
	control_menu.hide()

func _on_control_menu_hide_menu():
	control_menu.hide()

func _on_panel_bottom_left_request_change_realm(realm_string):
	realm.set_realm(realm_string)

func _on_panel_bottom_left_request_change_scene_radius(new_value):
	parcel_manager.set_scene_radius(new_value)

func _on_panel_bottom_left_request_pause_scenes(enabled):
	scene_runner.set_pause(enabled)

func _on_timer_timeout():
	label_ram.set_text("RAM Usage: "+ str(OS.get_static_memory_usage()/1024.0/1024.0) + " MB")
	label_fps.set_text(str(Engine.get_frames_per_second()) + " FPS")


func _on_control_menu_toggle_ram(visibility):
	label_ram.visible = visibility


func _on_control_menu_toggle_fps(visibility):
	label_fps.visible = visibility


func _on_control_menu_toggle_minimap(visibility):
	control_minimap.visible = visibility
