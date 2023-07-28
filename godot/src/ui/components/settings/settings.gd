extends Control
@onready var general = $VBoxContainer_General
@onready var graphics = $VBoxContainer_Graphics
@onready var monitoring = $VBoxContainer_Monitoring

@onready var text_edit = $VBoxContainer_General/VBoxContainer_CachePath/TextEdit_CachePath
@onready var window_size_menu_button = $VBoxContainer_Graphics/WindowSize/MenuButton_WindowSize
@onready var resolution_menu_button = $VBoxContainer_Graphics/Resolution/MenuButton_Resolution
@onready var minimap = $VBoxContainer_General/Checkbox_Minimap
@onready var h_slider_ui_scale = $VBoxContainer_Graphics/GuiScale/HSlider_GuiScale

signal toggle_ram_usage_visibility(visibility: bool)
signal toggle_map_visibility(visibility: bool)
signal toggle_fps_visibility(visibility: bool)

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
	if OS.get_name() == "Android":
		window_size_menu_button.disabled = true
		resolution_menu_button.disabled = true

	general.show()
	graphics.hide()
	monitoring.hide()

	text_edit.text = OS.get_user_data_dir() + "/content"

	var screen_size = DisplayServer.screen_get_size()
	for j in range(resolutions_16_9.size()):
		var option_size = resolutions_16_9[j]
		if screen_size.x >= option_size.x and screen_size.y >= option_size.y:
			window_size_menu_button.add_item("%d x %d" % [option_size.x, option_size.y], j)

	var err = config.load("user://settings.cfg")
	if err != OK:
		change_window_size(2)
		return

	var window = get_window()
	var window_size = config.get_value("display", "window")

	var size_text = "%d x %d" % [window_size.x, window_size.y]
	var resolution_text = "%d x %d" % [window.content_scale_size.x, window.content_scale_size.y]
	for idx in range(window_size_menu_button.item_count):
		var item_text = window_size_menu_button.get_item_text(idx)
		if item_text == size_text:
			change_window_size(window_size_menu_button.get_item_id(idx), false)
			window_size_menu_button.selected = idx
			break

	if resolution_text == "0 x 0":
		resolution_text = size_text

	for idx in range(resolution_menu_button.item_count):
		var item_text = resolution_menu_button.get_item_text(idx)
		if item_text == size_text:
			change_resolution(resolution_menu_button.get_item_id(idx), false)
			resolution_menu_button.selected = idx
			break

	#window.content_scale_factor = config.get_value("display", "ui")
	h_slider_ui_scale.value = 100.0 * window.content_scale_factor

	DisplayServer.window_set_position(screen_size * 0.5 - window.size * 0.5)


func change_window_size(id: int, save: bool = true) -> void:
	DisplayServer.window_set_size(resolutions_16_9[id])
	get_viewport().size = Vector2(resolutions_16_9[id])
	load_resolutions()
	change_resolution(id)

	var ui_factor = Vector2(resolutions_16_9[id]).x / 1280
	h_slider_ui_scale.value = 100 * ui_factor
	_on_h_slider_drag_ended.call_deferred(true)

	if save:
		config.set_value("display", "window", resolutions_16_9[id])
		_save()


func change_resolution(id: int, save: bool = true) -> void:
	var res_size = Vector2(resolutions_16_9[id])
	var factor = get_viewport().size.x / res_size.x
	var window = get_window()
	if factor == 1.0:
		window.content_scale_mode = Window.CONTENT_SCALE_MODE_DISABLED
		window.content_scale_size = Vector2.ZERO
	else:
		window.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
		window.content_scale_size = res_size

	if save:
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
		var res_size = resolutions_16_9[j]
		if window_size.x >= res_size.x and window_size.y >= res_size.y:
			resolution_menu_button.add_item("%d x %d" % [res_size.x, res_size.y], j)


func _on_general_button_toggled(_button_pressed):
	general.show()
	graphics.hide()
	monitoring.hide()


func _on_graphic_button_toggled(_button_pressed):
	general.hide()
	graphics.show()
	monitoring.hide()


func _on_monitoring_button_toggled(_button_pressed):
	general.hide()
	graphics.hide()
	monitoring.show()


func _on_h_slider_drag_ended(value_changed):
	if value_changed:
		var window = get_window()
		window.content_scale_factor = h_slider_ui_scale.value / 100.0
		config.set_value("display", "ui", window.content_scale_factor)
		_save()


func _save():
	config.save("user://settings.cfg")


func _on_button_clear_cache_pressed():
	# Clean the content cache folder
	if DirAccess.dir_exists_absolute("user://content/"):
		for file in DirAccess.get_files_at("user://content/"):
			DirAccess.remove_absolute("user://content/" + file)
		DirAccess.remove_absolute("user://content")

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")


func _on_ram_usage_toggled(button_pressed):
	emit_signal("toggle_ram_usage_visibility", button_pressed)


func _on_checkbox_fps_toggled(button_pressed):
	emit_signal("toggle_fps_visibility", button_pressed)


func _on_map_toggled(button_pressed):
	emit_signal("toggle_map_visibility", button_pressed)
