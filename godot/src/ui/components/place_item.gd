class_name PlaceItem
extends Control

signal item_pressed(data)
signal jump_in(position: Vector2i, realm: String)
signal close

const DISLIKE = preload("res://assets/ui/dislike.svg")
const DISLIKE_SOLID = preload("res://assets/ui/dislike_solid.svg")
const LIKE = preload("res://assets/ui/like.svg")
const LIKE_SOLID = preload("res://assets/ui/like_solid.svg")
const PLACES_API_BASE_URL = "https://places.decentraland.org/api"

@export var texture: Texture2D = texture_placeholder
@export var title: String = "Scene Title"
@export var description: String = "Scene Description"
@export var views: int = 0
@export var onlines: int = 0
@export var likes_percent: float = 0.0
@export var metadata: Dictionary = {}
@export var location: Vector2i = Vector2i(0, 0)
@export var realm: String = Realm.MAIN_REALM
@export var realm_title: String = "Genesis City"

var texture_placeholder = load("res://assets/ui/placeholder.png")
var _data = null
var _id = null
var _node_cache: Dictionary = {}


func _ready():
	UiSounds.install_audio_recusirve(self)
	_connect_signals()

	if metadata.is_empty():
		set_image(texture)
		set_views(views)
		set_online(onlines)
		set_title(title)
		set_description(description)
		set_likes_percent(likes_percent)
		set_location(location)
	else:
		set_data(metadata)


func _get_node_safe(node_name: String) -> Node:
	if not _node_cache.has(node_name):
		_node_cache[node_name] = get_node_or_null("%" + node_name)
	return _node_cache[node_name]


func _get_button_close() -> Button:
	return _get_node_safe("Button_Close")


func _get_buttons_container() -> HBoxContainer:
	return _get_node_safe("HBoxContainer_Buttons")
	
	
func _get_button_like() -> Button:
	return _get_node_safe("Button_Like")


func _get_button_dislike() -> Button:
	return _get_node_safe("Button_Dislike")


func _get_button_fav() -> Button:
	return _get_node_safe("Button_Fav")


func _get_button_share() -> Button:
	return _get_node_safe("Button_Share")


func _get_button_jump_in() -> Button:
	return _get_node_safe("Button_JumpIn")


func _get_label_location() -> Label:
	return _get_node_safe("Label_Location")


func _get_label_realm() -> Label:
	return _get_node_safe("Label_Realm")


func _get_label_creator() -> Label:
	return _get_node_safe("Label_Creator")


func _get_container_creator() -> Control:
	return _get_node_safe("HBoxContainer_Creator")


func _get_label_title() -> Label:
	return _get_node_safe("Label_Title")


func _get_label_description() -> Label:
	return _get_node_safe("Label_Description")


func _get_label_online() -> Label:
	return _get_node_safe("Label_Online")


func _get_container_online() -> Control:
	return _get_node_safe("Container_Online")


func _get_label_views() -> Label:
	return _get_node_safe("Label_Views")


func _get_container_views() -> Control:
	return _get_node_safe("HBoxContainer_Views")


func _get_label_likes() -> Label:
	return _get_node_safe("Label_Likes")


func _get_container_likes() -> Control:
	return _get_node_safe("HBoxContainer_Likes")


func _get_texture_image() -> TextureRect:
	return _get_node_safe("TextureRect_Image")


func _connect_signals():
	var button_close = _get_button_close()
	if button_close:
		if not button_close.pressed.is_connected(_on_button_close_pressed):
			button_close.pressed.connect(_on_button_close_pressed)

	var button_jump_in = _get_button_jump_in()
	if button_jump_in:
		if not button_jump_in.pressed.is_connected(_on_button_jump_in_pressed):
			button_jump_in.pressed.connect(_on_button_jump_in_pressed)

	var button_like = _get_button_like()
	if button_like:
		if not button_like.pressed.is_connected(_on_button_like_toggled):
			button_like.pressed.connect(_on_button_like_toggled)


func set_location(_location: Vector2i):
	var label = _get_label_location()
	if label:
		location = _location
		label.text = "%s, %s" % [_location.x, _location.y]


func set_image(_texture: Texture2D):
	var texture_rect = _get_texture_image()
	if texture_rect:
		texture_rect.texture = _texture


func set_title(_title: String):
	var label = _get_label_title()
	if label:
		label.text = _title


func set_description(_description: String):
	var label = _get_label_description()
	if label:
		label.text = _description


func set_views(_views: int):
	var label = _get_label_views()
	var container = _get_container_views()
	if label and container:
		container.set_visible(_views > 0)
		label.text = _format_number(_views)


func set_likes_percent(_likes: float):
	var label = _get_label_likes()
	var container = _get_container_likes()
	if label and container:
		container.set_visible(_likes > 0.0)
		label.text = str(round(_likes * 100)) + "%"


func set_online(_online: int):
	var label = _get_label_online()
	var container = _get_container_online()
	if label and container:
		container.set_visible(_online > 0)
		label.text = _format_number(_online)


func set_realm(_realm: String, _realm_title: String):
	realm = _realm
	var label = _get_label_realm()
	if label:
		label.text = _realm_title


func set_creator(_creator: String):
	var label = _get_label_creator()
	var container = _get_container_creator()
	if label and container:
		container.set_visible(not _creator.is_empty())
		label.text = _creator


