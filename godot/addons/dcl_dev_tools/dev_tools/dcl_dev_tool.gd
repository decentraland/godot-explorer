extends Node

# This class is a template, it should be extended and its methods overriden

var plugin: EditorPlugin


func _init(a_plugin: EditorPlugin):
	plugin = a_plugin


func populate_menu(_menu: PopupMenu, _id: int):
	assert(false, "Subclass responsibility")


func execute():
	assert(false, "Subclass responsibility")


func cleanup():
	return
