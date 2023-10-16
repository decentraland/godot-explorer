extends Node
class_name Realm

var realm_about = null
var realm_url: String = ""
var realm_string: String = ""

# Mirror realm_about.get("configuration")
var realm_name: String = ""
var realm_scene_urns: Array[Dictionary] = []
var realm_global_scene_urns: Array[Dictionary] = []
var realm_city_loader_content_base_url = ""

var content_base_url: String = ""

var http_requester: RustHttpRequesterWrapper = RustHttpRequesterWrapper.new()
const ABOUT_REQUEST = 1

signal realm_changed


func _process(_delta):
	http_requester.poll()


func _ready():
	http_requester.request_completed.connect(self._on_request_completed)


func _on_request_completed(response: RequestResponse):
	var status_code = response.status_code()
	if response.is_error() or status_code < 200 or status_code > 299:
		return null

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
		var parsed_urn = parse_urn(urn)
		if parsed_urn != null:
			realm_scene_urns.push_back(parsed_urn)

	realm_global_scene_urns.clear()
	for urn in configuration.get("globalScenesUrn", []):
		var parsed_urn = parse_urn(urn)
		if parsed_urn != null:
			realm_global_scene_urns.push_back(parsed_urn)

	realm_city_loader_content_base_url = configuration.get("cityLoaderContentServer", "")

	realm_name = configuration.get("realmName", "no_realm_name")

	content_base_url = ensure_ends_with_slash(realm_about.get("content", {}).get("publicUrl"))

	Global.config.last_realm_joined = realm_url
	Global.config.save_to_settings_file()

	emit_signal("realm_changed")


func is_dcl_ens(str_param: String) -> bool:
	var regex = RegEx.new()
	regex.compile("^[a-zA-Z0-9]+\\.dcl\\.eth$")
	return regex.search(str_param) != null


func dcl_world_url(dcl_name: String) -> String:
	return (
		"https://worlds-content-server.decentraland.org/world/" + dcl_name.to_lower().uri_encode()
	)


func ensure_ends_with_slash(str_param: String) -> String:
	return str_param.trim_suffix("/") + "/"


func resolve_realm_url(value: String) -> String:
	if is_dcl_ens(value):
		return dcl_world_url(value)
	return value


func get_params(url: String) -> Dictionary:
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


func parse_urn(urn: String):
	var regex = RegEx.new()
	regex.compile("^(urn\\:decentraland\\:entity\\:(ba[a-zA-Z0-9]{57}))")
	var matches = regex.search(urn)

	if matches == null:
		return null

	var base_url = get_params(urn).get("baseUrl", [""])[0]

	return {"urn": matches.get_string(0), "entityId": matches.get_string(2), "baseUrl": base_url}


func set_realm(new_realm_string: String) -> void:
	realm_string = new_realm_string
	realm_url = ensure_ends_with_slash(resolve_realm_url(realm_string))
	http_requester._requester.request_json(
		ABOUT_REQUEST, realm_url + "about", HTTPClient.METHOD_GET, "", []
	)
