extends PanelContainer

signal toast_clicked(notification: Dictionary)
signal toast_closed

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2

var notification_data: Dictionary = {}
var _timer: Timer

@onready var notification_image: TextureRect = %NotificationImage
@onready var icon_texture: TextureRect = %IconTexture
@onready var image_container: Panel = %ImageContainer
@onready var label_title: Label = %LabelTitle
@onready var label_description: RichTextLabel = %LabelDescription


func _ready() -> void:
	gui_input.connect(_on_gui_input)

	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)

	# Start above screen
	position.y = -size.y


func show_notification(notification: Dictionary) -> void:
	notification_data = notification
	_update_ui()

	# Animate slide in from top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", 20.0, SLIDE_IN_DURATION)

	# Start auto-hide timer
	_timer.start(DISPLAY_DURATION)


func _update_ui() -> void:
	if notification_data.is_empty():
		return

	# Get notification type and metadata
	var notif_type = notification_data.get("type", "")
	var metadata: Dictionary = notification_data.get("metadata", {}) if "metadata" in notification_data else {}

	# Generate header (title) and title (description) using helper
	label_title.text = NotificationTextHelper.get_notification_header(notif_type, metadata)
	label_description.text = NotificationTextHelper.get_notification_title(notif_type, metadata)

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


func _on_timer_timeout() -> void:
	async_hide_toast()


func async_hide_toast() -> void:
	# Animate slide out to top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -size.y - 20.0, SLIDE_OUT_DURATION)
	await tween.finished
	toast_closed.emit()  # Emit signal before freeing
	queue_free()


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			toast_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			async_hide_toast()
	elif event is InputEventScreenTouch:
		if event.pressed:
			toast_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			async_hide_toast()
