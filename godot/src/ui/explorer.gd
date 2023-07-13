extends Node

var scene_runner: SceneManager = null
var realm: Realm = null
var parcel_manager: ParcelManager = null
var pointer_tooltip_scene = preload("res://src/ui/components/pointer_tooltip/pointer_tooltip.tscn")

@onready var control_crosshair = $UI/Control_Crosshair
@onready var control_pointer_tooltip = $Control_PointerTooltip

@onready var label_fps = %Label_FPS
@onready var label_ram = %Label_RAM
@onready var control_menu = $UI/Control_Menu
@onready var control_minimap = $UI/Control_Minimap
@onready var panel_bottom_left = $UI/Panel_BottomLeft
@onready var player := $Player
@onready var contro_info_panel = $UI/Control_Minimap/Contro_InfoPanel

var parcel_position: Vector2i
var _last_parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0


func _process(_dt):
	parcel_position_real = Vector2(player.position.x * 0.0625, -player.position.z * 0.0625)
	control_minimap.set_center_position(parcel_position_real)

	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	if _last_parcel_position != parcel_position:
		parcel_manager.update_position(parcel_position)
		_last_parcel_position = parcel_position

		var scene_data = parcel_manager.get_current_scene_data()
		var title = scene_data.get("entity", {}).get("metadata", {}).get("display", {}).get(
			"title", "No title"
		)
		contro_info_panel.set_parcel_scene_name(title)


func _ready():
	control_pointer_tooltip.hide()
	var start_parcel_position: Vector2i = Vector2i(74, -2)
	player.position = 16 * Vector3(start_parcel_position.x, 0.1, -start_parcel_position.y)
	player.look_at(16 * Vector3(start_parcel_position.x + 1, 0, -(start_parcel_position.y + 1)))

	scene_runner = get_tree().root.get_node("scene_runner")
	scene_runner.set_camera_and_player_node(player.camera, player, self._on_scene_console_message)
	scene_runner.pointer_tooltip_changed.connect(self._on_pointer_tooltip_changed)

	realm = get_tree().root.get_node("realm")

	parcel_manager = ParcelManager.new()
	add_child(parcel_manager)


func _on_scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	_scene_console_message.call_deferred(scene_id, level, timestamp, text)


func _scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	var title: String = scene_runner.get_scene_title(scene_id)
	title += str(scene_runner.get_scene_base_parcel(scene_id))
	self.panel_bottom_left._on_console_add(title, level, timestamp, text)


func _on_pointer_tooltip_changed():
	change_tooltips.call_deferred()


func change_tooltips():
	var tooltips = scene_runner.get_tooltips()

	if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		if not tooltips.is_empty():
			control_pointer_tooltip.set_pointer_data(tooltips)
			control_pointer_tooltip.show()
		else:
			control_pointer_tooltip.hide()


func _on_check_button_toggled(button_pressed):
	scene_runner.set_pause(button_pressed)


func _on_ui_gui_input(event):
	if event is InputEventMouseButton and event.pressed:
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
			control_crosshair.show()


@onready var line_edit_command = $UI/LineEdit_Command


func _unhandled_input(event):
	if event is InputEventKey:
		if event.pressed and event.keycode == KEY_TAB:
			if not control_menu.visible:
				control_menu.show_last()
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				control_crosshair.hide()

		if event.pressed and event.keycode == KEY_M:
			if control_menu.visible:
				pass
			else:
				control_menu.show_map()
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				control_crosshair.hide()

		if event.pressed and event.keycode == KEY_ESCAPE:
			if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				control_crosshair.hide()

			if line_edit_command.visible:
				line_edit_command.hide()

		if event.pressed and event.keycode == KEY_ENTER:
			if not line_edit_command.visible:
				line_edit_command.text = ""
				line_edit_command.show()

			line_edit_command.grab_focus()


func _toggle_ram_usage(visibility: bool):
	if visibility:
		label_ram.show()
	else:
		label_ram.hide()


func _on_control_minimap_request_open_map():
	control_menu.show_map()


func _on_control_menu_jump_to(parcel: Vector2i):
	player.set_position(Vector3i(parcel.x * 16, 3, -parcel.y * 16))
	control_menu.close()


func _on_control_menu_hide_menu():
	control_menu.close()
	control_menu.control_map.clear()


func _on_panel_bottom_left_request_change_realm(realm_string):
	realm.set_realm(realm_string)


func _on_panel_bottom_left_request_change_scene_radius(new_value):
	parcel_manager.set_scene_radius(new_value)


func _on_panel_bottom_left_request_pause_scenes(enabled):
	scene_runner.set_pause(enabled)


func _on_timer_timeout():
	label_ram.set_text("RAM Usage: " + str(OS.get_static_memory_usage() / 1024.0 / 1024.0) + " MB")
	label_fps.set_text(str(Engine.get_frames_per_second()) + " FPS")


func _on_control_menu_toggle_ram(visibility):
	label_ram.visible = visibility


func _on_control_menu_toggle_fps(visibility):
	label_fps.visible = visibility


func _on_control_menu_toggle_minimap(visibility):
	control_minimap.visible = visibility


func _on_panel_bottom_left_preview_hot_reload(_scene_type, scene_id):
	parcel_manager.reload_scene(scene_id)


func _on_line_edit_command_text_submitted(new_text: String) -> void:
	line_edit_command.hide()

	var params := new_text.split(" ")
	var command_str := params[0].to_lower()
	if command_str == "/go" or command_str == "/goto" and params.size() > 1:
		var comma_params = params[1].split(",")
		if comma_params.size() > 1:
			_on_control_menu_jump_to(Vector2i(int(comma_params[0]), int(comma_params[1])))
		elif params.size() > 2:
			_on_control_menu_jump_to(Vector2i(int(params[1]), int(params[2])))
