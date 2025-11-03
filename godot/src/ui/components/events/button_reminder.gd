extends Button

const EVENTS_API_BASE_URL = "https://events.decentraland.org/api"

var event_id_value: String
var event_tags: String

@onready var texture_progress_bar: TextureProgressBar = $TextureProgressBar
@onready var texture_rect_add: TextureRect = %TextureRect_Add
@onready var texture_rect_remove: TextureRect = %TextureRect_Remove
@onready var label: Label = %Label
@onready var h_box_container_content: HBoxContainer = %HBoxContainer_Content


func _ready() -> void:
	pass


func _async_on_toggled(toggled_on: bool) -> void:
	if event_id_value == null:
		printerr("NO ID")
		set_pressed_no_signal(!toggled_on)
		return

	_set_loading(true)

	var url = EVENTS_API_BASE_URL + "/events/" + event_id_value + "/attendees"
	var method: HTTPClient.Method

	if toggled_on:
		method = HTTPClient.METHOD_POST
		Global.metrics.track_click_button(
			"reminder_set",
			"EVENT_DETAILS",
			JSON.stringify({"event_id": event_id_value, "event_tag": event_tags})
		)
	else:
		method = HTTPClient.METHOD_DELETE
		Global.metrics.track_click_button(
			"reminder_remove",
			"EVENT_DETAILS",
			JSON.stringify({"event_id": event_id_value, "event_tag": event_tags})
		)

	var response = await Global.async_signed_fetch(url, method)
	if response is PromiseError:
		printerr("Error unpdating attend intention: ", response.get_error())
		set_pressed_no_signal(!toggled_on)
	elif response != null:
		update_styles(toggled_on)
	else:
		set_pressed_no_signal(!toggled_on)
		printerr("Error unpdating attend intention")

	_set_loading(false)


func _set_loading(status: bool) -> void:
	if status:
		texture_progress_bar.show()
		self_modulate = "FFFFFF00"
		h_box_container_content.modulate = "FFFFFF00"
		disabled = true
	else:
		disabled = false
		texture_progress_bar.hide()
		self_modulate = "FFFFFF"
		h_box_container_content.modulate = "FFFFFF"


func update_styles(toggled_on):
	var guest_profile := Global.player_identity.is_guest
	if guest_profile:
		disabled = true
		label.text = "SIGN IN TO USE REMINDERS"
		label.label_settings.font_color = "#ffffff"
		label.label_settings.font_size = 18
		texture_rect_add.hide()
		texture_rect_remove.hide()
	else:
		disabled = false
		label.label_settings.font_size = 22
		if toggled_on:
			label.text = "REMINDER"
			label.label_settings.font_color = "#161518"
			texture_rect_add.hide()
			texture_rect_remove.show()
		else:
			label.text = "REMINDER"
			label.label_settings.font_color = "#ff2d55"
			texture_rect_add.show()
			texture_rect_remove.hide()
