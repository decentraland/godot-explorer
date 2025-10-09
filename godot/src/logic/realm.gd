class_name Realm
extends DclRealm

signal realm_changed

const MAIN_REALM: String = "https://realm-provider-ea.decentraland.org/main"

const WORLDS_URL: String = "https://worlds-content-server.decentraland.org/world/"

const DAO_SERVERS: Array[String] = [
	"https://peer-ec1.decentraland.org/",
	"https://peer-ec2.decentraland.org/",
	"https://peer-wc1.decentraland.org/",
	"https://peer-eu1.decentraland.org/",
	"https://peer-ap1.decentraland.org/",
	"https://interconnected.online/",
	"https://peer.decentral.io/",
	"https://peer.melonwave.com/",
	"https://peer.kyllian.me/",
	"https://peer.uadevops.com/",
	"https://peer.dclnodes.io/",
	"https://realm-provider.decentraland.org/main/",
	"https://realm-provider-ea.decentraland.org/main/"
]

var _has_realm = false


static func is_dcl_ens(str_param: String) -> bool:
	var regex = RegEx.new()
	regex.compile("^[a-zA-Z0-9]+\\.dcl\\.eth$")
	return regex.search(str_param) != null


static func is_genesis_city(_realm_name: String):
	_realm_name = Realm.ensure_ends_with_slash(Realm.resolve_realm_url(_realm_name))
	_realm_name = Realm.ensure_starts_with_https(_realm_name)
	for server in DAO_SERVERS:
		if server.contains(_realm_name):
			return true
	return false


static func dcl_world_url(dcl_name: String) -> String:
	return WORLDS_URL + dcl_name.to_lower().uri_encode()


static func ensure_reduce_url(url):
	return ensure_remove_slash(ensure_dcl_ens(url))


static func ensure_dcl_ens(url: String) -> String:
	return url.replace(WORLDS_URL, "")


static func ensure_remove_slash(str_param: String) -> String:
	if str_param.ends_with("/"):
		return str_param.left(str_param.length() - 1)
	return str_param


static func ensure_ends_with_slash(str_param: String) -> String:
	if str_param.is_empty():
		return ""

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


func has_realm():
	return _has_realm


func async_clear_realm():
	_has_realm = false
	realm_url = ""
	realm_string = ""
	realm_about = {}
	realm_scene_urns.clear()
	realm_global_scene_urns.clear()
	realm_city_loader_content_base_url = ""
	realm_name = ""
	network_id = 0
	content_base_url = ""
	Global.scene_runner.kill_all_scenes()


func async_set_realm(new_realm_string: String, search_new_pos: bool = false) -> void:
	prints("async_set_realm", new_realm_string, search_new_pos)
	realm_string = new_realm_string
	realm_url = Realm.ensure_ends_with_slash(Realm.resolve_realm_url(realm_string))
	realm_url = Realm.ensure_starts_with_https(realm_url)

	var promise: Promise = Global.http_requester.request_json(
		realm_url + "about", HTTPClient.METHOD_GET, "", {}
	)

	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
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

		var new_lambda_server_base_url = realm_about.get("lambdas", {}).get(
			"publicUrl", "https://peer.decentraland.org/lambdas/"
		)
		if not new_lambda_server_base_url.is_empty():
			new_lambda_server_base_url = Realm.ensure_ends_with_slash(new_lambda_server_base_url)

		self.set_lambda_server_base_url(new_lambda_server_base_url)

		realm_name = configuration.get("realmName", "no_realm_name")
		network_id = int(configuration.get("networkId", 1))  # 1=Ethereum

		# get minimap
		var map_config = configuration.get("map", {})
		var sizes = map_config.get("sizes", [])

		# Initialize with extreme values
		var min_bounds: Vector2i = Vector2i(INF, INF)
		var max_bounds: Vector2i = Vector2i(-INF, -INF)

		# Process each size entry
		for size_dict in sizes:
			var left = size_dict.get("left", 0)
			var top = size_dict.get("top", 0)
			var right = size_dict.get("right", 0)
			var bottom = size_dict.get("bottom", 0)

			# Update minimum bounds (leftmost and bottommost points)
			min_bounds.x = mini(min_bounds.x, left)
			min_bounds.y = mini(min_bounds.y, bottom)

			# Update maximum bounds (rightmost and topmost points)
			max_bounds.x = maxi(max_bounds.x, right)
			max_bounds.y = maxi(max_bounds.y, top)

		# Handle empty array case
		if sizes.is_empty():
			min_bounds = Vector2i(-150, -150)
			max_bounds = Vector2i(163, 158)

		set_realm_min_bounds(min_bounds)
		set_realm_max_bounds(max_bounds)

		content_base_url = Realm.ensure_ends_with_slash(
			realm_about.get("content", {}).get("publicUrl")
		)

		if not realm_scene_urns.is_empty() and search_new_pos:
			await async_request_set_position(realm_scene_urns.back())

		Global.get_config().last_realm_joined = realm_url
		Global.get_config().save_to_settings_file()

		Global.metrics.update_realm(realm_url)

		_has_realm = true
		realm_changed.emit()


func async_request_set_position(scene_urn):
	prints(scene_urn)
	var url = scene_urn.baseUrl + scene_urn.entityId

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})

	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr(
			"Rejected request async_request_set_position: ",
			scene_urn,
			" error message: ",
			res.get_error()
		)
	elif res is RequestResponse:
		var response: RequestResponse = res
		var json: Dictionary = response.get_string_response_as_json()
		if json == null:
			printerr("do_request_json failed because json_string is not a valid json")
			return

		var base_pos = json.get("metadata", {}).get("scene", {}).get("base", "0,0")
		var coord = base_pos.split(",")
		var x = int(coord[0])
		var y = int(coord[1])
		var explorer = Global.get_explorer()
		if is_instance_valid(explorer):
			explorer.teleport_to(Vector2i(x, y))
