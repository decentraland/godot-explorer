class_name FavButton
extends TextureButton

var _place_id

@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar


func update_data(id = null) -> void:
	_place_id = id
	async_update_visibility()


func async_update_visibility() -> void:
	if _place_id != null and _place_id != "-":
		await _async_update_status()
		show()
	else:
		hide()


func _async_set_fav(toggled_on) -> void:
	if _place_id == null:
		set_pressed_no_signal(!toggled_on)
		return

	disable_button()

	var url = DclUrls.places_api() + "/places/" + _place_id + "/favorites"
	var body = JSON.stringify({"favorites": toggled_on})

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)
	if response is PromiseError:
		printerr("Error patching favorites: ", response.get_error())
	if response != null:
		set_pressed_no_signal(toggled_on)
	else:
		set_pressed_no_signal(!toggled_on)
		printerr("Error patching favorites")
	enable_button()


func _async_update_status() -> void:
	disable_button()

	var url = DclUrls.places_api() + "/places/" + _place_id
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)

	if response == null:
		printerr("Error getting place's data")
		enable_button()
		return
	if response is PromiseError:
		printerr("Error getting place's data: ", response.get_error())
		return

	var json: Dictionary = response.get_string_response_as_json()
	var place_data = json.data

	set_pressed_no_signal(place_data.user_favorite)

	enable_button()


func disable_button() -> void:
	disabled = true
	self_modulate = Color.TRANSPARENT
	texture_progress_bar.show()


func enable_button() -> void:
	disabled = false
	self_modulate = Color.WHITE
	texture_progress_bar.hide()
