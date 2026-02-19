extends StaticButton


func _get_unread_count() -> int:
	var count = 0
	return count


func _connect_update_signals() -> void:
	pass


func _on_button_clicked() -> void:
	Global.close_navbar.emit()
	Global.open_settings.emit()
	Global.send_haptic_feedback()


func _get_button_metric_name() -> String:
	return "settings"
