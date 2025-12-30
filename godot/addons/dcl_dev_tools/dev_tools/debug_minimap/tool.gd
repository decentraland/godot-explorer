extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

var enabled: bool = false


func populate_menu(menu: PopupMenu, id: int):
	menu.add_check_item("Debug Minimap", id)
	menu.set_item_checked(menu.get_item_index(id), enabled)


func execute():
	# Toggle the state
	enabled = not enabled

	# Update run args
	var current_args = ProjectSettings.get("editor/run/main_run_args")
	if current_args == null:
		current_args = ""

	# Remove existing --debug-minimap flag if present
	var args_array = current_args.split(" ")
	var new_args = []
	for arg in args_array:
		if arg != "--debug-minimap" and arg != "":
			new_args.append(arg)

	# Add the flag if enabled
	if enabled:
		new_args.append("--debug-minimap")

	var new_args_string = " ".join(new_args)
	ProjectSettings.set("editor/run/main_run_args", new_args_string)

	# Update the menu checkmark
	_update_menu_checkmark(enabled)

	print("Debug Minimap: ", "Enabled" if enabled else "Disabled")
	if enabled:
		print("Run args: ", new_args_string)


func _update_menu_checkmark(enabled: bool):
	# Update both 2D and 3D menu checkmarks
	var menus = [
		plugin.get_editor_interface().get_base_control().find_child("DCL_ToolsMenu3D", true, false),
		plugin.get_editor_interface().get_base_control().find_child("DCL_ToolsMenu2D", true, false)
	]

	for menu_button in menus:
		if menu_button and menu_button is MenuButton:
			var popup = menu_button.get_popup()
			for i in range(popup.item_count):
				if popup.get_item_text(i) == "Debug Minimap":
					popup.set_item_checked(i, enabled)
					break
