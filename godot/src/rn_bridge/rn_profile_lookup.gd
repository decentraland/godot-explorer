class_name RnProfileLookup
extends RefCounted
## Resolves an address to the public profile fields RN renders (nearby users, chat senders).


static func async_friend_entry(address: String) -> Dictionary:
	var profile: Variant = await _async_fetch(address)
	return {
		"address": address,
		"name": profile.get_name() if profile != null else address,
		"has_claimed_name": profile.has_claimed_name() if profile != null else false,
		"profile_picture_url":
		profile.get_avatar().get_snapshots_face_url() if profile != null else "",
	}


static func _async_fetch(address: String) -> Variant:
	var result: Variant = await PromiseUtils.async_awaiter(
		Global.content_provider.fetch_profile(address)
	)
	return null if result is PromiseError else result
