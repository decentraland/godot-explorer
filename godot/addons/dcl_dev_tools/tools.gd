@tool
extends EditorPlugin

var tools = [
	preload("./dev_tools/test_runner/tool.gd").new(self),
	preload("./dev_tools/typical_places/tool.gd").new(self),
	preload("./dev_tools/renderdoc/tool.gd").new(self),
	preload("./dev_tools/resource_counter/tool.gd").new(self)
]

var custom_menu_3d: MenuButton
var custom_menu_2d: MenuButton
var menu_name = "DCL Tools"


func _enter_tree() -> void:
	# Create menu for 3D editor
	custom_menu_3d = MenuButton.new()
	custom_menu_3d.name = "DCL_ToolsMenu3D"
	custom_menu_3d.text = menu_name
	initialize_items(custom_menu_3d)
	add_control_to_container(EditorPlugin.CONTAINER_SPATIAL_EDITOR_MENU, custom_menu_3d)

	# Create menu for 2D editor
	custom_menu_2d = MenuButton.new()
	custom_menu_2d.name = "DCL_ToolsMenu2D"
	custom_menu_2d.text = menu_name
	initialize_items(custom_menu_2d)
	add_control_to_container(EditorPlugin.CONTAINER_CANVAS_EDITOR_MENU, custom_menu_2d)


func initialize_items(menu: MenuButton):
	var popup = menu.get_popup()
	for tool_id in tools.size():
		tools[tool_id].populate_menu(popup, tool_id)
	popup.id_pressed.connect(_on_menu_item_selected)


func _on_menu_item_selected(id: int) -> void:
	assert(tools[id], "No tool found for that item!")
	tools[id].execute()


func _exit_tree() -> void:
	# Clean up 3D editor menu
	if custom_menu_3d and is_instance_valid(custom_menu_3d):
		remove_control_from_container(EditorPlugin.CONTAINER_SPATIAL_EDITOR_MENU, custom_menu_3d)
		custom_menu_3d.queue_free()

	# Clean up 2D editor menu
	if custom_menu_2d and is_instance_valid(custom_menu_2d):
		remove_control_from_container(EditorPlugin.CONTAINER_CANVAS_EDITOR_MENU, custom_menu_2d)
		custom_menu_2d.queue_free()

	for tool in tools:
		tool.cleanup()
