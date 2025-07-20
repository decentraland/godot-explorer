extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

const SETTINGS_PATH_KEY = "renderdoc/executable_path"

var editor_settings: EditorSettings


func _init(a_plugin: EditorPlugin):
	plugin = a_plugin
	editor_settings = plugin.get_editor_interface().get_editor_settings()
	# Create RenderDoc path setting
	if not editor_settings.has_setting(SETTINGS_PATH_KEY):
		editor_settings.set(SETTINGS_PATH_KEY, "")
		editor_settings.add_property_info(
			{
				"type": TYPE_STRING,
				"name": SETTINGS_PATH_KEY,
				"hint": PROPERTY_HINT_FILE,
				"hint_string": "Executable"
			}
		)


func _is_renderdoc_available() -> bool:
	var path = editor_settings.get(SETTINGS_PATH_KEY)
	return path && FileAccess.file_exists(path)


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Launch RenderDoc", id)
	menu.set_item_disabled(id, !_is_renderdoc_available())
	editor_settings.settings_changed.connect(
		func(): menu.set_item_disabled(id, !_is_renderdoc_available())
	)


func execute():
	var script_dir = get_script().resource_path.get_base_dir()
	var base_settings_path = script_dir.path_join("base_settings.json")
	var output_path = script_dir.path_join("settings.cap")

	# Read base settings file
	var file = FileAccess.open(base_settings_path, FileAccess.READ)
	if !file:
		push_error("Missing base_settings.json", "File not found at: " + base_settings_path)
		return
	var json_content = file.get_as_text()
	file.close()

	# Replace placeholders
	json_content = json_content.replace("$GODOT_EXECUTABLE_PATH$", OS.get_executable_path())
	json_content = json_content.replace("$PROJECT_PATH$", ProjectSettings.globalize_path("res://"))

	# Write processed config
	var file_out = FileAccess.open(output_path, FileAccess.WRITE)
	if !file_out:
		push_error("Failed to write settings", "Could not create: " + output_path)
		return
	file_out.store_string(json_content)
	file_out.close()

	var path = editor_settings.get(SETTINGS_PATH_KEY)
	if _is_renderdoc_available():
		var pid = OS.create_process(path, [ProjectSettings.globalize_path(output_path)])
		if pid == -1:
			push_error(
				"RenderDoc Launch Failed\n",
				"Failed to execute RenderDoc at:\n%s\n\nCheck the path and try again." % path
			)
	else:
		push_error(
			"RenderDoc Not Found\n",
			(
				"Executable not found at:\n%s\n\nConfigure path in Editor Settings → renderdoc/executable_path"
				% path
			)
		)
