extends HBoxContainer

const DISLIKE = preload("res://assets/ui/dislike.svg")
const DISLIKE_SOLID = preload("res://assets/ui/dislike_solid.svg")
const LIKE = preload("res://assets/ui/like.svg")
const LIKE_SOLID = preload("res://assets/ui/like_solid.svg")
const PLACES_API_BASE_URL = "https://places.decentraland.org/api"

var place_id

@onready var button_like: Button = %Button_Like
@onready var button_dislike: Button = %Button_Dislike
@onready var button_fav: Button = %Button_Fav
@onready var button_share: Button = %Button_Share


func update_data(id = null) -> void:
	place_id = id
	update_visibility()

func update_visibility() -> void:
	if place_id != null:
		await _update_buttons_icons()
		show()
	else:
		hide()

func _on_button_share_pressed() -> void:
	pass # Replace with function body.


func _on_button_like_toggled(toggled_on: bool) -> void:	
	if place_id == null:
		button_like.set_pressed_no_signal(!toggled_on)
		return
		
	var url = PLACES_API_BASE_URL + "/places/" + place_id + "/likes"
	var body: String
	
	if toggled_on:
		# Activar like
		body = JSON.stringify({ like = true })
		# Desactivar dislike visualmente si estaba activado
		if button_dislike.is_pressed():
			button_dislike.set_pressed_no_signal(false)
	else:
		# Desactivar like (volver a neutral)
		body = JSON.stringify({ like = null })
	
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)
	if response != null:
		await _update_buttons_icons()
	else:
		# Revertir el estado del botón si falló el PATCH
		button_like.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")


func _on_button_dislike_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_dislike.set_pressed_no_signal(!toggled_on)
		return
		
	var url = PLACES_API_BASE_URL + "/places/" + place_id + "/likes"
	var body
	
	if toggled_on:
		# Activar dislike
		body = JSON.stringify({ like = false })
		# Desactivar like visualmente si estaba activado
		if button_like.is_pressed():
			button_like.set_pressed_no_signal(false)
	else:
		# Desactivar dislike (volver a neutral)
		body = JSON.stringify({ like = null })
	
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)
	if response != null:
		await _update_buttons_icons()
	else:
		if button_dislike:
			button_dislike.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")


func _on_button_fav_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_fav.set_pressed_no_signal(!toggled_on)
		return
		
	var url = PLACES_API_BASE_URL + "/places/" + place_id + "/favorites"
	var body = JSON.stringify({"favorites":toggled_on})
	
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)
	if response != null:
		await _update_buttons_icons()
	else:
		if button_fav:
			button_fav.set_pressed_no_signal(!toggled_on)
		printerr("Error patching favorites")


func _update_buttons_icons() -> void:
	var url = PLACES_API_BASE_URL + "/places/" + place_id
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)
	
	if response == null:
		printerr("Error getting place's data")
		return
		
	var place_data = response.data
	
	button_like.set_pressed_no_signal(place_data.user_like)
	if button_like.is_pressed():
		button_like.icon = LIKE_SOLID
	else:
		button_like.icon = LIKE
	
	button_dislike.set_pressed_no_signal(place_data.user_dislike)
	if button_dislike.is_pressed():
		button_dislike.icon = DISLIKE_SOLID
	else:
		button_dislike.icon = DISLIKE

	button_fav.set_pressed_no_signal(place_data.user_favorite)
