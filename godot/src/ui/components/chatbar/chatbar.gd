extends Control

signal share_place


func _on_hud_button_discover_pressed() -> void:
	Global.open_discover.emit()
	Global.send_haptic_feedback()


func _on_hud_button_share_pressed() -> void:
	share_place.emit()
	Global.send_haptic_feedback()


func _on_button_chat_pressed() -> void:
	Global.open_chat.emit()
	Global.send_haptic_feedback()
