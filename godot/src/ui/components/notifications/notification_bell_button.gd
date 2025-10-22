extends Button

signal bell_clicked

var _unread_count: int = 0
var _is_panel_open: bool = false

@onready var label_badge: Label = %Label_Badge
@onready var badge_container: PanelContainer = %Badge_Container

# Colors for button state
const COLOR_NORMAL = Color(1, 1, 1, 1)
const COLOR_ACTIVE = Color(0.4, 0.7, 1.0, 1)  # Light blue when active


func _ready() -> void:
	pressed.connect(_on_pressed)

	# Connect to NotificationsManager signals
	NotificationsManager.new_notifications.connect(_on_notifications_updated)
	NotificationsManager.notifications_updated.connect(_on_notifications_updated)

	# Initial update
	_update_badge()
	_update_button_color()


func _on_pressed() -> void:
	bell_clicked.emit()


func set_panel_open(is_open: bool) -> void:
	_is_panel_open = is_open
	_update_button_color()


func _update_button_color() -> void:
	if _is_panel_open:
		modulate = COLOR_ACTIVE
	else:
		modulate = COLOR_NORMAL


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
