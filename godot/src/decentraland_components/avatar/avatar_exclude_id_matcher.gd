class_name AvatarExcludeIdMatcher


# Decides whether `avatar_id` is present in `exclude_ids` for the purposes of
# AvatarModifierArea visibility. Ids compare case-insensitively, as on desktop
# (comms may deliver a checksummed address, scenes push lowercase ones), and an
# address also matches without its `0x` prefix. See issue #2166.
static func is_excluded(avatar_id: String, exclude_ids) -> bool:
	if avatar_id.is_empty():
		return false
	var normalized := _normalize(avatar_id)
	for raw in exclude_ids:
		if _normalize(String(raw)) == normalized:
			return true
	return false


## Whether `id` is an Ethereum address: `0x` and 40 hex digits, any casing.
static func is_address(id: String) -> bool:
	return (
		id.length() == 42 and id.to_lower().begins_with("0x") and id.substr(2).is_valid_hex_number()
	)


static func _normalize(id: String) -> String:
	var lower := id.to_lower()
	# Only an address gains the prefix: a scene id like "npc-1" is compared as sent.
	if lower.length() == 40 and lower.is_valid_hex_number():
		return "0x" + lower
	return lower
