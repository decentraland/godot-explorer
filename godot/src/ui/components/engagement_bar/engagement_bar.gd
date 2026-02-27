extends HBoxContainer

const DISLIKE = preload("res://assets/ui/dislike.svg")
const DISLIKE_SOLID = preload("res://assets/ui/dislike_solid.svg")
const LIKE = preload("res://assets/ui/like.svg")
const LIKE_SOLID = preload("res://assets/ui/like_solid.svg")

var place_id
var _debounced: DebouncedAction

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
	disable_buttons()
	await _async_update_status()


func _get_debounced() -> DebouncedAction:
	if _debounced == null:
		_debounced = DebouncedAction.new(_async_patch_like, PlacesHelper.LIKE.UNKNOWN)
		add_child(_debounced)
	return _debounced


func _async_patch_like(state: PlacesHelper.LIKE) -> void:
	await PlacesHelper.async_patch_like(place_id, state)


func _on_button_like_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_like.set_pressed_no_signal(!toggled_on)
		return

	button_like.icon = LIKE_SOLID if toggled_on else LIKE
	if toggled_on:
		button_dislike.set_pressed_no_signal(false)
		button_dislike.icon = DISLIKE
		(
			Global
			. metrics
			. track_click_button(
				"THUMBS_UP",
				"PLACE_DETAIL_CLICK",
				JSON.stringify({"place_id": place_id}),
			)
		)
	_get_debounced().schedule(PlacesHelper.LIKE.YES if toggled_on else PlacesHelper.LIKE.UNKNOWN)


func _on_button_dislike_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_dislike.set_pressed_no_signal(!toggled_on)
		return

	button_dislike.icon = DISLIKE_SOLID if toggled_on else DISLIKE
	if toggled_on:
		button_like.set_pressed_no_signal(false)
		button_like.icon = LIKE
		(
			Global
			. metrics
			. track_click_button(
				"THUMBS_DOWN",
				"PLACE_DETAIL_CLICK",
				JSON.stringify({"place_id": place_id}),
			)
		)
	_get_debounced().schedule(PlacesHelper.LIKE.NO if toggled_on else PlacesHelper.LIKE.UNKNOWN)


func _apply_button_state(data: Dictionary) -> void:
	var user_like := data.get("user_like", false) as bool
	var user_dislike := data.get("user_dislike", false) as bool

	button_like.set_pressed_no_signal(user_like)
	button_like.icon = LIKE_SOLID if user_like else LIKE

	button_dislike.set_pressed_no_signal(user_dislike)
	button_dislike.icon = DISLIKE_SOLID if user_dislike else DISLIKE

	var initial_state: PlacesHelper.LIKE
	if user_like:
		initial_state = PlacesHelper.LIKE.YES
	elif user_dislike:
		initial_state = PlacesHelper.LIKE.NO
	else:
		initial_state = PlacesHelper.LIKE.UNKNOWN
	_get_debounced().set_state_no_send(initial_state)


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
