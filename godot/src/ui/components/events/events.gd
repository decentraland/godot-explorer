extends Control

const EVENTS_API_BASE_URL = "https://events.decentraland.org/api"




func _ready() -> void:
	pass
	
	
func get_events() -> Array:
	var response = await Global.async_signed_fetch(EVENTS_API_BASE_URL + "/events", HTTPClient.METHOD_GET, "")
	if response is PromiseError:
		printerr("Error requesting events: ", response.get_error())
		return []
	var json: Dictionary = response.get_string_response_as_json()
	if json.has("data"):
		return json.data
	return []


func _on_button_pressed() -> void:
	var events = await get_events()
	for event in events:
		print("Event Title: ", event.get("name", "Unknown event"))
		print("live: ", event.get("live", "Unknown status"))
		print("Total attendees: ", int(event.get("total_attendees", 0)))
		print("==========================================================================")
