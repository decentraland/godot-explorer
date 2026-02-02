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
var place_data

@onready var button_like: Button = %Button_Like
@onready var button_dislike: Button = %Button_Dislike
@onready var button_share: Button = %Button_Share


func _ready() -> void:
	if button_share:
		button_share.visible = show_share_button


func update_data(id = null) -> void:
	place_id = id
	async_update_visibility()


func async_update_visibility() -> void:
	if place_id != null:
		await _async_update_buttons_icons()
		show()
	else:
		hide()


func _on_button_share_pressed() -> void:
	if not place_data or not place_data.has("id"):
		printerr("No place data available to share")
		return
	var place_url = DclUrls.host()
	var world = place_data.get("world", false)
	var world_name = place_data.get("world_name", "")
	if world:
		place_url += "/places/world/?name="+ world_name 
	else:
		var base_position = place_data.get("base_position", "0,0")
		place_url += "/places/place/?position=" + base_position


	var place_title = place_data.get("title", "Decentraland Place")

	var text = "Visit " + place_title + "' following this link " + place_url

	if Global.is_android():
		DclAndroidPlugin.share_text(text)
	elif Global.is_ios():
		DclIosPlugin.share_text(text)


func _async_on_button_like_toggled(toggled_on: bool) -> void:
	if place_id == null:
		button_like.set_pressed_no_signal(!toggled_on)
		return

	disable_buttons()

	var response = await PlacesHelper.async_patch_like(
		place_id, PlacesHelper.LIKE.YES if toggled_on else PlacesHelper.LIKE.UNKNOWN
	)

	if response is PromiseError:
		printerr("Error patching likes: ", response.get_error())
	if response != null:
		await _async_update_buttons_icons()
	else:
		button_like.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")

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
		printerr("Error patching likes: ", response.get_error())
	if response != null:
		await _async_update_buttons_icons()
	else:
		if button_dislike:
			button_dislike.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")

	enable_buttons()


func _async_update_buttons_icons() -> void:
	disable_buttons()

	var response = await PlacesHelper.async_get_by_id(place_id)

	if response == null:
		printerr("Error getting place's data")
		enable_buttons()
		return
	if response is PromiseError:
		printerr("Error getting place's data: ", response.get_error())
		enable_buttons()
		return

	var json: Dictionary = response.get_string_response_as_json()
	place_data = json.data

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

	enable_buttons()


func disable_buttons() -> void:
	if button_like:
		button_like.disabled = true
		button_like.get_node("TextureProgressBar").show()
	if button_dislike:
		button_dislike.disabled = true
		button_dislike.get_node("TextureProgressBar").show()


func enable_buttons() -> void:
	if button_like:
		button_like.disabled = false
		button_like.get_node("TextureProgressBar").hide()
	if button_dislike:
		button_dislike.disabled = false
		button_dislike.get_node("TextureProgressBar").hide()
