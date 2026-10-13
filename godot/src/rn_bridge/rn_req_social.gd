class_name RnReqSocial
extends RefCounted
## friends.* requests, backed by Global.social_service and the nearby avatars.

const PAGE_SIZE := 100
const NEARBY_LIMIT := 30
const ACTIONS := {
	"friends.accept": "accept_friend_request",
	"friends.reject": "reject_friend_request",
	"friends.cancel": "cancel_friend_request",
	"friends.unblock": "unblock_user",
	"friends.block": "block_user",
}


func async_handle(method: String, params: Dictionary) -> Dictionary:
	match method:
		"friends.list":
			var limit := int(params.get("limit", PAGE_SIZE))
			var promise: Promise = Global.social_service.get_friends(
				limit, int(params.get("offset", 0)), -1
			)
			return await RnReqUtil.async_unwrap(
				promise, func(data: Variant) -> Dictionary: return {"friends": _friends(data)}
			)
		"friends.requests":
			return await _async_requests()
		"friends.nearby":
			return await _async_nearby()
		"friends.send":
			var address := RnReqUtil.str_param(params, "address")
			if address.is_empty():
				return RnReqUtil.fail("address required")
			var message := str(params.get("message", ""))
			return await RnReqUtil.async_unwrap(
				Global.social_service.send_friend_request(address, message), _empty
			)
	if ACTIONS.has(method):
		var address := RnReqUtil.str_param(params, "address")
		if address.is_empty():
			return RnReqUtil.fail("address required")
		var promise: Promise = Global.social_service.call(ACTIONS[method], address)
		return await RnReqUtil.async_unwrap(promise, _empty)
	return RnReqUtil.fail("unknown method " + method)


func _empty(_data: Variant) -> Dictionary:
	return {}


func _async_requests() -> Dictionary:
	var received: Variant = await RnReqUtil.async_unwrap(
		Global.social_service.get_pending_requests(PAGE_SIZE, 0), _friends
	)
	if not received["ok"]:
		return received
	var sent: Variant = await RnReqUtil.async_unwrap(
		Global.social_service.get_sent_requests(PAGE_SIZE, 0), _friends
	)
	if not sent["ok"]:
		return sent
	return RnReqUtil.ok({"received": received["result"], "sent": sent["result"]})


func _async_nearby() -> Dictionary:
	var users: Array = []
	for avatar in Global.avatars.get_avatars():
		if users.size() >= NEARBY_LIMIT:
			break
		var address: String = avatar.avatar_id if avatar != null else ""
		if address.is_empty() or Global.social_blacklist.is_blocked(address):
			continue
		users.append(await RnProfileLookup.async_friend_entry(address))
	return RnReqUtil.ok({"users": users})


static func _friends(data: Variant) -> Array:
	var out: Array = []
	for item: Dictionary in data if data is Array else []:
		(
			out
			. append(
				{
					"address": str(item.get("address", "")),
					"name": str(item.get("name", "")),
					"has_claimed_name": bool(item.get("has_claimed_name", false)),
					"profile_picture_url": str(item.get("profile_picture_url", "")),
					"friendship_id": str(item.get("friendship_id", "")),
				}
			)
		)
	return out
