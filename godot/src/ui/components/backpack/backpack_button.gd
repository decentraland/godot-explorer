extends AnimatedButton


func _get_unread_count() -> int:
	return 0


func _connect_update_signals() -> void:
	pass


func _on_button_clicked() -> void:
	Global.open_backpack.emit()


func _get_button_metric_name() -> String:
	return "backpack"
