extends PanelContainer

signal notification_clicked(notification: Dictionary)
signal mark_as_read_clicked(notification_id: String)

var notification_data: Dictionary = {}

@onready var notification_image: TextureRect = %NotificationImage
@onready var icon_texture: TextureRect = %IconTexture
@onready var image_container: Panel = %ImageContainer
@onready var label_title: Label = %LabelTitle
@onready var label_description: RichTextLabel = %LabelDescription
@onready var label_timestamp: Label = %LabelTimestamp
@onready var unread_dot: Panel = %UnreadDot


func _ready() -> void:
	gui_input.connect(_on_gui_input)


func set_notification(notification: Dictionary) -> void:
	notification_data = notification
	_update_ui()


func _update_ui() -> void:
	if notification_data.is_empty():
		return

	# Get notification type and metadata
	var notif_type = notification_data.get("type", "")
	var metadata: Dictionary = notification_data.get("metadata", {}) if "metadata" in notification_data else {}

	# Generate header (title) and title (description) using helper
	label_title.text = NotificationTextHelper.get_notification_header(notif_type, metadata)
	label_description.text = NotificationTextHelper.get_notification_title(notif_type, metadata)

	# Set timestamp
	if "timestamp" in notification_data:
		var timestamp: int = int(notification_data["timestamp"])
		label_timestamp.text = _format_timestamp(timestamp)

	# Show/hide unread dot
	var is_read: bool = notification_data.get("read", false)
	unread_dot.visible = not is_read

	# Apply profile-style background for friend notifications first (before loading images)
	_apply_friend_notification_styling(notif_type, metadata)

	# Load notification image (main image)
	_load_notification_image()

	# Set icon based on notification type (small icon overlay)
	_set_icon_for_type(notif_type)


func _load_notification_image() -> void:
	var image_url = ""

	# Try to get image from metadata based on notification type
	if "metadata" in notification_data and notification_data["metadata"] is Dictionary:
		var metadata: Dictionary = notification_data["metadata"]

		# Check for different image sources based on notification type
		var notif_type = notification_data.get("type", "")

		match notif_type:
			"community_invite_received":
				image_url = metadata.get("thumbnailUrl", "")
			"badge_granted":
				image_url = metadata.get("image", "")
			"social_service_friendship_request":
				# Use sender's profile image
				if "sender" in metadata and metadata["sender"] is Dictionary:
					image_url = metadata["sender"].get("profileImageUrl", "")
			_:
				# Try generic image field
				image_url = metadata.get("image", metadata.get("thumbnailUrl", ""))

	# If we have a URL, load it
	if not image_url.is_empty():
		_async_load_image_from_url(image_url)
	else:
		# Use default notification image based on type
		_set_default_notification_image(notification_data.get("type", ""))


func _async_load_image_from_url(url: String) -> void:
	# Use Global.content_provider to fetch texture
	var hash = _get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(hash, url)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		_set_default_notification_image(notification_data.get("type", ""))
		return

	notification_image.texture = result.texture


func _get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]  # Return the last part

	# Convert URL to a hexadecimal hash
	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		context.update(url.to_utf8_buffer())
		var url_hash: PackedByteArray = context.finish()
		return url_hash.hex_encode()

	return "temp-file"


func _set_default_notification_image(_notif_type: String) -> void:
	# Use the default notification image as fallback when no specific image URL is provided
	# The type-specific images (Reward, Proposal, etc.) are for the small icon overlay, not the main image
	var image_path := "res://assets/ui/notifications/DefaultNotification.png"

	if ResourceLoader.exists(image_path):
		notification_image.texture = load(image_path)


