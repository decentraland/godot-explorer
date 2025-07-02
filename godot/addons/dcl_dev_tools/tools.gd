@tool
extends EditorPlugin

var tools = [
	preload("./dev_tools/test_runner/tool.gd").new(self),
	preload("./dev_tools/typical_places/tool.gd").new(self),
	preload("./dev_tools/renderdoc/tool.gd").new(self)
]

var custom_menu: MenuButton
var menu_name = "DCL Tools"


func _enter_tree() -> void:
	custom_menu = MenuButton.new()
	custom_menu.name = "DCL_ToolsMenu"
	custom_menu.text = menu_name

	initialize_items()
	add_control_to_container(EditorPlugin.CONTAINER_TOOLBAR, custom_menu)
	call_deferred("_position_menu")


func initialize_items():
	var popup = custom_menu.get_popup()
	for tool_id in tools.size():
		tools[tool_id].populate_menu(popup, tool_id)
	popup.id_pressed.connect(_on_menu_item_selected)


func _on_menu_item_selected(id: int) -> void:
	assert(tools[id], "No tool found for that item!")
	tools[id].execute()


func _exit_tree() -> void:
	if !custom_menu:
		return
	var container = custom_menu.get_parent().get_parent()
	custom_menu.get_parent().remove_child(custom_menu)
	container.add_child(custom_menu)
	remove_control_from_container(EditorPlugin.CONTAINER_TOOLBAR, custom_menu)
	custom_menu.queue_free()


func _position_menu() -> void:
	var menu_bar := custom_menu.get_parent()
	menu_bar.remove_child(custom_menu)
	menu_bar.get_child(1).add_child(custom_menu)
