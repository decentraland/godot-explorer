class_name RnReqScene
extends RefCounted
## scene.current request: title, creator, parcel and realm of the scene the player is in.


func async_handle(method: String, _params: Dictionary) -> Dictionary:
	if method != "scene.current":
		return RnReqUtil.fail("unknown method " + method)
	return await _async_current()


func _async_current() -> Dictionary:
	var genesis := Realm.is_genesis_city(Global.realm.realm_url)
	var parcel: Vector2i = Global.scene_fetcher.current_position
	if parcel == SceneFetcher.INVALID_PARCEL:
		parcel = Vector2i.ZERO
	var title := ""
	var scene = Global.scene_fetcher.get_current_scene_data()
	if scene != null and scene.scene_entity_definition != null:
		title = scene.scene_entity_definition.get_title()
	var place: Dictionary = await _async_place(genesis, parcel)
	var place_title = place.get("title", "")
	if place_title is String and not place_title.is_empty():
		title = place_title
	var creator = place.get("contact_name", "")
	return (
		RnReqUtil
		. ok(
			{
				"title": title,
				"creator": creator if creator is String else "",
				"parcel": {"x": parcel.x, "y": parcel.y},
				"realm": Global.realm.realm_url,
				"genesis": genesis,
			}
		)
	)


## The places API record for the current parcel (or world); empty when the lookup fails.
func _async_place(genesis: bool, parcel: Vector2i) -> Dictionary:
	var result: Variant
	if genesis:
		result = await PlacesHelper.async_get_by_position(parcel)
	else:
		result = await PlacesHelper.async_get_by_names(Global.realm.realm_name)
	if result is PromiseError:
		return {}
	var body: Variant = result.get_string_response_as_json()
	var data: Variant = body.get("data", []) if body is Dictionary else []
	if data is Array and not data.is_empty() and data[0] is Dictionary:
		return data[0]
	return {}