func _set_icon_for_type(notif_type: String) -> void:
	# Map notification types to small icons (overlay)
	var icon_path := ""

	match notif_type:
		"item_sold", "bid_accepted", "bid_received", "royalties_earned":
			icon_path = "res://assets/ui/notifications/RewardNotification.png"
		"governance_announcement", "governance_proposal_enacted", "governance_voting_ended":
			icon_path = "res://assets/ui/notifications/ProposalFinishedNotification.png"
		"governance_coauthor_requested":
			icon_path = "res://assets/ui/notifications/CoauthorNotification.png"
		"land":
			icon_path = "res://assets/ui/notifications/LandRentedNotification.png"
		"worlds_access_restored":
			icon_path = "res://assets/ui/notifications/WorldAccessRestoredNotification.png"
		"worlds_access_restricted", "worlds_missing_resources":
			icon_path = "res://assets/ui/notifications/WorldUnaccessibleNotification.png"
		"worlds_permission_granted", "worlds_permission_revoked":
			icon_path = "res://assets/ui/notifications/WorldAccessRestoredNotification.png"
		"social_service_friendship_request":
			icon_path = "res://assets/ui/notifications/FriendNotification.png"
		"community_invite_received":
			icon_path = "res://assets/ui/notifications/DefaultNotification.png"
		"badge_granted":
			icon_path = "res://assets/ui/notifications/RewardNotification.png"
		"credits_reminder_do_not_miss_out":
			icon_path = "res://assets/ui/notifications/RewardNotification.png"
		_:
			icon_path = "res://assets/ui/notifications/DefaultNotification.png"

	if ResourceLoader.exists(icon_path):
		icon_texture.texture = load(icon_path)


func _apply_friend_notification_styling(notif_type: String, metadata: Dictionary) -> void:
	# Only apply to friend notifications
	if notif_type != "social_service_friendship_request" and notif_type != "social_service_friendship_accepted":
		return

	if "sender" not in metadata or not metadata["sender"] is Dictionary:
		return

	var sender_name = metadata["sender"].get("name", "")
	if sender_name.is_empty():
		return

	# Get the avatar color for this username
	var color = _get_avatar_color(sender_name)
	if color == Color.WHITE:  # Default fallback
		return

	# Load and duplicate the avatar profile style resource
	var style_box_template = load("res://src/ui/components/notifications/avatar_profile_style.tres")
	if style_box_template == null:
		return

	var style_box = style_box_template.duplicate()
	style_box.bg_color = color.darkened(0.6)  # Darker background
	style_box.border_color = color  # Border is the avatar color
	# Same properties as profile button to make it completely circular
	style_box.border_width_left = 4
	style_box.border_width_top = 4
	style_box.border_width_right = 4
	style_box.border_width_bottom = 4
	style_box.corner_radius_top_left = 9999
	style_box.corner_radius_top_right = 9999
	style_box.corner_radius_bottom_right = 9999
	style_box.corner_radius_bottom_left = 9999
	style_box.corner_detail = 16
	style_box.anti_aliasing_size = 0.5

	# Apply the style to the image container
	if image_container is Panel:
		image_container.add_theme_stylebox_override("panel", style_box)


func _get_avatar_color(username: String) -> Color:
	if Global.avatars == null:
		return Color.WHITE

	var avatars = Global.avatars.get_avatars()
	if avatars.is_empty():
		return Color.WHITE

	return avatars[0].get_nickname_color(username)


func _format_timestamp(timestamp_ms: int) -> String:
	var timestamp_sec = timestamp_ms / 1000
	var current_time = Time.get_unix_time_from_system()
	var diff = current_time - timestamp_sec

	if diff < 60:
		return "just now"

	if diff < 3600:
		var minutes = int(diff / 60)
		if minutes == 1:
			return "1 minute ago"
		return "%d minutes ago" % minutes

	if diff < 86400:
		var hours = int(diff / 3600)
		if hours == 1:
			return "1 hour ago"
		return "%d hours ago" % hours

	if diff < 172800:  # Less than 2 days (86400 * 2)
		return "yesterday"

	# For all older notifications, show days ago
	var days = int(diff / 86400)
	return "%d days ago" % days


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			notification_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			# Mark as read when clicked
			if "id" in notification_data:
				mark_as_read_clicked.emit(notification_data["id"])
	elif event is InputEventScreenTouch:
		if event.pressed:
			notification_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			# Mark as read when clicked
			if "id" in notification_data:
				mark_as_read_clicked.emit(notification_data["id"])
