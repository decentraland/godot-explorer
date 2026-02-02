extends HBoxContainer

const DISLIKE = preload("res://assets/ui/dislike.svg")
const DISLIKE_SOLID = preload("res://assets/ui/dislike_solid.svg")
const LIKE = preload("res://assets/ui/like.svg")
const LIKE_SOLID = preload("res://assets/ui/like_solid.svg")

@export var show_share_button: bool = false:
	set(value):
		show_share_button = value
		if button_share:
			button_share.visible = value

var place_id

@onready var button_like: Button = %Button_Like
@onready var button_dislike: Button = %Button_Dislike
@onready var button_fav: Button = %Button_Fav
@onready var button_share: Button = %Button_Share


func _ready() -> void:
	if button_share:
		button_share.visible = show_share_button


func update_data(data: Dictionary = {}) -> void:
	place_id = data.get("id", null)
	if place_id != null:
		_apply_button_state(data)
		show()
	else:
		hide()


func _on_button_share_pressed() -> void:
	pass  # Replace with function body.


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


func _async_on_button_fav_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_fav.set_pressed_no_signal(!toggled_on)
		return

	disable_buttons()

	var response = await PlacesHelper.async_patch_favorite(place_id, toggled_on)

	if response is PromiseError:
		button_fav.set_pressed_no_signal(!toggled_on)
		printerr("Error patching favorites: ", response.get_error())
	elif response == null:
		button_fav.set_pressed_no_signal(!toggled_on)
		printerr("Error patching favorites")

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

	button_fav.set_pressed_no_signal(data.get("user_favorite", false))


func disable_buttons() -> void:
	if button_like:
		button_like.disabled = true
		button_like.get_node("TextureProgressBar").show()
	if button_dislike:
		button_dislike.disabled = true
		button_dislike.get_node("TextureProgressBar").show()
	if button_fav:
		button_fav.disabled = true
		button_fav.get_node("TextureProgressBar").show()


func enable_buttons() -> void:
	if button_like:
		button_like.disabled = false
		button_like.get_node("TextureProgressBar").hide()
	if button_dislike:
		button_dislike.disabled = false
		button_dislike.get_node("TextureProgressBar").hide()
	if button_fav:
		button_fav.disabled = false
		button_fav.get_node("TextureProgressBar").hide()
