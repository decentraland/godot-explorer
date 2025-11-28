class_name NotificationUtils
extends RefCounted

## Utility functions for notification system
## All functions are static and can be called without instantiation


## Generate random notification text for events
## @param event_name: Name of the event to generate text for
## @return: Dictionary with "title" and "body" keys containing random text
static func generate_event_notification_text(event_name: String) -> Dictionary:
	# Title templates (5 options)
	var title_templates = [
		"{EventName} is Live!",
		"{EventName} Started!",
		"{EventName} is Starting!",
		"{EventName} is ON!",
		"{EventName} Begins!"
	]

	# Description options (7 options)
	var descriptions = [
		"Dress up and be the soul of the party",
		"Your gang awaits, jump in and party on!",
		"Join and meet your people",
		"Don't miss out on the action!",
		"Gather your crew and make your mark!",
		"Join your friends and meet people!",
		"Hop in and don't miss a beat!"
	]

	# Select random title and description
	var random_title_template = title_templates[randi() % title_templates.size()]
	var random_description = descriptions[randi() % descriptions.size()]

	# Substitute event name in title
	var title = random_title_template.replace("{EventName}", event_name)

	return {"title": title, "body": random_description}


## Get hash from URL for content provider
## @param url: The URL to hash
## @return: Hash string for content provider lookup
static func get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]

	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		context.update(url.to_utf8_buffer())
		var url_hash: PackedByteArray = context.finish()
		return url_hash.hex_encode()

	return "temp-file"


## Check if user is authenticated (not a guest)
## @return: true if user has a valid address
static func is_user_authenticated() -> bool:
	if not Global.player_identity:
		return false
	var address = Global.player_identity.get_address_str()
	return not address.is_empty()
