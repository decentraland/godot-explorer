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


## Sanitize notification text for iOS compatibility
## Optionally removes emojis and/or normalizes accented characters
## By default, preserves both emojis and accents
## @param text: The text to sanitize
## @param remove_emojis: If true, removes emojis. If false, preserves them (default: false)
## @param normalize_accents: If true, normalizes accented characters. If false, preserves them (default: false)
## @return: Sanitized text
static func sanitize_notification_text(
	text: String, remove_emojis: bool = false, normalize_accents: bool = false
) -> String:
	if text.is_empty():
		return text

	var sanitized = text

	# Optionally normalize accented characters to their base form
	if normalize_accents:
		var accent_map = {
			"á": "a",
			"à": "a",
			"ä": "a",
			"â": "a",
			"ã": "a",
			"å": "a",
			"ā": "a",
			"ă": "a",
			"ą": "a",
			"Á": "A",
			"À": "A",
			"Ä": "A",
			"Â": "A",
			"Ã": "A",
			"Å": "A",
			"Ā": "A",
			"Ă": "A",
			"Ą": "A",
			"é": "e",
			"è": "e",
			"ë": "e",
			"ê": "e",
			"ē": "e",
			"ĕ": "e",
			"ė": "e",
			"ę": "e",
			"ě": "e",
			"É": "E",
			"È": "E",
			"Ë": "E",
			"Ê": "E",
			"Ē": "E",
			"Ĕ": "E",
			"Ė": "E",
			"Ę": "E",
			"Ě": "E",
			"í": "i",
			"ì": "i",
			"ï": "i",
			"î": "i",
			"ī": "i",
			"ĭ": "i",
			"į": "i",
			"ı": "i",
			"Í": "I",
			"Ì": "I",
			"Ï": "I",
			"Î": "I",
			"Ī": "I",
			"Ĭ": "I",
			"Į": "I",
			"İ": "I",
			"ó": "o",
			"ò": "o",
			"ö": "o",
			"ô": "o",
			"õ": "o",
			"ø": "o",
			"ō": "o",
			"ŏ": "o",
			"ő": "o",
			"Ó": "O",
			"Ò": "O",
			"Ö": "O",
			"Ô": "O",
			"Õ": "O",
			"Ø": "O",
			"Ō": "O",
			"Ŏ": "O",
			"Ő": "O",
			"ú": "u",
			"ù": "u",
			"ü": "u",
			"û": "u",
			"ū": "u",
			"ŭ": "u",
			"ů": "u",
			"ű": "u",
			"ų": "u",
			"Ú": "U",
			"Ù": "U",
			"Ü": "U",
			"Û": "U",
			"Ū": "U",
			"Ŭ": "U",
			"Ů": "U",
			"Ű": "U",
			"Ų": "U",
			"ý": "y",
			"ỳ": "y",
			"ÿ": "y",
			"ŷ": "y",
			"ȳ": "y",
			"ỹ": "y",
			"Ý": "Y",
			"Ỳ": "Y",
			"Ÿ": "Y",
			"Ŷ": "Y",
			"Ȳ": "Y",
			"Ỹ": "Y",
			"ñ": "n",
			"Ñ": "N",
			"ç": "c",
			"Ç": "C",
			"ß": "ss",
			"æ": "ae",
			"Æ": "AE",
			"œ": "oe",
			"Œ": "OE"
		}

		for accent_char in accent_map.keys():
			sanitized = sanitized.replace(accent_char, accent_map[accent_char])

	# Optionally remove emojis if requested
	if remove_emojis:
		var result = ""
		for i in range(sanitized.length()):
			var char_code = sanitized.unicode_at(i)

			# Check if character is in emoji ranges
			var is_emoji = (
				# Miscellaneous Symbols and Pictographs (U+1F300 to U+1F9FF)
				(char_code >= 0x1F300 and char_code <= 0x1F9FF)
				# Miscellaneous Symbols (U+2600 to U+26FF)
				or (char_code >= 0x2600 and char_code <= 0x26FF)
				# Dingbats (U+2700 to U+27BF)
				or (char_code >= 0x2700 and char_code <= 0x27BF)
				# Variation Selectors (U+FE00 to U+FE0F)
				or (char_code >= 0xFE00 and char_code <= 0xFE0F)
				# Supplemental Symbols and Pictographs (U+1F900 to U+1F9FF)
				or (char_code >= 0x1F900 and char_code <= 0x1F9FF)
				# Regional Indicator Symbols (U+1F1E0 to U+1F1FF)
				or (char_code >= 0x1F1E0 and char_code <= 0x1F1FF)
				# Zero Width Joiner (U+200D)
				or (char_code == 0x200D)
				# Combining Enclosing Keycap (U+20E3)
				or (char_code == 0x20E3)
				# Symbols and Pictographs Extended-A (U+1FA00 to U+1FAFF)
				or (char_code >= 0x1FA00 and char_code <= 0x1FAFF)
				# Emoticons (U+1F600 to U+1F64F)
				or (char_code >= 0x1F600 and char_code <= 0x1F64F)
				# Transport and Map Symbols (U+1F680 to U+1F6FF)
				or (char_code >= 0x1F680 and char_code <= 0x1F6FF)
				# Additional emoji ranges
				or (char_code >= 0x1F000 and char_code <= 0x1F02F)  # Mahjong Tiles
				or (char_code >= 0x1F0A0 and char_code <= 0x1F0FF)
			)  # Playing Cards

			# Keep all non-emoji characters (including normalized accented chars and emojis if not removing)
			if not is_emoji:
				# Keep printable ASCII and common whitespace, plus all Unicode characters that aren't emojis
				if (
					(char_code >= 32 and char_code <= 126)
					or char_code == 9
					or char_code == 10
					or char_code == 13
					or (char_code > 127 and not is_emoji)
				):
					result += sanitized[i]

		return result

	# Return text with accents and emojis preserved (unless normalized/removed above)
	return sanitized
