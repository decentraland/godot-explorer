extends Button

const BELL_OUTLINE = preload("res://assets/ui/bell-outline.svg")
const UNSUSCRIBE = preload("res://assets/ui/unsuscribe.svg")
const EVENTS_API_BASE_URL = "https://events.decentraland.org/api"

var event_id_value:String
@onready var texture_progress_bar: TextureProgressBar = $TextureProgressBar

func _ready() -> void:
	pass



func _async_on_toggled(toggled_on: bool) -> void:
	if event_id_value == null:
		set_pressed_no_signal(!toggled_on)
		return
	
	_set_loading(true)

	var url = EVENTS_API_BASE_URL + "/events/" + event_id_value + "/likes"
	var body:String = JSON.stringify({event_id = event_id_value})
	var method: HTTPClient.Method
	
	if toggled_on:
		method = HTTPClient.METHOD_POST
	else:
		method = HTTPClient.METHOD_DELETE

	var response = await Global.async_signed_fetch(url, method, body)
	
	if response is PromiseError:
		printerr("Error unpdating attend intention: ", response.get_error())
		set_pressed_no_signal(!toggled_on)
	elif response != null:
		if toggled_on:
			icon = UNSUSCRIBE
			text = "REMOVE REMINDER"
		else:
			icon = BELL_OUTLINE
			text = "REMINDER"
	else:
		set_pressed_no_signal(!toggled_on)
		printerr("Error unpdating attend intention")

	_set_loading(false)
	
func _set_loading(status:bool) -> void:
	if status:
		texture_progress_bar.show()
		self_modulate = "FFFFFF00"
	else:
		texture_progress_bar.hide()
		self_modulate = "FFFFFF"
