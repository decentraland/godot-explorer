extends Node
class_name Realm

var requester: HTTPManyRequester = null
	
var realm_desired_running_scenes: Array[Dictionary] = []
var realm_about = null
var realm_url: String = ""
var realm_string: String = ""

var content_base_url: String = ""

signal realm_changed()

func _ready():
	
	requester = HTTPManyRequester.new()
	add_child(requester)

func is_dcl_ens(str_param: String) -> bool:
	var regex = RegEx.new()
	regex.compile("^[a-zA-Z0-9]+\\.dcl\\.eth$")
	return regex.search(str_param) != null
	
func dcl_world_url(dcl_name: String) -> String:
	return "https://worlds-content-server.decentraland.org/world/" + dcl_name.to_lower().uri_encode()

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
	
	return {
		"urn": matches.get_string(0),
		"entityId": matches.get_string(2),
		"baseUrl": base_url
	}

func get_active_entities(pointers: Array) -> Variant:
	
	if realm_about == null:
		await self.realm_changed
	
	var body_json = JSON.stringify({"pointers": pointers})
	var entities_response = await requester.do_request_json(content_base_url + "entities/active", HTTPClient.METHOD_POST, body_json, ["Content-type: application/json"])
	if entities_response == null:
		printerr("Failed getting active entities " + self.realm_string)
		return
		
	return entities_response

func set_realm(new_realm_string: String) -> void:
	realm_string = new_realm_string
	realm_url = ensure_ends_with_slash(resolve_realm_url(realm_string))
	var about_response = await requester.do_request_json(realm_url + "about", HTTPClient.METHOD_GET)
	if about_response == null or not about_response is Dictionary:
		printerr("Failed setting new realm " + realm_string)
		return
		
	realm_about = about_response
	
	realm_desired_running_scenes.clear()
	for urn in realm_about.get("configurations", {}).get("scenesUrn", []):
		var parsed_urn = parse_urn(urn)
		if parsed_urn != null:
			realm_desired_running_scenes.push_back(parsed_urn)

	content_base_url = ensure_ends_with_slash(realm_about.get("content", {}).get("publicUrl"))
	
	emit_signal("realm_changed")

