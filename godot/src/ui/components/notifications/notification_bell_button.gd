extends Button

signal bell_clicked

var _unread_count: int = 0

@onready var label_badge: Label = %Label_Badge
@onready var badge_container: PanelContainer = %Badge_Container


func _ready() -> void:
	pressed.connect(_on_pressed)

	# Connect to NotificationsManager signals
	NotificationsManager.new_notifications.connect(_on_notifications_updated)
	NotificationsManager.notifications_updated.connect(_on_notifications_updated)

	# Initial update
	_update_badge()


func _on_pressed() -> void:
	bell_clicked.emit()


func _on_notifications_updated(_notifications: Array = []) -> void:
	_update_badge()


func _update_badge() -> void:
	var notifications = NotificationsManager.get_notifications()
	_unread_count = 0

	for notif in notifications:
		if not notif.get("read", false):
			_unread_count += 1

	if _unread_count > 0:
		badge_container.visible = true
		if _unread_count > 99:
			label_badge.text = "99+"
		else:
			label_badge.text = str(_unread_count)
	else:
		badge_container.visible = false


func get_unread_count() -> int:
	return _unread_count
