extends Control

## NotificationContent
##
## Shared component for displaying notification content.
## Used by both NotificationItem and NotificationToast to ensure consistent styling.
##
## The image area holds three pre-built, mutually-exclusive nodes (added in the scene, styled from
## the node) and this script only toggles which one is visible and feeds it data — so we never
## manipulate corners/borders at runtime:
##   - ProfilePictureFriend (round avatar, reused from social_item) for friend notifications
##   - EventImage (rounded thumbnail, #E8B9FF outline) for events and any other type
##   - ItemImage (rounded thumbnail, rarity-coloured outline) for received items/rewards

# Fallback shown when a notification carries no image URL, so the framed slot is never empty.
const DEFAULT_IMAGE: Texture2D = preload("res://assets/ui/notifications/DefaultNotification.png")

var notification_data: Dictionary = {}

@onready var profile_picture_friend: ProfilePicture = %ProfilePictureFriend
@onready var event_image: AsyncImage = %EventImage
@onready var item_image: AsyncImage = %ItemImage
@onready var label_title: RichTextLabel = %LabelTitle
@onready var label_description: Label = %LabelDescription


func set_notification(notification: Dictionary) -> void:
	notification_data = notification
	_update_ui()


func _notification(what: int) -> void:
	# Title/description are assigned from code, so they don't auto-retranslate on a language change.
	if what == NOTIFICATION_TRANSLATION_CHANGED:
		_update_ui()


func _update_ui() -> void:
	if notification_data.is_empty():
		return

	var notif_type = notification_data.get("type", "")
	var metadata: Dictionary = (
		notification_data.get("metadata", {}) if "metadata" in notification_data else {}
	)

	# Title carries only the subject (coloured player name / event name / item label); the node owns
	# the font weight, so the helper adds only the [color] tag when needed.
	label_title.text = NotificationTextHelper.get_notification_header(notif_type, metadata)
	label_description.text = NotificationTextHelper.get_notification_title(notif_type, metadata)

	_update_image(notif_type, metadata)


## Shows the one image node that matches the notification type and hides the other two.
func _update_image(notif_type: String, metadata: Dictionary) -> void:
	profile_picture_friend.hide()
	event_image.hide()
	item_image.hide()

	match notif_type:
		"social_service_friendship_request", "social_service_friendship_accepted":
			_show_friend(metadata)
		"reward_assignment", "reward_in_progress", "badge_granted":
			_show_item(metadata)
		_:
			# Events and every other type: a rounded thumbnail from whatever image the server sent.
			_show_event(metadata)


## Friend: round avatar via the shared ProfilePicture (owns its own circular mask + name colour).
func _show_friend(metadata: Dictionary) -> void:
	profile_picture_friend.show()
	var data := SocialItemData.new()
	if "sender" in metadata and metadata["sender"] is Dictionary:
		var sender: Dictionary = metadata["sender"]
		data.name = sender.get("name", "")
		data.address = sender.get("address", "")
		data.profile_picture_url = sender.get("profileImageUrl", "")
		data.has_claimed_name = sender.get("hasClaimedName", false)
	profile_picture_friend.async_update_profile_picture(data)


## Event / generic: rounded thumbnail with the scene's #E8B9FF outline.
func _show_event(metadata: Dictionary) -> void:
	event_image.show()
	var url: String = metadata.get(
		"image", metadata.get("thumbnailUrl", metadata.get("thumbnail", ""))
	)
	if url.is_empty():
		event_image.set_texture(DEFAULT_IMAGE)
	else:
		event_image.load_from_url(url)


## Received item/reward: rounded thumbnail with an outline coloured by its rarity.
func _show_item(metadata: Dictionary) -> void:
	item_image.show()
	item_image.border_color = _rarity_color(metadata.get("tokenRarity", ""))
	var url: String = metadata.get("tokenImage", metadata.get("image", ""))
	if url.is_empty():
		item_image.set_texture(DEFAULT_IMAGE)
	else:
		item_image.load_from_url(url)


func _rarity_color(rarity: String) -> Color:
	match rarity.to_lower():
		Wearables.ItemRarity.COMMON:
			return Wearables.RarityColor.COMMON
		Wearables.ItemRarity.UNCOMMON:
			return Wearables.RarityColor.UNCOMMON
		Wearables.ItemRarity.RARE:
			return Wearables.RarityColor.RARE
		Wearables.ItemRarity.EPIC:
			return Wearables.RarityColor.EPIC
		Wearables.ItemRarity.LEGENDARY:
			return Wearables.RarityColor.LEGENDARY
		Wearables.ItemRarity.MYTHIC:
			return Wearables.RarityColor.MYTHIC
		Wearables.ItemRarity.UNIQUE:
			return Wearables.RarityColor.UNIQUE
		Wearables.ItemRarity.EXOTIC:
			return Wearables.RarityColor.EXOTIC
		_:
			return Wearables.RarityColor.BASE