func set_data(item_data):
	_data = item_data
	_update_buttons_icons()

	set_title(item_data.get("title", ""))
	set_description(_get_or_empty_string(item_data, "description"))

	set_views(item_data.get("user_visits", 0))
	var like_score = item_data.get("like_score", 0.0)
	set_likes_percent(like_score if like_score is float else 0.0)
	set_online(item_data.get("user_count", 0))
	
	if _get_texture_image():
		var image_url = item_data.get("image", "")
		if not image_url.is_empty():
			_async_download_image(image_url)
		else:
			set_image(texture_placeholder)

	var location_vector = item_data.get("base_position", "0,0").split(",")
	if location_vector.size() == 2:
		set_location(Vector2i(int(location_vector[0]), int(location_vector[1])))

	set_creator(_get_or_empty_string(item_data, "contact_name"))

	var world = item_data.get("world", false)
	if world:
		var world_name = item_data.get("world_name")
		set_realm(world_name, world_name)
	else:
		set_realm(Realm.MAIN_REALM, "Genesis City")


func _async_download_image(url: String):
	var url_hash = get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		set_image(texture_placeholder)
		printerr("places_generator::_async_download_image promise error: ", result.get_error())
		return
	set_image(result.texture)


func _on_button_jump_in_pressed():
	jump_in.emit(location, realm)


func _on_button_close_pressed() -> void:
	close.emit()


func _on_pressed():
	item_pressed.emit(_data)


func _format_number(num: int) -> String:
	if num < 1e3:
		return str(num)
	if num < 1e6:
		return str(ceil(num / 1000.0)) + "k"
	return str(floor(num / 1000000.0)) + "M"


func get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]  # Return the last part

	# Convert URL to a hexadecimal
	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		# Convert the URL string to UTF-8 bytes and update the context with this data
		context.update(url.to_utf8_buffer())
		# Finalize the hashing process and get the hash as a PackedByteArray
		var url_hash: PackedByteArray = context.finish()
		# Encode the hash as hexadecimal
		return url_hash.hex_encode()

	return "temp-file"


func _get_or_empty_string(dict: Dictionary, key: String) -> String:
	var value = dict.get(key, null)
	if value is String:
		return value
	return ""


func _on_button_share_pressed() -> void:
	pass # Replace with function body.


func _on_button_like_toggled(toggled_on: bool) -> void:
	var place_id = _data.get("id", null)
	var button_like = _get_button_like()
	var button_dislike = _get_button_dislike()
	
	if place_id == null:
		if button_like:
			button_like.set_pressed_no_signal(!toggled_on)
		return
		
	var url = PLACES_API_BASE_URL + "/places/" + place_id + "/likes"
	var body: String
	
	if toggled_on:
		# Activar like
		body = JSON.stringify({ like = true })
		# Desactivar dislike visualmente si estaba activado
		if button_dislike and button_dislike.is_pressed():
			button_dislike.set_pressed_no_signal(false)
	else:
		# Desactivar like (volver a neutral)
		body = JSON.stringify({ like = null })
	
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)
	if response != null:
		await _update_buttons_icons()
	else:
		# Revertir el estado del botón si falló el PATCH
		if button_like:
			button_like.set_pressed_no_signal(!toggled_on)
		printerr("Error patching likes")


func _on_button_dislike_toggled(toggled_on: bool) -> void:
	var place_id = _data.get("id", null)
	var button_dislike = _get_button_dislike()
	var button_like = _get_button_like()
	
	if place_id == null:
		if button_dislike:
			button_dislike.set_pressed_no_signal(!toggled_on)
		return
		
	var url = PLACES_API_BASE_URL + "/places/" + place_id + "/likes"
	var body
	
	if toggled_on:
		# Activar dislike
		body = JSON.stringify({ like = false })
		# Desactivar like visualmente si estaba activado
		if button_like and button_like.is_pressed():
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
	var place_id = _data.get("id", null)
	var button_fav = _get_button_fav()
	
	if place_id == null:
		if button_fav:
			button_fav.set_pressed_no_signal(!toggled_on)
		return
		
	var url = PLACES_API_BASE_URL + "/places/" + place_id + "/favorites"
	var body = JSON.stringify({ favorites = toggled_on })
	
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)
	if response != null:
		await _update_buttons_icons()
	else:
		if button_fav:
			button_fav.set_pressed_no_signal(!toggled_on)
		printerr("Error patching favorites")


func _update_buttons_icons() -> void:
	var place_id = _data.get("id", null)
	var buttons_container = _get_buttons_container()
	
	if place_id == null:
		if buttons_container:
			buttons_container.visible = false
		return
		
	if buttons_container:	
		buttons_container.visible = true
	
	var url = PLACES_API_BASE_URL + "/places/" + place_id
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)
	
	if response == null:
		printerr("Error al obtener datos del lugar")
		return
		
	var place_data = response.data
	var button_like = _get_button_like()
	var button_dislike = _get_button_dislike()
	var button_fav = _get_button_fav()
	
	if button_like:
		button_like.set_pressed_no_signal(place_data.user_like)
		if button_like.is_pressed():
			button_like.icon = LIKE_SOLID
		else:
			button_like.icon = LIKE
	
	if button_dislike:
		button_dislike.set_pressed_no_signal(place_data.user_dislike)
		if button_dislike.is_pressed():
			button_dislike.icon = DISLIKE_SOLID
		else:
			button_dislike.icon = DISLIKE
	
	if button_fav:
		button_fav.set_pressed_no_signal(place_data.user_favorite)
