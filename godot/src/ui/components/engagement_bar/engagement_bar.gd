extends HBoxContainer

const DISLIKE = preload("res://assets/ui/dislike.svg")
const DISLIKE_SOLID = preload("res://assets/ui/dislike_solid.svg")
const LIKE = preload("res://assets/ui/like.svg")
const LIKE_SOLID = preload("res://assets/ui/like_solid.svg")


var place_id

@onready var button_like: Button = %Button_Like
@onready var button_dislike: Button = %Button_Dislike
@onready var button_like_pressed: Button = %Button_Like_Pressed
@onready var button_dislike_pressed: Button = %Button_Dislike_Pressed



func update_data(id = null) -> void:
	place_id = id
	if place_id != null and place_id != "-":
		async_update_state()
	else:
		hide()


func async_update_state() -> void:
	await _async_update_status()
	show()


func _async_on_button_like_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_like.set_pressed_no_signal(!toggled_on)
		return

	disable_buttons()

	var response = await PlacesHelper.async_patch_like(
		place_id, PlacesHelper.LIKE.YES if toggled_on else PlacesHelper.LIKE.UNKNOWN
	)

	if response is PromiseError:
		button_like.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes: ", response.get_error())
	elif response == null:
		button_like.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")
	else:
		button_like.icon = LIKE_SOLID if toggled_on else LIKE
		if toggled_on:
			button_dislike.set_pressed_no_signal(false)
			button_dislike.icon = DISLIKE

	enable_buttons()


func _async_on_button_dislike_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_dislike.set_pressed_no_signal(!toggled_on)
		return

	disable_buttons()

	var response = await PlacesHelper.async_patch_like(
		place_id, PlacesHelper.LIKE.NO if toggled_on else PlacesHelper.LIKE.UNKNOWN
	)

	if response is PromiseError:
		button_dislike.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes: ", response.get_error())
	elif response == null:
		button_dislike.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")
	else:
		button_dislike.icon = DISLIKE_SOLID if toggled_on else DISLIKE
		if toggled_on:
			button_like.set_pressed_no_signal(false)
			button_like.icon = LIKE

	enable_buttons()


func _apply_button_state(data: Dictionary) -> void:
	button_like.set_pressed_no_signal(data.get("user_like", false))
	if button_like.is_pressed():
		button_like.icon = LIKE_SOLID
	else:
		button_like.icon = LIKE

	button_dislike.set_pressed_no_signal(data.get("user_dislike", false))
	if button_dislike.is_pressed():
		button_dislike.icon = DISLIKE_SOLID
	else:
		button_dislike.icon = DISLIKE


func _async_update_status() -> void:
	if place_id == null:
		enable_buttons()
		return

	disable_buttons()

	var url = DclUrls.places_api() + "/places/" + str(place_id)
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)

	if response == null:
		enable_buttons()
		return
	if response is PromiseError:
		printerr("Error getting place data for likes: ", response.get_error())
		enable_buttons()
		return

	var json: Dictionary = response.get_string_response_as_json()
	var place_data = json.get("data", json)
	_apply_button_state(place_data)
	enable_buttons()


func disable_buttons() -> void:
	if button_like:
		button_like.disabled = true
		button_like.self_modulate = Color.TRANSPARENT
		button_like_pressed.show()
	if button_dislike:
		button_dislike.disabled = true
		button_dislike.self_modulate = Color.TRANSPARENT
		button_dislike_pressed.show()


func enable_buttons() -> void:
	if button_like:
		button_like.disabled = false
		button_like.self_modulate = Color.WHITE
		button_like_pressed.hide()
	if button_dislike:
		button_dislike.disabled = false
		button_dislike.self_modulate = Color.WHITE
		button_dislike_pressed.hide()
