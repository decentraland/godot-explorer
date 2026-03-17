class_name FavButton
extends TextureButton

var _place_id
var _is_world: bool = false
var _debounced: DebouncedAction

@onready var texture_rect_pressed: TextureRect = %TextureRect_Pressed


func update_data(id = null, is_world: bool = false) -> void:
	_place_id = id
	_is_world = is_world
	async_update_visibility()


func async_update_visibility() -> void:
	if _place_id != null and _place_id != "-":
		show()
		disabled = true
		await _async_update_status()
	else:
		hide()


func _async_set_fav(toggled_on) -> void:
	if _place_id == null:
		set_pressed_no_signal(!toggled_on)
		return

	set_pressed_no_signal(toggled_on)
	var button_text := "FAVORITE" if toggled_on else "UNFAVORITE"
	(
		Global
		. metrics
		. track_click_button(
			button_text,
			"PLACE_DETAIL_CLICK",
			JSON.stringify({"place_id": _place_id}),
		)
	)
	_get_debounced().schedule(toggled_on)


func _get_debounced() -> DebouncedAction:
	if _debounced == null:
		_debounced = DebouncedAction.new(_async_patch_fav, false)
		add_child(_debounced)
	return _debounced


func _async_patch_fav(toggled_on: bool) -> void:
	await PlacesHelper.async_patch_favorite(_place_id, toggled_on, _is_world)


func _async_update_status() -> void:
	var url: String
	if _is_world:
		url = DclUrls.places_api() + "/worlds?names=" + _place_id.uri_encode()
	else:
		url = DclUrls.places_api() + "/places/" + _place_id
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)

	if response == null:
		printerr("Error getting place's data")
		disabled = false
		return
	if response is PromiseError:
		printerr("Error getting place's data: ", response.get_error())
		disabled = false
		return

	var json: Dictionary = response.get_string_response_as_json()
	var place_data: Dictionary
	if _is_world:
		var data_array: Array = json.get("data", [])
		if data_array.is_empty():
			disabled = false
			return
		place_data = data_array[0]
	else:
		place_data = json.data

	var is_fav: bool = place_data.get("user_favorite", false)
	set_pressed_no_signal(is_fav)
	_get_debounced().set_state_no_send(is_fav)
	disabled = false
