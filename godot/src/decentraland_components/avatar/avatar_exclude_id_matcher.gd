class_name AvatarExcludeIdMatcher


# Decides whether `avatar_id` is present in `exclude_ids` for the purposes of
# AvatarModifierArea visibility. Treats Ethereum addresses case-insensitively
# and tolerates a missing `0x` prefix on either side: profiles delivered over
# comms may carry the checksummed form, while scenes commonly push lowercase
# addresses. See issue #2166.
static func is_excluded(avatar_id: String, exclude_ids) -> bool:
	if avatar_id.is_empty():
		return false
	var normalized := _normalize(avatar_id)
	for raw in exclude_ids:
		if _normalize(String(raw)) == normalized:
			return true
	return false


static func _normalize(addr: String) -> String:
	var lower := addr.to_lower()
	if not lower.begins_with("0x"):
		lower = "0x" + lower
	return lower
