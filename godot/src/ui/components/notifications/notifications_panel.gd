extends PanelContainer

signal panel_closed

const NotificationItemScene = preload(
	"res://src/ui/components/notifications/notification_item.tscn"
)

var _notification_items: Array[Control] = []

@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var notifications_list: VBoxContainer = %NotificationsList
@onready var label_empty_state: Label = %LabelEmptyState
@onready var button_mark_all_read: Button = %ButtonMarkAllRead
@onready var fade_overlay: TextureRect = %FadeOverlay


func _ready() -> void:
	# Ensure the panel blocks touch/mouse events from passing through
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
	# Only handle input when panel is visible
	if not visible:
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
		fade_overlay.visible = false
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
		button_mark_all_read.visible = true
	else:
		button_mark_all_read.visible = false

	# Create notification items
	for notif in notifications:
		var item = NotificationItemScene.instantiate()
		notifications_list.add_child(item)
		item.set_notification(notif)

		# Connect signals
		item.mark_as_read_clicked.connect(_async_on_notification_mark_as_read)
		item.notification_clicked.connect(_on_notification_clicked)

		_notification_items.append(item)

	# Update gradient visibility after items are added (next frame)
	await get_tree().process_frame
	_update_gradient_visibility()


func _on_new_notifications(notifications: Array) -> void:
	_display_notifications(notifications)


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


func _on_notification_clicked(_notification: Dictionary) -> void:
	# Close the panel when a notification is clicked
	hide_panel()
	panel_closed.emit()


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
	# Hide scroll container and button
	scroll_container.visible = false
	button_mark_all_read.visible = false
	fade_overlay.visible = false

	# Show custom message for guests
	label_empty_state.visible = true
	label_empty_state.text = "Sign in to get notifications!"


func _update_gradient_visibility() -> void:
	# Check if scrollbar is visible (content exceeds container height)
	var v_scroll = scroll_container.get_v_scroll_bar()
	if v_scroll:
		fade_overlay.visible = v_scroll.visible
	else:
		fade_overlay.visible = false
