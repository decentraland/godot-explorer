extends Control


func _on_button_discover_pressed() -> void:
	Global.get_explorer().control_menu.async_show_discover()


func _on_button_map_pressed() -> void:
	Global.get_explorer().control_menu.show_map()


func _on_button_backpack_pressed() -> void:
	Global.get_explorer().control_menu.async_show_backpack()


func _on_button_settings_pressed() -> void:
	Global.get_explorer().control_menu.async_show_settings()
