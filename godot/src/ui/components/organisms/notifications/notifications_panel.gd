extends PanelContainer

signal panel_closed

const NotificationItemScene = preload(
	"res://src/ui/components/organisms/notifications/notification_item.tscn"
)

# Live items keyed by notification id, so a refresh reuses them (keeping their loaded thumbnail)
# instead of tearing everything down — a full rebuild cancels the newest item's in-flight image load.
var _items_by_id: Dictionary = {}

@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var notifications_list: VBoxContainer = %NotificationsList
@onready var button_mark_all_read: Button = %ButtonMarkAllRead
@onready var v_box_container_no_notifications: VBoxContainer = %VBoxContainer_NoNotifications
@onready var label_no_notifications: Label = %Label_NoNotifications


func _ready() -> void:
	# Ensure the panel blocks touch/mouse events from passing through when visible
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_process_input(true)

	button_mark_all_read.pressed.connect(_async_on_mark_all_read_pressed)

	# Connect to NotificationsManager signals
	NotificationsManager.new_notifications.connect(_on_new_notifications)
	NotificationsManager.notifications_updated.connect(_on_notifications_updated)
	NotificationsManager.notification_error.connect(_on_notification_error)

	# Initial load
	_refresh_notifications()


func _input(event: InputEvent) -> void:
	# Only handle input when panel is visible in tree
	if not is_visible_in_tree():
		return

	# Only process touch events (includes emulated touch from mouse)
	# Ignore mouse events to avoid duplicate processing with emulation enabled
	if not (event is InputEventScreenTouch or event is InputEventScreenDrag):
		return

	# Check if input is within the panel's rectangle
	var pos = event.position
	var rect = get_global_rect()
	var is_inside_panel = rect.has_point(pos)

	# Only release focus on touch press (not during drag) to prevent camera rotation
	# This allows ScrollContainer to handle drag events normally
	if is_inside_panel and event is InputEventScreenTouch and event.pressed:
		if Global.explorer_has_focus():
			Global.explorer_release_focus()


func _refresh_notifications() -> void:
	# Check if user is authenticated
	if not _is_user_authenticated():
		_show_guest_message()
		return

	var notifications = NotificationsManager.get_notifications()
	async_display_notifications(notifications)


func async_display_notifications(notifications: Array) -> void:
	# Empty state
	if notifications.size() == 0:
		_clear_items()
		v_box_container_no_notifications.visible = true
		scroll_container.visible = false
		button_mark_all_read.disabled = true
		return

	v_box_container_no_notifications.visible = false
	scroll_container.visible = true

	# Mark-all-read is enabled only while there's something unread.
	var unread_count = 0
	for notif in notifications:
		if not notif.get("read", false):
			unread_count += 1
	button_mark_all_read.disabled = unread_count == 0

	# Incremental update: reuse the item already showing each notification id (keeping its loaded
	# thumbnail) and only build items for genuinely new ids, then reorder. A full teardown would
	# cancel the newest item's in-flight image load — the "blank thumbnail" bug.
	var desired: Dictionary = {}
	for i in notifications.size():
		var notif = notifications[i]
		var id = str(notif.get("id", ""))
		desired[id] = true
		var item
		if _items_by_id.has(id):
			# Same notification: refresh only read state + timestamp; the thumbnail stays loaded.
			item = _items_by_id[id]
			item.call("refresh", notif)
		else:
			item = NotificationItemScene.instantiate()
			notifications_list.add_child(item)
			item.set_notification(notif)
			item.mark_as_read_clicked.connect(_async_on_notification_mark_as_read)
			item.notification_clicked.connect(_on_notification_clicked)
			_items_by_id[id] = item
		notifications_list.move_child(item, i)

	# Drop items whose notification is no longer present.
	for id in _items_by_id.keys():
		if not desired.has(id):
			_items_by_id[id].queue_free()
			_items_by_id.erase(id)


func _clear_items() -> void:
	for item in _items_by_id.values():
		item.queue_free()
	_items_by_id.clear()


func _on_new_notifications(notifications: Array) -> void:
	async_display_notifications(notifications)


func _on_notifications_updated() -> void:
	_refresh_notifications()


func _on_notification_error(error_message: String) -> void:
	printerr("NotificationsPanel: Error - ", error_message)


func _async_on_notification_mark_as_read(notification_id: String) -> void:
	var ids = PackedStringArray([notification_id])
	var promise = NotificationsManager.mark_as_read(ids)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("NotificationsPanel: Failed to mark as read - ", result.get_error())


func _async_on_mark_all_read_pressed() -> void:
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


func _on_notification_clicked(notification: Dictionary) -> void:
	# Close the panel when a notification is clicked
	hide_panel()
	panel_closed.emit()

	# Collapse the navbar for notifications that navigate away (events, rewards); friend ones keep it
	# open since they open the Friends panel inside the navbar.
	var notif_type: String = notification.get("type", "")
	if (
		notif_type
		not in ["social_service_friendship_request", "social_service_friendship_accepted"]
	):
		Global.close_navbar.emit()


func show_panel() -> void:
	show()
	_refresh_notifications()


func hide_panel() -> void:
	hide()


func _is_user_authenticated() -> bool:
	var player_identity = Global.get_player_identity()
	if player_identity == null:
		return false
	var address = player_identity.get_address_str()
	return not address.is_empty()


func _show_guest_message() -> void:
	# Hide the scroll container; keep the button visible but disabled.
	_clear_items()
	scroll_container.visible = false
	button_mark_all_read.disabled = true

	# Show custom message for guests
	v_box_container_no_notifications.visible = true
	label_no_notifications.text = tr("NOTIFICATIONS_SIGN_IN_TO_GET_NOTIFICATIONS")
