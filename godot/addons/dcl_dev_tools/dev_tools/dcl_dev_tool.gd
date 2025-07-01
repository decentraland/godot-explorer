extends Node

# This class is a template, it should be extended and its methods overriden

var plugin : EditorPlugin

func _init(a_plugin:EditorPlugin):
	plugin = a_plugin

# Should create the menu entry
func populate_menu(menu: PopupMenu, id: int):
	assert(false, "Subclass responsibility")


# Should run the action
func execute():
	assert(false, "Subclass responsibility")
