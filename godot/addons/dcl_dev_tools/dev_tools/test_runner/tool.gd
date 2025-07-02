extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Run Avatar Tests", id)


func execute():
	var old_args = ProjectSettings.get("editor/run/main_run_args")
	ProjectSettings.set("editor/run/main_run_args", "--avatar-renderer --avatars --use-test-input")
	plugin.get_editor_interface().play_main_scene()
	ProjectSettings.set("editor/run/main_run_args", old_args)
