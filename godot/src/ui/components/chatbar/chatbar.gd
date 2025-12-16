extends Control


func _on_hud_button_discover_pressed() -> void:
	Global.open_discover.emit()


func _on_hud_button_share_pressed() -> void:
	print("Share scene")


func _on_button_chat_pressed() -> void:
	Global.open_chat.emit()
