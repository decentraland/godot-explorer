extends PanelContainer

signal notification_clicked(notification: Dictionary)
signal mark_as_read_clicked(notification_id: String)

var notification_data: Dictionary = {}

@onready var icon_texture: TextureRect = %IconTexture
@onready var label_title: Label = %LabelTitle
@onready var label_description: Label = %LabelDescription
@onready var label_timestamp: Label = %LabelTimestamp
@onready var button_mark_read: Button = %ButtonMarkRead


func _ready() -> void:
	button_mark_read.pressed.connect(_on_mark_as_read_pressed)
	gui_input.connect(_on_gui_input)


func set_notification(notification: Dictionary) -> void:
	notification_data = notification
	_update_ui()


func _update_ui() -> void:
	if notification_data.is_empty():
		return

	# Set title and description from metadata
	if "metadata" in notification_data and notification_data["metadata"] is Dictionary:
		var metadata: Dictionary = notification_data["metadata"]
		label_title.text = metadata.get("title", "Notification")
		label_description.text = metadata.get("description", "")
	else:
		label_title.text = notification_data.get("type", "notification")
		label_description.text = ""

	# Set timestamp
	if "timestamp" in notification_data:
		var timestamp: int = int(notification_data["timestamp"])
		label_timestamp.text = _format_timestamp(timestamp)

	# Show/hide mark as read button
	var is_read: bool = notification_data.get("read", false)
	button_mark_read.visible = not is_read

	# Set icon based on notification type
	_set_icon_for_type(notification_data.get("type", ""))


func _set_icon_for_type(notif_type: String) -> void:
	# Map notification types to icons
	var icon_path := ""

	match notif_type:
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			icon_path = "res://assets/ui/notifications/RewardNotification.png"
		"governance_announcement", "governance_proposal_enacted", "governance_voting_ended", "governance_coauthor_requested":
			icon_path = "res://assets/ui/notifications/ProposalFinishedNotification.png"
		"land":
			icon_path = "res://assets/ui/notifications/LandRentedNotification.png"
		"worlds_access_restored", "worlds_access_restricted", "worlds_missing_resources", "worlds_permission_granted", "worlds_permission_revoked":
			icon_path = "res://assets/ui/notifications/WorldAccessRestoredNotification.png"
		_:
			icon_path = "res://assets/ui/notifications/DefaultNotification.png"

	if ResourceLoader.exists(icon_path):
		icon_texture.texture = load(icon_path)


func _format_timestamp(timestamp_ms: int) -> String:
	var timestamp_sec = timestamp_ms / 1000
	var current_time = Time.get_unix_time_from_system()
	var diff = current_time - timestamp_sec

	if diff < 60:
		return "Just now"

	if diff < 3600:
		var minutes = int(diff / 60)
		return "%d min ago" % minutes

	if diff < 86400:
		var hours = int(diff / 3600)
		return "%d hour%s ago" % [hours, "s" if hours > 1 else ""]

	if diff < 604800:
		var days = int(diff / 86400)
		return "%d day%s ago" % [days, "s" if days > 1 else ""]

	# Format as date
	var timezone_info = Time.get_time_zone_from_system()
	var local_unix_time = timestamp_sec + (timezone_info.bias * 60)
	var datetime = Time.get_datetime_dict_from_unix_time(local_unix_time)
	return "%02d/%02d/%d" % [datetime.month, datetime.day, datetime.year]


func _on_mark_as_read_pressed() -> void:
	if "id" in notification_data:
		mark_as_read_clicked.emit(notification_data["id"])


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			notification_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
	elif event is InputEventScreenTouch:
		if event.pressed:
			notification_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
