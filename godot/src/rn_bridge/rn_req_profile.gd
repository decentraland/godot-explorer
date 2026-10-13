class_name RnReqProfile
extends RefCounted
## profile.* and account.* requests: reuse DclUserProfile, ProfileService and the Global sign-out flow.

# RN field -> DclUserProfile accessor suffix (get_<x> / set_<x>).
const TEXT_FIELDS := {
	"name": "name",
	"description": "description",
	"country": "country",
	"pronouns": "pronouns",
	"gender": "gender",
	"sexual_orientation": "sexual_orientation",
	"relationship_status": "relationship_status",
	"employment_status": "employment_status",
	"profession": "profession",
	"real_name": "real_name",
	"hobbies": "hobbies",
}


func async_handle(method: String, params: Dictionary) -> Dictionary:
	match method:
		"profile.get":
			return await _async_get(RnReqUtil.str_param(params, "address"))
		"profile.names":
			return await _async_names()
		"profile.deploy":
			return await _async_deploy(params.get("fields", {}))
		"account.sign_out":
			Global.sign_out.call_deferred()
			return RnReqUtil.ok()
		"account.delete":
			return await _async_delete()
	return RnReqUtil.fail("unknown method " + method)


func _async_get(address: String) -> Dictionary:
	var own: DclUserProfile = Global.player_identity.get_mutable_profile()
	var own_address := String(Global.player_identity.get_address_str()).to_lower()
	var profile: Variant = own
	if not address.is_empty() and address.to_lower() != own_address:
		profile = await PromiseUtils.async_awaiter(Global.content_provider.fetch_profile(address))
		if profile is PromiseError:
			return RnReqUtil.fail(profile.get_error())
	if profile == null:
		return RnReqUtil.fail("profile not available")
	return RnReqUtil.ok(_to_dict(profile))


static func _to_dict(profile: DclUserProfile) -> Dictionary:
	var out := {
		"address": profile.get_ethereum_address(),
		"version": profile.get_profile_version(),
		"has_claimed_name": profile.has_claimed_name(),
		"links": RnReqUtil.json_safe(profile.get_links()),
		"profile_picture_url": profile.get_avatar().get_snapshots_face_url(),
	}
	for field: String in TEXT_FIELDS:
		out[field] = profile.call("get_" + TEXT_FIELDS[field])
	return out


func _async_names() -> Dictionary:
	var response: NamesRequest.LambdaNamesResponse = await NamesRequest.async_request_all_names()
	if response == null:
		return RnReqUtil.fail("could not load names")
	var names: Array = []
	for item in response.elements:
		names.append(item.name)
	return RnReqUtil.ok({"names": names})


func _async_deploy(fields: Variant) -> Dictionary:
	var profile: DclUserProfile = Global.player_identity.get_mutable_profile()
	if profile == null or not fields is Dictionary:
		return RnReqUtil.fail("profile not available")
	for field: String in TEXT_FIELDS:
		if fields.has(field):
			profile.call("set_" + TEXT_FIELDS[field], str(fields[field]))
	if fields.has("has_claimed_name"):
		profile.set_has_claimed_name(bool(fields["has_claimed_name"]))
	if fields.get("links") is Array:
		var links: Array[Dictionary] = []
		for link: Variant in fields["links"]:
			if link is Dictionary:
				links.append({"title": str(link.get("title", "")), "url": str(link.get("url", ""))})
		profile.set_links(links)
	return await RnReqUtil.async_unwrap(ProfileService.async_deploy_profile(profile), _empty)


func _async_delete() -> Dictionary:
	var response: Variant = await Global.async_signed_fetch(
		DclUrls.account_deletion(), HTTPClient.METHOD_POST, ""
	)
	if response is PromiseError:
		return RnReqUtil.fail(response.get_error())
	var data: Variant = response.get_string_response_as_json()
	if not data is Dictionary or not data.get("ok", false):
		return RnReqUtil.fail(
			str(data.get("error", "invalid response")) if data is Dictionary else "invalid response"
		)
	return RnReqUtil.ok()


func _empty(_data: Variant) -> Dictionary:
	return {}
