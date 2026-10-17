class_name RnReqMisc
extends RefCounted
## notifications.*, fetch.signed, places.*, clipboard.copy and screenshot.capture requests.

const METHODS := {
	"GET": HTTPClient.METHOD_GET,
	"POST": HTTPClient.METHOD_POST,
	"PUT": HTTPClient.METHOD_PUT,
	"PATCH": HTTPClient.METHOD_PATCH,
	"DELETE": HTTPClient.METHOD_DELETE,
}
const SCREENSHOT_PATH := "user://rn_screenshot.png"


func async_handle(method: String, params: Dictionary) -> Dictionary:
	match method:
		"notifications.list":
			return await RnReqUtil.async_unwrap(
				NotificationsManager.fetch_notifications(
					int(params.get("from", -1)),
					int(params.get("limit", -1)),
					bool(params.get("only_unread", false))
				),
				func(data: Variant) -> Dictionary:
					return {"notifications": RnReqUtil.json_safe(data)}
			)
		"notifications.read":
			var ids := PackedStringArray()
			var raw_ids: Variant = params.get("ids", [])
			for id: Variant in raw_ids if raw_ids is Array else []:
				ids.append(str(id))
			return await RnReqUtil.async_unwrap(
				NotificationsManager.mark_as_read(ids),
				func(data: Variant) -> Dictionary: return {"updated": int(data)}
			)
		"fetch.signed":
			return await _async_signed_fetch(params)
		"places.favorite":
			return await _async_favorite(params)
		"places.vote":
			return await _async_vote(params)
		"clipboard.copy":
			DisplayServer.clipboard_set(str(params.get("text", "")))
			return RnReqUtil.ok()
		"screenshot.capture":
			return await _async_screenshot()
	return RnReqUtil.fail("unknown method " + method)


func _async_signed_fetch(params: Dictionary) -> Dictionary:
	var url := RnReqUtil.str_param(params, "url")
	if not RnFetchAllowlist.is_allowed(url):
		return RnReqUtil.fail("url not allowed")
	var verb := str(params.get("method", "GET")).to_upper()
	if not METHODS.has(verb):
		return RnReqUtil.fail("method not allowed")
	var headers := _safe_headers(params.get("headers"))
	var response: Variant = await Global.async_signed_fetch(
		url, METHODS[verb], str(params.get("body", "")), "", headers
	)
	if response is PromiseError:
		return RnReqUtil.fail(response.get_error())
	return RnReqUtil.ok(
		{"status": 200, "body": RnReqUtil.json_safe(response.get_string_response_as_json())}
	)


## RN may add request headers but never override the identity signature or the target host.
static func _safe_headers(raw: Variant) -> Dictionary:
	var out := {}
	if not raw is Dictionary:
		return out
	for key: Variant in raw:
		var name := str(key).to_lower()
		if name.begins_with("x-identity") or name == "host" or name == "content-length":
			continue
		out[str(key)] = str(raw[key])
	return out


func _async_favorite(params: Dictionary) -> Dictionary:
	var response: Variant = await PlacesHelper.async_patch_favorite(
		RnReqUtil.str_param(params, "place_id"),
		bool(params.get("value", false)),
		bool(params.get("is_world", false))
	)
	return _signed_result(response)


func _async_vote(params: Dictionary) -> Dictionary:
	var like := PlacesHelper.LIKE.UNKNOWN
	match str(params.get("vote", "none")):
		"up":
			like = PlacesHelper.LIKE.YES
		"down":
			like = PlacesHelper.LIKE.NO
	var response: Variant = await PlacesHelper.async_patch_like(
		RnReqUtil.str_param(params, "place_id"), like, bool(params.get("is_world", false))
	)
	return _signed_result(response)


static func _signed_result(response: Variant) -> Dictionary:
	if response is PromiseError:
		return RnReqUtil.fail(response.get_error())
	return RnReqUtil.ok()


func _async_screenshot() -> Dictionary:
	await RenderingServer.frame_post_draw
	var image: Image = Engine.get_main_loop().root.get_viewport().get_texture().get_image()
	if image == null or image.save_png(SCREENSHOT_PATH) != OK:
		return RnReqUtil.fail("screenshot failed")
	return RnReqUtil.ok({"path": ProjectSettings.globalize_path(SCREENSHOT_PATH)})
