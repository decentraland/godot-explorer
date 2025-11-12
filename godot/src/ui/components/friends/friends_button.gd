extends AnimatedButton

signal friends_clicked


func _get_unread_count() -> int:
	#NEED TO SET FROM FRIENDS MANAGER
	var notifications = NotificationsManager.get_notifications()
	var count = 0
	
	for notif in notifications:
		if not notif.get("read", false):
			count += 1
	
	return count


func _connect_update_signals() -> void:
	# NEED TO CONNECT FROM FRIENDS MANAGER
	NotificationsManager.new_notifications.connect(_on_notifications_updated)
	NotificationsManager.notifications_updated.connect(_on_notifications_updated)


func _on_button_clicked() -> void:
	friends_clicked.emit()


func _get_button_metric_name() -> String:
	return "friends_button"
