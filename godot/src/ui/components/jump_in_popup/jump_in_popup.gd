extends ColorRect

var texture_placeholder = load("res://assets/ui/placeholder2.png")

var location: Vector2i = Vector2i(0, 0)
var realm: String = Realm.MAIN_REALM

@onready var texture_rect: TextureRect = %TextureRect
@onready var label_title: Label = %Label_Title
@onready var label_creator: Label = %Label_Creator


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	hide()


func open(pos: Vector2i) -> void:
	location = pos
	async_load_place_position()


func _on_button_jump_in_cancel_pressed() -> void:
	close()


func _on_button_jump_in_pressed() -> void:
	close()
	Global.teleport_to(location, realm)


func async_load_place_position():
	var url: String = DclUrls.places_api() + "/places?limit=1"
	url += "&positions=%d,%d" % [location.x, location.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request places jump in", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		label_creator.show()
		label_creator.text = "Created by Unknown"
		label_title.text = "Empty parcel"
		texture_rect.texture = texture_placeholder
	else:
		var creator = json.data[0].get("contact_name", "Unknown")
		if creator != "Unknown" and creator:
			label_creator.show()
			label_creator.text = "Created by " + creator
		else:
			label_creator.hide()
		var title = json.data[0].get("title", "interactive-text")
		if title != "interactive-text":
			label_title.text = title
		else:
			label_title.text = "Unknown Place"
		var image_url = json.data[0].get("image", "")
		_async_download_image(image_url)
	show()


func _async_download_image(url: String):
	var url_hash = get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		texture_rect.texture = texture_placeholder
		printerr("places_generator::_async_download_image promise error: ", result.get_error())
		return
	texture_rect.texture = result.texture


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
