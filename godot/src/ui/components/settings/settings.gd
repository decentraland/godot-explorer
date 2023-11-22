extends Control

var resolution_manager: ResolutionManager = ResolutionManager.new()

@onready var general = $VBoxContainer_General
@onready var graphics = $VBoxContainer_Graphics
@onready var monitoring = $VBoxContainer_Monitoring

@onready var text_edit = $VBoxContainer_General/VBoxContainer_CachePath/TextEdit_CachePath
@onready var window_size_menu_button = $VBoxContainer_Graphics/WindowSize/MenuButton_WindowSize
@onready var resolution_menu_button = $VBoxContainer_Graphics/Resolution/MenuButton_Resolution
@onready var minimap = $VBoxContainer_General/Checkbox_Minimap
@onready var label_gui_scale = $VBoxContainer_Graphics/GuiScale/HBoxContainer/Label_GuiScale
@onready var h_slider_gui_scale = $VBoxContainer_Graphics/GuiScale/HBoxContainer/HSlider_GuiScale
@onready var menu_button_limit_fps = $VBoxContainer_Graphics/LimitFps/MenuButton_LimitFps
@onready var menu_button_skybox = $VBoxContainer_Graphics/Skybox/MenuButton_Skybox


func _ready():
	if Global.is_mobile:
		window_size_menu_button.disabled = true
		resolution_menu_button.disabled = true

	general.show()
	graphics.hide()
	monitoring.hide()

	text_edit.text = Global.config.local_content_dir

	resolution_manager.refresh_window_options()
	for item in resolution_manager.window_options.keys():
		window_size_menu_button.add_item(item)

	load_resolutions()
	refresh_resolution()

	menu_button_limit_fps.selected = Global.config.limit_fps
	menu_button_skybox.selected = Global.config.skybox


func _on_button_pressed():
	self.hide()


func _on_window_size_menu_button_item_selected(index):
	var current_window_size: String = window_size_menu_button.get_item_text(index)
	resolution_manager.change_window_size(get_window(), get_viewport(), current_window_size)
	load_resolutions()
	resolution_manager.center_window(get_window())
	Global.config.window_size = current_window_size
	Global.config.resolution = current_window_size
	Global.config.ui_scale = 1.0
	refresh_resolution()


func _on_resolution_menu_button_item_selected(index):
	var current_res: String = resolution_menu_button.get_item_text(index)
	resolution_manager.change_resolution(get_window(), get_viewport(), current_res)
	resolution_manager.center_window(get_window())
	resolution_manager.change_ui_scale(get_window(), 1.0)
	Global.config.resolution = current_res
	Global.config.ui_scale = 1.0
	Global.config.save_to_settings_file()
	refresh_resolution()


func refresh_resolution():
	for index in range(resolution_menu_button.item_count):
		var current_res: String = resolution_menu_button.get_item_text(index)
		if current_res == Global.config.resolution:
			resolution_menu_button.selected = index

	for index in range(window_size_menu_button.item_count):
		var current_res: String = window_size_menu_button.get_item_text(index)
		if current_res == Global.config.window_size:
			window_size_menu_button.selected = index

	h_slider_gui_scale.set_value_no_signal(Global.config.ui_scale * 100.0)
	label_gui_scale.text = str(round(100.0 * Global.config.ui_scale)) + "%"


func load_resolutions():
	resolution_menu_button.clear()
	for item in resolution_manager.resolution_options:
		resolution_menu_button.add_item(item)


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
		resolution_manager.change_ui_scale(get_window(), h_slider_gui_scale.value / 100.0)
		Global.config.ui_scale = h_slider_gui_scale.value / 100.0
		label_gui_scale.text = str(round(100.0 * Global.config.ui_scale)) + "%"
		Global.config.save_to_settings_file()


func _on_button_clear_cache_pressed():
	# Clean the content cache folder
	if DirAccess.dir_exists_absolute(Global.config.local_content_dir):
		for file in DirAccess.get_files_at(Global.config.local_content_dir):
			DirAccess.remove_absolute(Global.config.local_content_dir + file)
		DirAccess.remove_absolute(Global.config.local_content_dir)

	if not DirAccess.dir_exists_absolute(Global.config.local_content_dir):
		DirAccess.make_dir_absolute(Global.config.local_content_dir)


func _on_checkbox_fps_toggled(button_pressed):
	Global.config.show_fps = button_pressed


func _on_menu_button_limit_fps_item_selected(index):
	Global.config.limit_fps = index
	resolution_manager.apply_fps_limit()
	Global.config.save_to_settings_file()


func _on_menu_button_skybox_item_selected(index):
	Global.config.skybox = index
	Global.config.save_to_settings_file()
