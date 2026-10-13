extends StaticButton


func _get_unread_count() -> int:
	var count = 0
	return count


func _connect_update_signals() -> void:
	pass


func _on_button_clicked() -> void:
	# Same button script is used by the landscape navbar and the portrait full-screen menu switcher.
	# Landscape opens the docked side panel; portrait keeps the full-screen Discover screen.
	if Global.is_orientation_portrait():
		Global.open_discover.emit()
	else:
		Global.open_discover_panel.emit()
	Global.send_haptic_feedback()


func _get_button_metric_name() -> String:
	return "discover"
