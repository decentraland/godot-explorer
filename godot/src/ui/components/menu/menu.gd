extends Control

@export var group: ButtonGroup
var buttons_quantity: int = 0
var pressed_index: int = 0

signal hide_menu
signal jump_to(Vector2i)
signal toggle_minimap
signal toggle_fps
signal toggle_ram

#signals from advanced settings
signal request_pause_scenes(enabled: bool)
signal preview_hot_reload(scene_type: String, scene_id: String)

@onready var color_rect_header = $ColorRect_Header

@onready var control_discover = $ColorRect_Background/Control_Discover
@onready var control_settings = $ColorRect_Background/Control_Settings
@onready var control_map = $ColorRect_Background/Control_Map
@onready var control_advance_settings = $ColorRect_Background/Control_AdvanceSettings
@onready var control_backpack = $ColorRect_Background/Control_Backpack

var selected_node: Control

@onready var button_discover = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_Discover
@onready var button_map = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_Map
@onready var button_settings = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_Settings
@onready
var button_advance_settings = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_AdvanceSettings

var resolutions := [
	Vector2i(1920, 1080), Vector2i(1280, 720), Vector2i(800, 600), Vector2i(400, 300)
]
var sizes := [Vector2i(1152, 648), Vector2i(576, 324)]


func _ready():
	self.modulate = Color(1, 1, 1, 0)
	button_settings.set_pressed(true)
	selected_node = control_settings
	control_map.hide()
	control_settings.show()
	control_discover.hide()
	control_advance_settings.hide()
	control_backpack.hide()
	control_map.jump_to.connect(_jump_to)


func _unhandled_input(event):
	if event is InputEventKey and visible:
		if event.pressed and event.keycode == KEY_TAB:
			pressed_index = group.get_pressed_button().get_index()
			buttons_quantity = group.get_buttons().size() - 1
			control_map.clear()

			if pressed_index < buttons_quantity:
				group.get_buttons()[pressed_index + 1].set_pressed(true)
				group.get_buttons()[pressed_index + 1].emit_signal("pressed")
			else:
				#change index to 0 to include "Control Discover"
				group.get_buttons()[1].set_pressed(true)
				group.get_buttons()[1].emit_signal("pressed")
		if event.pressed and event.keycode == KEY_ESCAPE:
			emit_signal("hide_menu")
		if event.pressed and event.keycode == KEY_M:
			if selected_node == control_map:
				emit_signal("hide_menu")
			else:
				show_map()
		if event.pressed and event.keycode == KEY_P:
			if selected_node == control_settings:
				emit_signal("hide_menu")
			else:
				_on_button_settings_pressed()


func modulate_all():
	var tween_m = create_tween()
	tween_m.tween_property(control_discover, "modulate", Color(1, 1, 1, 0), 0.125)
	tween_m.tween_property(control_map, "modulate", Color(1, 1, 1, 0), 0.125)
	tween_m.tween_property(control_settings, "modulate", Color(1, 1, 1, 0), 0.125)
	tween_m.tween_property(control_advance_settings, "modulate", Color(1, 1, 1, 0), 0.125)


func hide_all():
	control_discover.hide()
	control_map.hide()
	control_settings.hide()
	control_advance_settings.hide()


func _on_button_close_pressed():
	emit_signal("hide_menu")


func _jump_to(parcel: Vector2i):
	emit_signal("jump_to", parcel)


func close():
	color_rect_header.hide()
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3).set_ease(Tween.EASE_IN_OUT)
	var tween_h = create_tween()
	tween_h.tween_callback(hide).set_delay(0.3)


func show_last():
	self.show()
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.3).set_ease(Tween.EASE_IN_OUT)
	color_rect_header.show()


func show_map():
	self.show()

	if selected_node != control_map:
		self._on_button_map_pressed()
		button_map.set_pressed(true)
	var tween = create_tween()
	tween.tween_property(self, "modulate", Color(1, 1, 1), 0.25).set_ease(Tween.EASE_IN_OUT)
	color_rect_header.show()


func _on_control_settings_toggle_fps_visibility(visibility):
	emit_signal("toggle_fps", visibility)


func _on_control_settings_toggle_map_visibility(visibility):
	emit_signal("toggle_minimap", visibility)


func _on_control_settings_toggle_ram_usage_visibility(visibility):
	emit_signal("toggle_ram", visibility)


func _on_button_advance_settings_pressed():
	if selected_node != control_advance_settings:
		fade_out(selected_node)
		fade_in(control_advance_settings)


func _on_button_settings_pressed():
	if selected_node != control_settings:
		fade_out(selected_node)
		fade_in(control_settings)


func _on_button_map_pressed():
	if selected_node != control_map:
		fade_out(selected_node)
		fade_in(control_map)


func _on_button_discover_pressed():
	if selected_node != control_discover:
		fade_out(selected_node)
		fade_in(control_discover)


func _on_button_backpack_pressed():
	if selected_node != control_backpack:
		fade_out(selected_node)
		fade_in(control_backpack)


func fade_in(node: Control):
	selected_node = node
	node.show()
	var tween_m = create_tween()
	tween_m.tween_property(node, "modulate", Color(1, 1, 1), 0.3)


func fade_out(node: Control):
	var tween_m = create_tween()
	tween_m.tween_property(node, "modulate", Color(1, 1, 1, 0), 0.3)
	var tween_h = create_tween()
	tween_h.tween_callback(node.hide).set_delay(0.3)


func _on_control_advance_settings_preview_hot_reload(scene_type, scene_id):
	emit_signal("preview_hot_reload", scene_type, scene_id)


func _on_control_advance_settings_request_pause_scenes(enabled):
	emit_signal("request_pause_scene", enabled)
