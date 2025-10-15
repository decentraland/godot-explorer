extends PanelContainer

signal panel_closed

const NotificationItemScene = preload(
	"res://src/ui/components/notifications/notification_item.tscn"
)

var _notification_items: Array[Control] = []

@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var notifications_list: VBoxContainer = %NotificationsList
@onready var label_unread_count: Label = %LabelUnreadCount
@onready var label_empty_state: Label = %LabelEmptyState
@onready var button_close: Button = %ButtonClose
@onready var button_mark_all_read: Button = %ButtonMarkAllRead


func _ready() -> void:
	button_close.pressed.connect(_on_close_pressed)
	button_mark_all_read.pressed.connect(_on_mark_all_read_pressed)

	# Connect to NotificationsManager signals
	NotificationsManager.new_notifications.connect(_on_new_notifications)
	NotificationsManager.notifications_updated.connect(_on_notifications_updated)
	NotificationsManager.notification_error.connect(_on_notification_error)

	# Initial load
	_refresh_notifications()


func _refresh_notifications() -> void:
	var notifications = NotificationsManager.get_notifications()
	_display_notifications(notifications)


func _display_notifications(notifications: Array) -> void:
	# Clear existing items
	for item in _notification_items:
		item.queue_free()
	_notification_items.clear()

	# Show empty state if no notifications
	if notifications.size() == 0:
		label_empty_state.visible = true
		scroll_container.visible = false
		button_mark_all_read.visible = false
		label_unread_count.text = "No notifications"
		return

	label_empty_state.visible = false
	scroll_container.visible = true

	# Count unread notifications
	var unread_count = 0
	for notif in notifications:
		if not notif.get("read", false):
			unread_count += 1

	# Update header
	if unread_count > 0:
		label_unread_count.text = "%d unread" % unread_count
		button_mark_all_read.visible = true
	else:
		label_unread_count.text = "All caught up!"
		button_mark_all_read.visible = false

	# Create notification items
	for notif in notifications:
		var item = NotificationItemScene.instantiate()
		notifications_list.add_child(item)
		item.set_notification(notif)

		# Connect signals
		item.mark_as_read_clicked.connect(_on_notification_mark_as_read)
		item.notification_clicked.connect(_on_notification_clicked)

		_notification_items.append(item)


func _on_new_notifications(notifications: Array) -> void:
	_display_notifications(notifications)


func _on_notifications_updated() -> void:
	_refresh_notifications()


func _on_notification_error(error_message: String) -> void:
	printerr("NotificationsPanel: Error - ", error_message)


func _on_notification_mark_as_read(notification_id: String) -> void:
	var ids = PackedStringArray([notification_id])
	var promise = NotificationsManager.mark_as_read(ids)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("NotificationsPanel: Failed to mark as read - ", result.get_error())


func _on_mark_all_read_pressed() -> void:
	var notifications = NotificationsManager.get_notifications()
	var unread_ids: Array[String] = []

	for notif in notifications:
		if not notif.get("read", false) and "id" in notif:
			unread_ids.append(notif["id"])

	if unread_ids.size() == 0:
		return

	var ids = PackedStringArray(unread_ids)
	var promise = NotificationsManager.mark_as_read(ids)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("NotificationsPanel: Failed to mark all as read - ", result.get_error())


func _on_notification_clicked(_notification: Dictionary) -> void:
	pass


func _on_close_pressed() -> void:
	panel_closed.emit()
	hide()


func show_panel() -> void:
	show()
	_refresh_notifications()


func hide_panel() -> void:
	hide()
