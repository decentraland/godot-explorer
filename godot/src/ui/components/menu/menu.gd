extends Control

signal hide_menu()
signal jump_to(Vector2i)
signal toggle_minimap()
signal toggle_fps()
signal toggle_ram()

@onready var button_discover = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_Discover
@onready var button_map = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_Map
@onready var button_settings = $ColorRect_Header/HBoxContainer_ButtonsPanel/Button_Settings

@onready var control_settings = $Control_Settings
@onready var control_map = $Control_Map
@onready var control_discover = $Control_Discover

var resolutions:= [Vector2i(1920, 1080),Vector2i(1280,720),Vector2i(800,600), Vector2i(400,300)]
var sizes:= [Vector2i(1152, 648),Vector2i(576,324)]

func _ready():
	button_settings.set_pressed(true)
	control_map.hide()
	control_settings.show()
	control_discover.hide()
	
	control_map.jump_to.connect(_jump_to)

func _unhandled_input(event):
	if event is InputEventKey:
		if event.pressed and event.keycode == KEY_ESCAPE:
			emit_signal("hide_menu")
	
		if visible and event.pressed and event.keycode == KEY_TAB:
			if button_settings.button_pressed:
				button_map.set_pressed(true)
				hide_all()
				control_map.show()
#			elif button_map.button_pressed:
			else:
				button_settings.set_pressed(true)
				hide_all()
				control_settings.show()
#			else:
#				button_discover.set_pressed(true)
#				hide_all()
#				control_discover.show()


	
func _on_button_settings_pressed():
	hide_all()
	control_settings.show()

func _on_button_map_pressed():
	hide_all()
	control_map.show()
	
func _on_button_discover_pressed():
	hide_all()
	control_discover.show()
	
func hide_all():
	control_discover.hide()
	control_map.hide()
	control_settings.hide()
	
func _on_button_close_pressed():
	emit_signal("hide_menu")
	
func _jump_to(parcel:Vector2i):
	emit_signal('jump_to', parcel)
	
func show_map():
	self.show()
	self._on_button_map_pressed()
	button_map.set_pressed(true)
	

func _on_control_settings_toggle_fps_visibility(visibility):
	emit_signal('toggle_fps', visibility)


func _on_control_settings_toggle_map_visibility(visibility):
	emit_signal('toggle_minimap', visibility)


func _on_control_settings_toggle_ram_usage_visibility(visibility):
	emit_signal('toggle_ram', visibility)

