extends Control

signal hide_menu()
signal jump_to(Vector2i)

@onready var control_settings = $Control_Settings
@onready var control_map = $Control_Map

var resolutions:= [Vector2i(1920, 1080),Vector2i(1280,720),Vector2i(800,600), Vector2i(400,300)]
var sizes:= [Vector2i(1152, 648),Vector2i(576,324)]

func _ready():
	control_map.hide()
	control_settings.show()
	control_map.jump_to.connect(_jump_to)

func _unhandled_input(event):
	if event is InputEventKey:
		if event.pressed and event.keycode == KEY_ESCAPE:
			emit_signal("hide_menu")

func _on_button_close_pressed():
	emit_signal("hide_menu")
	
func _on_button_settings_pressed():
	control_map.hide()
	control_settings.show()

func _on_button_map_pressed():
	control_settings.hide()
	control_map.show()
	
func _jump_to(parcel:Vector2i):
	emit_signal('jump_to', parcel)
	
func show_map():
	self.show()
	self._on_button_map_pressed()
	
