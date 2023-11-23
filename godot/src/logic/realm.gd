extends DclRealm
class_name Realm

var http_requester: RustHttpRequesterWrapper = Global.http_requester

signal realm_changed


static func is_dcl_ens(str_param: String) -> bool:
	var regex = RegEx.new()
	regex.compile("^[a-zA-Z0-9]+\\.dcl\\.eth$")
	return regex.search(str_param) != null


static func dcl_world_url(dcl_name: String) -> String:
	return (
		"https://worlds-content-server.decentraland.org/world/" + dcl_name.to_lower().uri_encode()
	)


static func ensure_ends_with_slash(str_param: String) -> String:
	return str_param.trim_suffix("/") + "/"


static func ensure_starts_with_https(str_param: String) -> String:
	if str_param.begins_with("https://"):
		return str_param

	if str_param.begins_with("http://"):
		return str_param

	return "https://" + str_param


static func resolve_realm_url(value: String) -> String:
	if Realm.is_dcl_ens(value):
		return Realm.dcl_world_url(value)
	return value


static func get_params(url: String) -> Dictionary:
	var ret: Dictionary = {}
	var parts = url.split("?")
	if parts.size() > 1:
		var params = parts[1].split("&")
		for param in params:
			var key_value = param.split("=")
			var current_values = ret.get(key_value[0], [])
			if key_value.size() > 1:
				current_values.push_back(key_value[1])
			else:
				current_values.push_back(true)
			ret[key_value[0]] = current_values
	return ret


static func parse_urn(urn: String):
	var regex = RegEx.new()
	regex.compile("^(urn\\:decentraland\\:entity\\:(ba[a-zA-Z0-9]{57}))")
	var matches = regex.search(urn)

	if matches == null:
		return null

	var base_url = get_params(urn).get("baseUrl", [""])[0]

	return {"urn": matches.get_string(0), "entityId": matches.get_string(2), "baseUrl": base_url}


func set_realm(new_realm_string: String) -> void:
	realm_string = new_realm_string
	realm_url = Realm.ensure_ends_with_slash(Realm.resolve_realm_url(realm_string))
	realm_url = Realm.ensure_starts_with_https(realm_url)
	var promise: Promise = http_requester.request_json(
		realm_url + "about", HTTPClient.METHOD_GET, "", []
	)

	var res = await promise.co_awaiter()
	if res is Promise.Error:
		printerr(
			"Rejected request change realm to: ",
			new_realm_string,
			" error message: ",
			res.get_error()
		)
	elif res is RequestResponse:
		var response: RequestResponse = res

		var json = response.get_string_response_as_json()
		if json == null:
			printerr("do_request_json failed because json_string is not a valid json")
			return

		var about_response = json
		if about_response == null or not about_response is Dictionary:
			printerr("Failed setting new realm " + realm_string)
			return

		realm_about = about_response

		var configuration = realm_about.get("configurations", {})

		realm_scene_urns.clear()
		for urn in configuration.get("scenesUrn", []):
			var parsed_urn = Realm.parse_urn(urn)
			if parsed_urn != null:
				realm_scene_urns.push_back(parsed_urn)

		realm_global_scene_urns.clear()
		for urn in configuration.get("globalScenesUrn", []):
			var parsed_urn = Realm.parse_urn(urn)
			if parsed_urn != null:
				realm_global_scene_urns.push_back(parsed_urn)

		realm_city_loader_content_base_url = configuration.get("cityLoaderContentServer", "")
		if not realm_city_loader_content_base_url.is_empty():
			realm_city_loader_content_base_url = Realm.ensure_ends_with_slash(
				configuration.get("cityLoaderContentServer", "")
			)

		realm_name = configuration.get("realmName", "no_realm_name")

		content_base_url = Realm.ensure_ends_with_slash(
			realm_about.get("content", {}).get("publicUrl")
		)

		Global.config.last_realm_joined = realm_url
		Global.config.save_to_settings_file()

		emit_signal("realm_changed")
