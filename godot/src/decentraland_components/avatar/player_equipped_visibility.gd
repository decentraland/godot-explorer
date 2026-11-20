extends RefCounted


## Reports which of the player's profile wearables ended up rendered, for
## AvatarEquippedData.wearables[].visible. Skipped while an AvatarShape
## override is showing, since AvatarEquippedData keeps the profile outfit.
static func report(avatar: Node) -> void:
	if avatar.get_meta("avatar_shape_override", false):
		return
	var ids := PackedStringArray()
	for wearable in avatar.wearables_by_category.values():
		ids.append(wearable.get_id())
	Global.avatars.set_player_equipped_visibility(ids)
