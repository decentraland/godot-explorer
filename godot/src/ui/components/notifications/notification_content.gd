extends Control

## NotificationContent
##
## Shared component for displaying notification content.
## Used by both NotificationItem and NotificationToast to ensure consistent styling.

var notification_data: Dictionary = {}

@onready var notification_image: TextureRect = %NotificationImage
@onready var icon_texture: TextureRect = %IconTexture
@onready var image_container: Panel = %ImageContainer
@onready var label_title: RichTextLabel = %LabelTitle
@onready var label_description: RichTextLabel = %LabelDescription


func _ready() -> void:
	pass  # Initialization if needed


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
	# Wrap title in [b] tags for bold
	var title_text = NotificationTextHelper.get_notification_header(notif_type, metadata)
	label_title.text = "[b]" + title_text + "[/b]"
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
	# Use type-specific images for certain notification types
	var image_path := ""

	match _notif_type:
		"credits_reminder_do_not_miss_out", "item_sold", "bid_accepted", "bid_received", "royalties_earned":
			image_path = "res://assets/ui/notifications/RewardNotification.png"
		"governance_announcement", "governance_proposal_enacted", "governance_voting_ended":
			image_path = "res://assets/ui/notifications/ProposalFinishedNotification.png"
		_:
			image_path = "res://assets/ui/notifications/DefaultNotification.png"

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

	# Create a new StyleBoxFlat for the avatar with circular border
	var style_box = StyleBoxFlat.new()
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
	style_box.anti_aliasing = true
	style_box.anti_aliasing_size = 0.5

	# Apply the style to the image container
	if image_container is Panel:
		image_container.add_theme_stylebox_override("panel", style_box)


func _get_avatar_color(username: String) -> Color:
	var explorer = Global.get_explorer()
	if explorer == null or explorer.player == null:
		return Color.WHITE

	var player_avatar = explorer.player.avatar
	if player_avatar == null:
		return Color.WHITE

	return player_avatar.get_nickname_color(username)
