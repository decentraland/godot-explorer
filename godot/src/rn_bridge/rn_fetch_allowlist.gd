class_name RnFetchAllowlist
extends RefCounted
## Hosts RN may reach through fetch.signed: the Decentraland services the client already talks to.


static func is_allowed(url: String) -> bool:
	if not url.begins_with("https://"):
		return false
	var host := _host(url)
	return not host.is_empty() and host in _allowed_hosts()


static func _allowed_hosts() -> PackedStringArray:
	var hosts := PackedStringArray()
	for base: String in [
		DclUrls.places_api(),
		DclUrls.destinations_api(),
		DclUrls.events_api(),
		DclUrls.mobile_events_api(),
		DclUrls.notifications_api(),
		DclUrls.credits_server(),
		DclUrls.account_deletion(),
		DclUrls.social_service(),
		DclUrls.mobile_bff(),
	]:
		hosts.append(_host(base))
	return hosts


## Returns "" for anything but a plain `host[:port]` authority (userinfo, backslashes, spaces).
static func _host(url: String) -> String:
	var rest := url.trim_prefix("https://")
	if "\\" in rest or " " in rest or "\t" in rest or "\n" in rest or "\r" in rest:
		return ""
	var authority := rest.split("/")[0].split("?")[0].split("#")[0]
	if "@" in authority:
		return ""
	var parts := authority.split(":")
	if parts.size() > 2 or (parts.size() == 2 and not parts[1].is_valid_int()):
		return ""
	return parts[0].to_lower()
