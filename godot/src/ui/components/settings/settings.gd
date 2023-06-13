extends Control
@onready var general = $General
@onready var graphics = $Graphics
@onready var monitoring = $Monitoring

@onready var text_edit = $General/CachePath/TextEdit
@onready var window_size_menu_button = $Graphics/WindowSize/WindowSizeMenuButton
@onready var resolution_menu_button = $Graphics/Resolution/ResolutionMenuButton
@onready var map = $General/Map
@onready var h_slider = $Graphics/GuiScale/HSlider

signal toggle_scenes_list_visibility()
signal toggle_scenes_spawner_visibility()
signal toggle_ram_usage_visibility()
signal toggle_map_visibility()

var resolutions_16_9 := [
	Vector2i(1280, 720),
	Vector2i(1366, 768), 
	Vector2i(1920, 1080), 
	Vector2i(2560, 1440), 
	Vector2i(3200, 1800), 
	Vector2i(3840, 2160)
]

var config = ConfigFile.new()

func _ready():
	general.show()
	graphics.hide()
	monitoring.hide()
	
	text_edit.text = OS.get_user_data_dir() + "/content"
	
	var screen_size = DisplayServer.screen_get_size()
	for j in range(resolutions_16_9.size()):
		var size = resolutions_16_9[j]
		if screen_size.x >= size.x and screen_size.y >= size.y:
			window_size_menu_button.add_item("%d x %d" % [size.x, size.y], j)
		
	var err = config.load("user://settings.cfg")
	if err != OK:
		change_window_size(2)
		return
	
	var window = get_window()
	var size  = config.get_value("display", "window")
	DisplayServer.window_set_size(size)
	get_viewport().size = Vector2(size)
	window.content_scale_mode = config.get_value("display", "mode")
	window.content_scale_size = config.get_value("display", "resolution")
	window.content_scale_factor = config.get_value("display", "ui")

func change_window_size(id: int) -> void:
	DisplayServer.window_set_size(resolutions_16_9[id])
	get_viewport().size = Vector2(resolutions_16_9[id])
	load_resolutions()
	change_resolution(id)
	
	config.set_value("display", "window", resolutions_16_9[id])
	_save()
	
func change_resolution(id: int) -> void:
	var res_size = Vector2(resolutions_16_9[id])
	var factor = get_viewport().size.x / res_size.x
	var window = get_window()
	if factor == 1.0:
		window.content_scale_mode = Window.CONTENT_SCALE_MODE_DISABLED
		window.content_scale_size = Vector2.ZERO
	else:
		window.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
		window.content_scale_size = res_size
		
	config.set_value("display", "mode", window.content_scale_mode)
	config.set_value("display", "resolution", window.content_scale_size)
	_save()
	
func _on_button_pressed():
	self.hide() 
	
# Set window size
func _on_window_size_menu_button_item_selected(index):
	change_window_size(window_size_menu_button.get_item_id(index))

# Set resolution
func _on_resolution_menu_button_item_selected(index):
	var id = resolution_menu_button.get_item_id(index)
	change_resolution(id)
	
func load_resolutions():
	resolution_menu_button.clear()
	var window_size = DisplayServer.window_get_size()
	for j in range(resolutions_16_9.size()):
		var size = resolutions_16_9[j]
		if window_size.x >= size.x and window_size.y >= size.y:
			resolution_menu_button.add_item("%d x %d" % [size.x, size.y], j)

func _on_general_button_toggled(button_pressed):
	general.show()
	graphics.hide()
	monitoring.hide()

func _on_graphic_button_toggled(button_pressed):
	general.hide()
	graphics.show()
	monitoring.hide()

func _on_monitoring_button_toggled(button_pressed):
	general.hide()
	graphics.hide()
	monitoring.show()


func _on_spawned_scenes_toggled(button_pressed):
	emit_signal('toggle_scenes_list_visibility', button_pressed)

func _on_scenes_selector_2_toggled(button_pressed):
	emit_signal('toggle_scenes_spawner_visibility', button_pressed)

func _on_ram_usage_toggled(button_pressed):
	emit_signal('toggle_ram_usage_visibility', button_pressed)

func _on_map_toggled(button_pressed):
	emit_signal('toggle_map_visibility', button_pressed)


func _on_menu_button_gui_scale_item_selected(index):
	pass # Replace with function body.

func _on_h_slider_drag_ended(value_changed):
	if value_changed:
		var window = get_window()
		window.content_scale_factor = h_slider.value / 100.0
		config.set_value("display", "ui", window.content_scale_factor)
		_save()
		
func _save():
	config.save("user://settings.cfg")
