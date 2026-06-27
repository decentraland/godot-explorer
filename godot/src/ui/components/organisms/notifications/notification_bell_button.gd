extends StaticButton


func _get_unread_count() -> int:
	var notifications = Services.notifications_manager.get_notifications()
	var count = 0

	for notif in notifications:
		if not notif.get("read", false):
			count += 1

	return count


func _connect_update_signals() -> void:
	# Connect to NotificationsManager signals
	Services.notifications_manager.new_notifications.connect(_on_notifications_updated)
	Services.notifications_manager.notifications_updated.connect(_on_notifications_updated)


func _on_button_clicked() -> void:
	Global.close_menu.emit()
	Global.open_notifications_panel.emit()
	Global.send_haptic_feedback()


func _get_button_metric_name() -> String:
	return "notification_bell"
